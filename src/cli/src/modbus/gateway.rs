use std::net::SocketAddr;

use super::server::*;

#[derive(Debug, Clone)]
pub struct Gateway {
  address: SocketAddr,
  slaves: Vec<Slave>,
}

impl Gateway {
  pub async fn new(address: SocketAddr) -> anyhow::Result<Self> {
    let mut slaves = Vec::new();
    for num in
      tokio_modbus::Slave::min_device().0..tokio_modbus::Slave::max_device().0
    {
      if let Some(connection) =
        super::conn::Connection::connect_slave(address, Slave(num)).await
      {
      }
    }
  }
}

#[async_trait::async_trait]
impl Server for Gateway {
  fn address(&self) -> SocketAddr {
    self.address
  }

  fn slaves(&self) -> Vec<Slave> {
    self.slaves
  }

  async fn measure(&self) -> Vec<Measurement> {
    todo!()
  }
}
