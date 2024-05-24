#![deny(
  unsafe_code,
  // reason = "Let's just not do it"
)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  clippy::unreachable,
  clippy::arithmetic_side_effects
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

// TODO: configurable timeouts

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
  let manager = config::Manager::new().await?; // NITPICK: handle this more appropriately
  let config = manager.values().await;
  let id = config.cloud.id.clone();

  println!("Started: {id}");

  let services = service::Container::new(config.clone());
  let processes = process::Container::new(manager.clone(), services.clone());

  let log_level = config.log_level.to_string();
  tracing::subscriber::set_global_default(
    tracing_subscriber::FmtSubscriber::builder()
      .with_env_filter(
        tracing_subscriber::EnvFilter::builder()
          .with_default_directive(
            tracing::level_filters::LevelFilter::WARN.into(),
          )
          .from_env()?
          .add_directive(format!("pidgeon={log_level}").parse()?),
      )
      .finish(),
  )?;

  services
    .db()
    .migrate()
    .timeout(futures_time::time::Duration::from_millis(10_000))
    .await??;

  processes.startup().await?;

  if let Err(error) = tokio::signal::ctrl_c().await {
    tracing::error!("Failed waiting for ctrlc signal {}", error);
  };
  if let Err(error) = processes
    .shutdown()
    .timeout(futures_time::time::Duration::from_millis(10_000))
    .await
  {
    tracing::error!("Timed out shutting down processes {}", error);
  }

  Ok(())
}
