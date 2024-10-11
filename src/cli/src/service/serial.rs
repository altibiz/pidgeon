use crate::*;

#[derive(Debug, Clone)]
pub(crate) struct Service {}

impl service::Service for Service {
  fn new(config: config::Values) -> Self {
    Self {}
  }
}

#[derive(Debug, Clone)]
pub struct SerialPort {
  path: String,
  baud_rate: u32,
}

impl Service {
  #[tracing::instrument(skip(self))]
  pub(crate) async fn scan_modbus(&self) -> Vec<SerialPort> {
    vec![]
  }
}
