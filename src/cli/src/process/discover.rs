use futures::future::join_all;

use crate::{service::*, *};

// TODO: set timeout

pub struct Process {
  config: config::Manager,
  services: service::Container,
}

impl process::Process for Process {
  fn new(config: config::Manager, services: service::Container) -> Self {
    Self { config, services }
  }
}

#[async_trait::async_trait]
impl process::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let addresses = self.services.network().scan().await;

    let config = self.config.reload_async().await?;

    join_all(
      join_all(
        addresses
          .into_iter()
          .flat_map(modbus::Destination::r#for)
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
    config: &config::Values,
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
      .flatten()
      .map(|device| self.match_id(device, destination)),
    )
    .await
    .into_iter()
    .flatten()
  }

  async fn match_device(
    &self,
    device: config::Device,
    destination: modbus::Destination,
  ) -> Option<config::Device> {
    self
      .services
      .modbus()
      .read_from_destination(destination, device.detect.clone())
      .await
      .ok()?
      .into_iter()
      .all(|register| register.matches())
      .then_some(device)
  }

  async fn match_id(
    &self,
    device: config::Device,
    destination: modbus::Destination,
  ) -> Option<DeviceMatch> {
    self
      .services
      .modbus()
      .read_from_destination(destination, device.id)
      .await
      .ok()
      .map(|id_registers| DeviceMatch {
        kind: device.kind.clone(),
        destination,
        id: modbus::make_id(device.kind, id_registers),
      })
  }

  async fn consolidate(&self, device_match: DeviceMatch) {
    match self
      .services
      .db()
      .get_device(device_match.id.as_str())
      .await
    {
      Ok(Some(_)) => {
        let now = chrono::Utc::now();
        self.services.db().update_device_destination(
          &device_match.id,
          db::to_network(device_match.destination.address.ip()),
          db::to_db_slave(device_match.destination.slave),
          now,
          now,
        );
      }
      Ok(None) => {
        let now = chrono::Utc::now();
        self
          .services
          .db()
          .insert_device(db::Device {
            id: device_match.id,
            kind: device_match.kind,
            status: db::DeviceStatus::Healthy,
            seen: now,
            pinged: now,
            address: db::to_network(device_match.destination.address.ip()),
            slave: db::to_db_slave(device_match.destination.slave),
          })
          .await;
      }
      _ => {}
    }
  }
}
