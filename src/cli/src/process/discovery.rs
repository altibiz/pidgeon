use futures::future::join_all;

use crate::{
  config::{self, ParsedDevice},
  service::*,
};

// TODO: set timeout

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

    join_all(
      join_all(
        addresses
          .into_iter()
          .map(modbus::Destination::r#for)
          .flatten()
          .map(|destination| self.match_destination(&config, destination)),
      )
      .await
      .into_iter()
      .flatten()
      .map(|r#match| self.consolidate(r#match)),
    )
    .await;

    Ok(())
  }
}

#[derive(Debug, Clone)]
struct DeviceMatch {
  id: String,
  kind: String,
  destination: modbus::Destination,
}

impl Process {
  async fn match_destination(
    &self,
    config: &config::Parsed,
    destination: modbus::Destination,
  ) -> impl Iterator<Item = DeviceMatch> {
    join_all(
      join_all(
        config
          .modbus
          .devices
          .values()
          .map(move |device| self.match_device(device.clone(), destination)),
      )
      .await
      .into_iter()
      .filter_map(std::convert::identity)
      .map(|device| self.match_id(device, destination)),
    )
    .await
    .into_iter()
    .filter_map(std::convert::identity)
  }

  async fn match_device(
    &self,
    device: ParsedDevice,
    destination: modbus::Destination,
  ) -> Option<ParsedDevice> {
    self
      .services
      .modbus
      .read_from_destination(destination, device.detect.clone())
      .await
      .ok()?
      .into_iter()
      .all(|register| register.matches())
      .then_some(device)
  }

  async fn match_id(
    &self,
    device: ParsedDevice,
    destination: modbus::Destination,
  ) -> Option<DeviceMatch> {
    self
      .services
      .modbus
      .read_from_destination(destination, device.id)
      .await
      .ok()
      .map(|id_registers| DeviceMatch {
        kind: device.kind.clone(),
        destination,
        id: modbus::make_id(device.kind, id_registers),
      })
  }

  async fn consolidate(&self, r#match: DeviceMatch) {
    match self.services.db.get_device(r#match.id.as_str()).await {
      Ok(None) => {
        self
          .services
          .db
          .insert_device(db::Device {
            id: r#match.id,
            kind: r#match.kind,
            status: db::DeviceStatus::Healthy,
            seen: chrono::Utc::now(),
            address: db::to_network(r#match.destination.address.ip()),
            slave: r#match.destination.slave.map(|slave| slave as i32),
          })
          .await;
      }
      _ => {}
    }
  }
}
