pub mod cloud;
pub mod db;
pub mod hardware;
pub mod modbus;
pub mod network;

use crate::*;

pub trait Service {
  fn new(config: config::Values) -> Self;
}

#[derive(Debug)]
struct Values {
  db: db::Service,
  cloud: cloud::Service,
  modbus: modbus::Service,
  network: network::Service,
  hardware: hardware::Service,
}

#[derive(Debug, Clone)]
pub struct Container {
  values: Arc<Values>,
}

impl Container {
  pub fn new(config: config::Values) -> Self {
    Self {
      values: Arc::new(Values {
        db: db::Service::new(config.clone()),
        cloud: cloud::Service::new(config.clone()),
        modbus: modbus::Service::new(config.clone()),
        network: network::Service::new(config.clone()),
        hardware: hardware::Service::new(config.clone()),
      }),
    }
  }

  pub fn db(&self) -> &db::Service {
    &self.values.db
  }

  pub fn cloud(&self) -> &cloud::Service {
    &self.values.cloud
  }

  pub fn modbus(&self) -> &modbus::Service {
    &self.values.modbus
  }

  pub fn network(&self) -> &network::Service {
    &self.values.network
  }

  pub fn hardware(&self) -> &hardware::Service {
    &self.values.hardware
  }
}
