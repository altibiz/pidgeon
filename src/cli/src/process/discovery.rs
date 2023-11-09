use std::net::SocketAddr;

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
impl super::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let addresses = self.services.network.scan().await;

    let config = self.config.reload_async().await?;

    let discovered = Vec::new();
    for address in addresses {}

    Ok(())
  }
}

impl Process {
  fn r#match(
    &self,
    config: &config::Parsed,
    destination: modbus::Destination,
  ) -> Option<config::Device> {
    for (kind, device) in config.modbus.devices {
      let detect_registers = device.detect.normalize().into_iter().map(|register| modbus::DetectRegister::<modbus::RegisterKind> {
        address: register.address,
        storage: modbus::RegisterKind::
      })
    }
  }
}
