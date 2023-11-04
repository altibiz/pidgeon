use std::net::SocketAddr;

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

#[async_trait::async_trait]
pub trait Server {
  fn address(&self) -> SocketAddr;

  fn slaves(&self) -> Vec<Slave>;

  async fn measure(&self) -> Vec<Measurement>;
}
