use thiserror::Error;

use crate::services::{ServiceError, Services};

#[derive(Debug)]
pub struct Runtime {
  r#async: tokio::runtime::Runtime,
  services: Services,
}

struct Interval {
  token: tokio_util::sync::CancellationToken,
  handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
  #[error("Logging setup error")]
  LogSetup,

  #[error("Async runtime error")]
  AsyncRuntime(#[from] std::io::Error),

  #[error("Service error")]
  Service(#[from] ServiceError),
}

macro_rules! interval {
  ($rt:ident,$handler:ident,$duration:literal) => {{
    let services = $rt.services.clone();
    let token = tokio_util::sync::CancellationToken::new();
    let child_token = token.child_token();
    let handle = $rt.r#async.spawn(async move {
      let mut interval =
        tokio::time::interval(std::time::Duration::from_millis($duration));

      loop {
        tokio::select! {
            _ = child_token.cancelled() => {
                return;
            },
            _ = async {
                if let Err(error) = services.$handler().await {
                    tracing::error! { %error, "interval handler failed" };
                }

                interval.tick().await;
            } => {

            }
        }
      }
    });

    Interval { token, handle }
  }};
}

macro_rules! kill_intervals {
    [$($interval:expr),*] => {
        $(
            $interval.token.cancel();
        )*

        $(
            if let Err(error) = $interval.handle.await {
                tracing::error! { %error, "Interval exited with error" }
            }
        )*
    };
}

impl Runtime {
  pub fn new() -> Result<Self, RuntimeError> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    if tracing::subscriber::set_global_default(subscriber).is_err() {
      return Err(RuntimeError::LogSetup);
    };

    let services = Services::new()?;

    let r#async = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(4)
      .enable_all()
      .build()?;

    let runtime = Self { services, r#async };

    Ok(runtime)
  }

  pub fn start(&self) -> Result<(), RuntimeError> {
    self.r#async.block_on(async { self.start_async().await })
  }

  async fn start_async(&self) -> Result<(), RuntimeError> {
    self.services.on_setup().await?;

    let scan = interval!(self, on_scan, 60000);
    let pull = interval!(self, on_pull, 60000);
    let push = interval!(self, on_push, 60000);

    if let Err(error) = tokio::signal::ctrl_c().await {
      tracing::error! { %error, "Failed waiting for Ctrl+C" }
    }

    kill_intervals![scan, pull, push];

    Ok(())
  }
}
