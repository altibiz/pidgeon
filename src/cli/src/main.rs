#![deny(
  unsafe_code,
  // reason = "Let's just not do it"
)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  clippy::unreachable,
  // reason = "We have to handle errors properly"
)]
#![deny(
  clippy::dbg_macro,
  // reason = "Use tracing instead"
)]

mod config;
mod process;
mod service;

use futures_time::future::FutureExt;

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
  let manager = config::Manager::new().await?;
  let config = manager.values().await;

  let services = service::Container::new(config.clone());
  let processes = process::Container::new(manager.clone(), services.clone());

  tracing::subscriber::set_global_default(
    tracing_subscriber::FmtSubscriber::builder()
      .with_max_level(config.log_level)
      .with_env_filter(tracing_subscriber::EnvFilter::builder()
        .from_env()?
        .add_directive("warn".parse()?)
        .add_directive("pidgeon-cli".parse()?)
      )
      .finish(),
  )?;

  services.db().migrate().await?; // NITPICK: handle this more appropriately

  processes.spawn().await;
  if let Err(error) = tokio::signal::ctrl_c().await {
    tracing::error!("Failed waiting for ctrlc signal {}", error);
  };
  if let Err(error) = processes
    .cancel()
    .timeout(futures_time::time::Duration::from_millis(10000))
    .await
  {
    tracing::error!("Timed out cancelling processes {}", error);
    processes.abort().await;
  }

  Ok(())
}
