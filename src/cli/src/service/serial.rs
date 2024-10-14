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
  pub(crate) async fn scan_serial(&self) -> Vec<SerialPort> {
    let available = match serialport::available_ports() {
      Ok(available) => available,
      Err(_) => return Vec::new(),
    };

    available
      .into_iter()
      .filter(|port| port.port_type == serialport::SerialPortType::Unknown)
      .map(|port| SerialPort {
        path: port.port_name,
        baud_rate: 57600,
      })
      .collect::<Vec<_>>()
  }
}
