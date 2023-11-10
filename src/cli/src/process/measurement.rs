use std::os::unix::raw::dev_t;

use crate::{config, service::*};

pub struct Process {
  config: config::Manager,
  services: super::Services,
}

impl super::Process for Process {
  fn new(config: config::Manager, services: super::Services) -> Self {
    Self { config, services }
  }
}

#[async_trait::async_trait]
impl super::Background for Process {
  async fn execute(&self) {
    let mut streams = Vec::new();

    let config = self.config.reload_async().await.unwrap();

    self
      .services
      .db
      .get_devices()
      .await
      .unwrap()
      .into_iter()
      .filter_map(|device| {
        config
          .modbus
          .devices
          .values()
          .filter(|device_config| device_config.kind == device.kind)
          .next()
          .map(|device_config| StreamDevice {
            id: device.id,
            kind: device.kind,
            destination: modbus::Destination {
              address: network::to_socket(db::to_ip(device.address)),
              slave: device.slave.map(),
            },
          })
      });

    loop {}
  }
}

#[derive(Clone, Debug)]
struct StreamDevice {
  id: String,
  kind: String,
  destination: modbus::Destination,
  id_registers: Vec<modbus::IdRegister<modbus::RegisterKind>>,
  measurement_registers: Vec<modbus::IdRegister<modbus::RegisterKind>>,
}
