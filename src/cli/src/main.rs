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

mod config;
mod process;
mod service;

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
  let manager = config::Manager::new_async().await?;
  let config = manager.values_async().await;

  let services = service::Container::new(config.clone());
  let processes = process::Container::new(manager.clone(), services.clone());

  tracing::subscriber::set_global_default(
    tracing_subscriber::FmtSubscriber::builder()
      .with_max_level(config.log_level)
      .finish(),
  )?;

  services.db().migrate().await?; // NITPICK: handle this more appropriately

  processes.spawn().await;
  if let Err(error) = tokio::signal::ctrl_c().await {
    tracing::error!("Failed waiting for ctrlc signal {:?}", error);
  };
  processes.cancel().await;

  Ok(())
}
