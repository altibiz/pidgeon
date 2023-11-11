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
  Ok(())
}
