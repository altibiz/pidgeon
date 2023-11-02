use std::{future::Future, net::SocketAddr, pin::Pin};

use tokio_modbus::SlaveId;

use super::register::*;

#[derive(Debug, Clone)]
pub struct Slave {
  pub num: SlaveId,
  pub id: String,
  pub kind: String,
}

#[derive(Debug, Clone)]
pub struct Measurement {
  pub slave: Slave,
  pub registers: MeasurementRegister<RegisterValue>,
}

#[derive(Debug, Clone)]
pub struct Discovery {}

pub trait Server {
  fn address(&self) -> SocketAddr;

  fn slaves(&self) -> Vec<Slave>;

  fn measure(&mut self) -> Pin<Box<dyn Future<Output = Vec<Measurement>>>>;
}
