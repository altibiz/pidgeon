use crate::*;

// TODO: detect parameters

#[derive(Debug, Clone)]
pub(crate) struct Service {}

impl service::Service for Service {
  fn new(_config: config::Values) -> Self {
    Self {}
  }
}

#[derive(Debug, Clone)]
pub(crate) struct SerialPort {
  pub(crate) path: String,
  pub(crate) baud_rate: u32,
}

impl Service {
  #[tracing::instrument(skip(self))]
  pub(crate) async fn scan_modbus(&self) -> Vec<SerialPort> {
    let available = match serialport::available_ports() {
      Ok(available) => available,
      Err(_) => return Vec::new(),
    };

    available
      .into_iter()
      .filter(|port| port.port_type == serialport::SerialPortType::Unknown)
      .filter(|port| FILE_PATH_REGEX.is_match(&port.port_name))
      .map(|port| SerialPort {
        path: port.port_name,
        baud_rate: 38400,
      })
      .collect::<Vec<_>>()
  }
}

lazy_static::lazy_static! {
  static ref FILE_PATH_REGEX: regex::Regex = {
    #[allow(clippy::unwrap_used)] // NOTE: valid static file path regex
    let regex = regex::Regex::new("^/[^/]+(/[^/]+)+$").unwrap();
    regex
  };
}
