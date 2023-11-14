pub mod discover;
pub mod measure;
pub mod ping;
pub mod push;
pub mod update;

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{config, service};

// TODO: pidgeon health update recurring process

pub trait Process {
  fn new(config: config::Manager, services: service::Container) -> Self;
}

#[async_trait::async_trait]
pub trait Recurring {
  async fn execute(&self) -> anyhow::Result<()>;
}

pub struct Container {
  config: config::Manager,
  services: service::Container,
  handles: Arc<Mutex<Option<Vec<Handle>>>>,
}

impl Container {
  pub fn new(config: config::Manager, services: service::Container) -> Self {
    Self {
      config,
      services,
      handles: Arc::new(Mutex::new(None)),
    }
  }

  pub async fn abort(&self) {
    {
      let mut handles = self.handles.clone().lock_owned().await;
      if let Some(handles) = &*handles {
        for handle in handles {
          handle.abort.abort();
        }
      }
      *handles = None;
    }
  }

  pub async fn cancel(&self) {
    {
      let mut handles = self.handles.clone().lock_owned().await;
      if let Some(handles) = &mut *handles {
        for handle in handles.iter() {
          handle.token.cancel();
        }

        for handle in handles.drain(0..) {
          if let Err(error) = handle.join.await {
            tracing::error! {
              %error,
              "Joining process handle on cancel failed"
            }
          }
        }
      }
      *handles = None;
    }
  }

  pub async fn join(&self) {
    {
      let mut handles = self.handles.clone().lock_owned().await;
      if let Some(handles) = &mut *handles {
        for handle in handles.drain(0..) {
          if let Err(error) = handle.join.await {
            tracing::error! {
              %error,
              "Joining process handle failed"
            }
          }
        }
      }
      *handles = None;
    }
  }
}

impl Container {
  pub async fn spawn(&self) {
    let config = self.config.values_async().await;
    let specs = vec![
      self.make_recurring_spec::<discover::Process>(config.discover_interval),
      self.make_recurring_spec::<ping::Process>(config.ping_interval),
      self.make_recurring_spec::<measure::Process>(config.measure_interval),
      self.make_recurring_spec::<push::Process>(config.push_interval),
      self.make_recurring_spec::<update::Process>(config.update_interval),
    ];

    {
      let mut handles = self.handles.clone().lock_owned().await;
      *handles = Some(specs.into_iter().map(Handle::recurring).collect());
    }
  }

  fn make_recurring_spec<T: Process + Recurring + Send + Sync + 'static>(
    &self,
    interval: chrono::Duration,
  ) -> RecurringSpec {
    RecurringSpec {
      process: Box::new(T::new(self.config.clone(), self.services.clone())),
      interval,
    }
  }
}

struct Handle {
  token: tokio_util::sync::CancellationToken,
  abort: tokio::task::AbortHandle,
  join: tokio::task::JoinHandle<()>,
}

struct RecurringSpec {
  process: Box<dyn Recurring + Sync + Send>,
  interval: chrono::Duration,
}

impl Handle {
  fn recurring(spec: RecurringSpec) -> Self {
    let token = tokio_util::sync::CancellationToken::new();
    let child_token = token.child_token();
    let join = tokio::spawn(async move {
      let mut interval =
        tokio::time::interval(std::time::Duration::from_millis(
          spec.interval.num_milliseconds() as u64,
        ));
      loop {
        tokio::select! {
            _ = child_token.cancelled() => { return; },
            _ = async {
                if let Err(error) = spec.process.execute().await {
                    tracing::error! { %error, "Process execution failed" };
                }

                interval.tick().await;
            } => { }
        }
      }
    });
    let abort = join.abort_handle();
    Self { token, abort, join }
  }
}
