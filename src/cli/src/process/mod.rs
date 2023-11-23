mod discover;
mod health;
mod measure;
mod ping;
mod push;
mod update;

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{config, service};

// OPTIMIZE: all processes by removing unnecessary cloning at least

pub(crate) trait Process {
  fn process_name(&self) -> &'static str {
    std::any::type_name::<Self>()
  }
}

#[async_trait::async_trait]
pub(crate) trait Recurring: Process {
  async fn execute(&self) -> anyhow::Result<()>;
}

pub(crate) struct Container {
  config: config::Manager,
  services: service::Container,
  handles: Arc<Mutex<Option<Vec<Handle>>>>,
}

impl Container {
  pub(crate) fn new(
    config: config::Manager,
    services: service::Container,
  ) -> Self {
    Self {
      config,
      services,
      handles: Arc::new(Mutex::new(None)),
    }
  }

  pub(crate) async fn abort(&self) {
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

  pub(crate) async fn cancel(&self) {
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
}

macro_rules! make_recurring_spec {
  ($self: ident, $type: ty, $interval: expr) => {
    RecurringSpec {
      process: Box::new(<$type>::new(
        $self.config.clone(),
        $self.services.clone(),
      )),
      interval: $interval,
    }
  };
}

impl Container {
  pub(crate) async fn spawn(&self) {
    let config = self.config.values().await;
    let specs = vec![
      make_recurring_spec!(self, discover::Process, config.discover_interval),
      make_recurring_spec!(self, ping::Process, config.ping_interval),
      make_recurring_spec!(self, measure::Process, config.measure_interval),
      make_recurring_spec!(self, push::Process, config.push_interval),
      make_recurring_spec!(self, update::Process, config.update_interval),
      make_recurring_spec!(self, health::Process, config.health_interval),
    ];

    {
      let mut handles = self.handles.clone().lock_owned().await;
      *handles = Some(specs.into_iter().map(Handle::recurring).collect());
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
                  tracing::error!(
                    "Process execution failed {} for {}",
                    error,
                    spec.process.process_name()
                  );
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
