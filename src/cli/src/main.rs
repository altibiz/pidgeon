#![deny(
  unsafe_code,
  // reason = "Let's just not do it"
)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  // reason = "We have to handle errors properly"
)]

mod cloud;
mod config;
mod db;
mod modbus;
mod runtime;
mod scan;

use crate::runtime::{Runtime, RuntimeError};

fn main() -> Result<(), RuntimeError> {
    let runtime = Runtime::new()?;
    runtime.start()
}
