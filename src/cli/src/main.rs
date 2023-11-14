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
async fn main() -> anyhow::Result<()> {
  let manager = config::Manager::new_async().await?;
  let config = manager.values_async().await;

  let services = service::Container::new(config.clone());
  let processes = process::Container::new(manager.clone(), services.clone());

  services.db().migrate().await?;
  processes.spawn().await;
  tokio::signal::ctrl_c().await?;
  processes.cancel().await;

  Ok(())
}
