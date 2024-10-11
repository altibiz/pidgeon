pub mod cloud;
pub mod db;
pub mod i2c;
pub mod modbus;
pub mod net;
pub mod serial;

use std::sync::Arc;

use crate::*;

pub(crate) trait Service {
  fn new(config: config::Values) -> Self;
}

#[derive(Debug)]
struct Values {
  db: db::Service,
  cloud: cloud::Service,
  modbus: modbus::Service,
  net: net::Service,
  i2c: i2c::Service,
  serial: serial::Service,
}

#[derive(Debug, Clone)]
pub(crate) struct Container {
  values: Arc<Values>,
}

impl Container {
  pub(crate) fn new(config: config::Values) -> Self {
    Self {
      values: Arc::new(Values {
        db: db::Service::new(config.clone()),
        cloud: cloud::Service::new(config.clone()),
        modbus: modbus::Service::new(config.clone()),
        net: net::Service::new(config.clone()),
        i2c: i2c::Service::new(config.clone()),
        serial: serial::Service::new(config.clone()),
      }),
    }
  }

  #[inline]
  pub(crate) fn db(&self) -> &db::Service {
    &self.values.db
  }

  #[inline]
  pub(crate) fn cloud(&self) -> &cloud::Service {
    &self.values.cloud
  }

  #[inline]
  pub(crate) fn modbus(&self) -> &modbus::Service {
    &self.values.modbus
  }

  #[inline]
  pub(crate) fn net(&self) -> &net::Service {
    &self.values.net
  }

  #[inline]
  pub(crate) fn i2c(&self) -> &i2c::Service {
    &self.values.i2c
  }

  #[inline]
  pub(crate) fn serial(&self) -> &serial::Service {
    &self.values.serial
  }
}
