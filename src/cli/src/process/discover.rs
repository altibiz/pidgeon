use futures::future::join_all;

use crate::{service::*, *};

// TODO: set timeout

pub(crate) struct Process {
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
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let config = self.config.reload_async().await;

    let addresses = self.services.network().scan_modbus().await;
    let addresses_len = addresses.len();

    let device_matches = join_all(
      addresses
        .into_iter()
        .flat_map(modbus::Destination::r#for)
        .map(|destination| self.match_destination(&config, destination)),
    )
    .await;
    let device_matches_len = device_matches.len();

    let consolidated_matches = join_all(
      device_matches
        .into_iter()
        .flatten()
        .map(|r#match| self.consolidate(r#match)),
    )
    .await;
    let consolidated_matches_len = consolidated_matches.len();

    tracing::info!(
      "Scanned {:?} modbus servers with {:?} devices of which {:?} were consolidated",
      addresses_len,
      device_matches_len,
      consolidated_matches_len
    );

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
    let destination_matches = join_all(
      config
        .modbus
        .devices
        .values()
        .map(move |device| self.match_device(device.clone(), destination)),
    )
    .await;

    let id_matches = join_all(
      destination_matches
        .into_iter()
        .flatten()
        .map(|device| self.match_id(device, destination)),
    )
    .await;

    let device_matches = id_matches.into_iter().flatten();

    device_matches
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

  #[tracing::instrument(skip(self))]
  async fn consolidate(
    &self,
    device_match: DeviceMatch,
  ) -> Option<DeviceMatch> {
    match self
      .services
      .db()
      .get_device(device_match.id.as_str())
      .await
    {
      Err(error) => {
        tracing::error!("Failed fetching device {}", error);

        return None;
      }
      Ok(Some(_)) => {
        let now = chrono::Utc::now();
        if let Err(error) = self
          .services
          .db()
          .update_device_destination(
            &device_match.id,
            db::to_network(device_match.destination.address.ip()),
            db::to_db_slave(device_match.destination.slave),
            now,
            now,
          )
          .await
        {
          tracing::error!("Failed updating device destination {}", error);

          return None;
        }
      }
      Ok(None) => {
        let now = chrono::Utc::now();
        if let Err(error) = self
          .services
          .db()
          .insert_device(db::Device {
            id: device_match.id.clone(),
            kind: device_match.kind.clone(),
            status: db::DeviceStatus::Healthy,
            seen: now,
            pinged: now,
            address: db::to_network(device_match.destination.address.ip()),
            slave: db::to_db_slave(device_match.destination.slave),
          })
          .await
        {
          tracing::error!("Failed inserting new device {}", error);

          return None;
        }
      }
    }

    tracing::debug!("Matched device");

    return Some(device_match);
  }
}
