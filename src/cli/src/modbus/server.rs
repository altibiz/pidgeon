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

#[derive(Debug, Clone)]
pub struct DiscoveryDevice {
  pub kind: String,
  pub detect: Vec<DetectRegister<RegisterKind>>,
  pub id: Vec<IdRegister<RegisterKind>>,
  pub measurement: Vec<MeasurementRegister<RegisterKind>>,
}

#[derive(Debug, Clone)]
pub struct Discovery {
  pub devices: DiscoveryDevice,
}

#[async_trait::async_trait]
pub trait Server {
  fn address(&self) -> SocketAddr;

  fn slaves(&self) -> Vec<Slave>;

  async fn measure(&self) -> Vec<Measurement>;
}
