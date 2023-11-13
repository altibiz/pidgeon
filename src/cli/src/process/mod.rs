pub mod discover;
pub mod measure;
pub mod ping;
pub mod push;
pub mod update;

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{config, service};

pub trait Process {
  fn new(config: config::Manager, services: service::Container) -> Self;
}

#[async_trait::async_trait]
pub trait Recurring {
  async fn execute(&self) -> anyhow::Result<()>;
}

struct Handle {
  token: tokio_util::sync::CancellationToken,
  abort: tokio::task::AbortHandle,
  join: tokio::task::JoinHandle<()>,
}

pub struct Processes {
  config: config::Manager,
  services: service::Container,
  handles: Arc<Mutex<Option<Vec<Handle>>>>,
}

impl Processes {
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

  pub async fn join(&self) {
    {
      let mut handles = self.handles.clone().lock_owned().await;
      if let Some(handles) = &mut *handles {
        for handle in handles.iter() {
          handle.token.cancel();
        }

        for handle in handles.drain(0..) {
          handle.join.await;
        }
      }
      *handles = None;
    }
  }

  pub async fn spawn(&self) {
    let config = self.config.values_async().await;
    struct Spec {
      process: Box<dyn Recurring + Sync + Send>,
      interval: chrono::Duration
    }

    {
      let mut handles = self.handles.clone().lock_owned().await;
        *handles = Some(vec![
        Spec {
          process: Box::new( discover::Process::new(self.config.clone(), self.services.clone())),
          interval: config.discover_interval,
        },
        ].into_iter().map(|Spec { process, interval }| { 
          let token = tokio_util::sync::CancellationToken::new();
          let child_token = token.child_token();
          let join = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(interval.num_milliseconds() as u64));
            loop {
              tokio::select! {
                  _ = child_token.cancelled() => {
                      return;
                  },
                  _ = async {
                      if let Err(error) = process.execute().await {
                          tracing::error! { %error, "Process execution failed" };
                      }

                      interval.tick().await;
                  } => {

                  }
              }
            }
          });
          
            Handle {
              token,
              abort: join.abort_handle(),
            join
            
          }
        }).collect());
      }
  }
}
