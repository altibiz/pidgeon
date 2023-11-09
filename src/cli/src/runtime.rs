use std::time::Duration;

use thiserror::Error;

use crate::{
  config::{self, Manager, ParseError},
  services::{ServiceError, Services},
};

#[derive(Debug)]
pub struct Runtime {
  scan_interval: Duration,
  pull_interval: Duration,
  push_interval: Duration,
  r#async: tokio::runtime::Runtime,
  config_manager: Manager,
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

  #[error("Config manager error")]
  ConfigManager(#[from] ParseError),

  #[error("Service error")]
  Service(#[from] ServiceError),
}

macro_rules! interval {
  ($rt:ident,$services:ident,$handler:ident,$duration:expr) => {{
    let services = $services.clone();
    let token = tokio_util::sync::CancellationToken::new();
    let child_token = token.child_token();
    let duration = $duration.clone();
    let handle = $rt.r#async.spawn(async move {
      let mut interval = tokio::time::interval(duration);

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
    let config_manager = Manager::new()?;
    let config = config_manager.config()?;

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
      .with_max_level(match config.runtime.log_level {
        config::LogLevel::Trace => tracing::level_filters::LevelFilter::TRACE,
        config::LogLevel::Debug => tracing::level_filters::LevelFilter::DEBUG,
        config::LogLevel::Info => tracing::level_filters::LevelFilter::INFO,
        config::LogLevel::Warn => tracing::level_filters::LevelFilter::DEBUG,
        config::LogLevel::Error => tracing::level_filters::LevelFilter::ERROR,
      })
      .finish();
    if tracing::subscriber::set_global_default(subscriber).is_err() {
      return Err(RuntimeError::LogSetup);
    };

    let r#async = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(4)
      .enable_all()
      .build()?;

    let runtime = Self {
      scan_interval: Duration::from_millis(config.runtime.scan_interval),
      pull_interval: Duration::from_millis(config.runtime.pull_interval),
      push_interval: Duration::from_millis(config.runtime.push_interval),
      r#async,
      config_manager,
    };

    Ok(runtime)
  }

  pub fn start(&self) -> Result<(), RuntimeError> {
    self.r#async.block_on(async { self.start_async().await })
  }

  async fn start_async(&self) -> Result<(), RuntimeError> {
    let services = Services::new(self.config_manager.clone()).await?;

    services.on_setup().await?;

    let scan = interval!(self, services, on_scan, self.scan_interval);
    let pull = interval!(self, services, on_pull, self.pull_interval);
    let push = interval!(self, services, on_push, self.push_interval);

    if let Err(error) = tokio::signal::ctrl_c().await {
      tracing::error! { %error, "Failed waiting for Ctrl+C" }
    }

    kill_intervals![scan, pull, push];

    Ok(())
  }
}
