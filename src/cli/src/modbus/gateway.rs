use std::net::SocketAddr;

use tokio_modbus::Slave;

use super::server::*;

#[derive(Debug, Clone)]
pub struct Gateway {
  address: SocketAddr,
  slaves: Vec<Slave>,
}

impl Gateway {
  pub async fn new(
    address: SocketAddr,
    timeout: chrono::Duration,
    backoff: chrono::Duration,
    retries: usize,
  ) -> anyhow::Result<Self> {
    let mut slaves = Vec::new();
    for num in Slave::min_device().0..Slave::max_device().0 {
      if let Some(connection) = super::conn::Connection::connect_slave(
        address,
        Slave(num),
        timeout,
        backoff,
        retries,
      )
      .await
      {}
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
