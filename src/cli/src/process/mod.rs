pub mod discover;
pub mod measure;
pub mod ping;
pub mod push;
pub mod update;

use std::sync::Arc;

use crate::{config, service::*};

pub struct Services {
  db: db::Service,
  cloud: cloud::Service,
  modbus: modbus::Service,
  network: network::Service,
  hardware: hardware::Service,
}

impl Services {
  pub fn new(config: config::Values) -> Self {
    Self {
      db: db::Service::new(config.clone()),
      cloud: cloud::Service::new(config.clone()),
      modbus: modbus::Service::new(config.clone()),
      network: network::Service::new(config.clone()),
      hardware: hardware::Service::new(config.clone()),
    }
  }
}

pub trait Process {
  fn new(config: config::Manager, services: Arc<Services>) -> Self;
}

#[async_trait::async_trait]
pub trait Recurring: Process {
  async fn execute(&self) -> anyhow::Result<()>;
}



pub fn 
