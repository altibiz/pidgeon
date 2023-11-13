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

use std::sync::Arc;

use process::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let config = config::Manager::new()?;

  let services = Arc::new(Services::new(config.config_async().await?));

  let discover = discover::Process::new(config.clone(), services.clone());
  let ping = ping::Process::new(config.clone(), services.clone());
  let measure = measure::Process::new(config.clone(), services.clone());
  let push = push::Process::new(config.clone(), services.clone());
  let update = update::Process::new(config.clone(), services.clone());

  Ok(())
}

struct Interval {
  token: tokio_util::sync::CancellationToken,
  handle: tokio::task::JoinHandle<()>,
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
