use super::server::*;

#[derive(Debug, Clone)]
pub struct Gateway {}

impl Gateway {
  pub fn new() -> anyhow::Result<Self> {
    todo!()
  }
}

#[async_trait::async_trait]
impl Server for Gateway {
  fn address(&self) -> SocketAddr {
    todo!()
  }

  fn slaves(&self) -> Vec<Slave> {
    todo!()
  }

  async fn measure(&self) -> Vec<Measurement> {
    todo!()
  }
}
