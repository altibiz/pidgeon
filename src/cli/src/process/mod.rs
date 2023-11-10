mod discovery;
mod measurement;
mod ping;
mod push;
mod update;

use crate::{config, service::*};

pub struct Services {
  db: db::Client,
  cloud: cloud::Client,
  modbus: modbus::Client,
  network: network::Client,
  hardware: hardware::Client,
}

pub trait Process {
  fn new(config: config::Manager, services: Services) -> Self;
}

#[async_trait::async_trait]
pub trait Recurring: Process {
  async fn execute(&self) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait Background: Process {
  async fn execute(&self);
}
