use futures::future::join_all;
use futures_time::future::FutureExt;

#[allow(unused_imports)]
use crate::{service::*, *};

use self::modbus::connection::Device;

pub(crate) struct Process {
  #[allow(unused)]
  config: config::Manager,

  #[allow(unused)]
  services: service::Container,
}

impl Process {
  pub(crate) fn new(
    config: config::Manager,
    services: service::Container,
  ) -> Self {
    Self { config, services }
  }
}

impl super::Process for Process {}

#[async_trait::async_trait]
impl super::Recurring for Process {
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let config = self.config.values().await;

    let addresses = self.services.net().scan_modbus().await;
    let addresses_len = addresses.len();

    let ports = self.services.serial().scan_modbus().await;
    let ports_len = ports.len();

    let device_matches = join_all(
      addresses
        .into_iter()
        .map(|address| self.match_modbus_device(&config, Device::Tcp(address)))
        .chain(ports.into_iter().map(|port| {
          self.match_modbus_device(
            &config,
            Device::Rtu {
              path: port.path.clone(),
              baud_rate: port.baud_rate,
            },
          )
        })),
    )
    .await
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let device_matches_len = device_matches.len();

    let consolidated_matches = join_all(
      device_matches
        .into_iter()
        .map(|device_match| self.consolidate(device_match)),
    )
    .await;
    let consolidated_matches_len = consolidated_matches.len();

    tracing::info!(
      "Scanned {:?} modbus servers with {:?} devices of which {:?} were consolidated",
      addresses_len.saturating_add(ports_len),
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
  #[tracing::instrument(skip(self, config))]
  async fn match_modbus_device(
    &self,
    config: &config::Values,
    modbus_device: Device,
  ) -> Vec<DeviceMatch> {
    if let Some(device_match) = self
      .match_destination(
        config,
        modbus::Destination::standalone_for(modbus_device.clone()),
      )
      .await
    {
      return vec![device_match];
    }

    let mut device_matches = Vec::new();
    for destination in modbus::Destination::slaves_for(modbus_device) {
      if let Some(device_match) =
        self.match_destination(config, destination).await
      {
        device_matches.push(device_match);
      } else {
        break;
      }
    }

    device_matches
  }

  #[tracing::instrument(skip(self, config))]
  async fn match_destination(
    &self,
    config: &config::Values,
    destination: modbus::Destination,
  ) -> Option<DeviceMatch> {
    let matching_destination = destination.clone();
    let device = join_all(config.modbus.devices.values().map(move |device| {
      let matching_destination = matching_destination.clone();
      Box::pin(
        self
          .match_device(device.clone(), matching_destination)
          .timeout(timeout_from_chrono(config.modbus.discovery_timeout)),
      )
    }))
    .await
    .into_iter()
    .find(|device| device.as_ref().ok().is_some_and(|x| x.is_some()))?
    .ok()??;

    let device_match = self
      .match_id(device, destination)
      .timeout(timeout_from_chrono(config.modbus.discovery_timeout))
      .await
      .ok()
      .flatten()?;

    tracing::debug!(
      "Matched {:?} devices on {:?}",
      device_match.id,
      device_match.destination
    );

    Some(device_match)
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
            match &device_match.destination.device {
              Device::Tcp(address) => Some(db::to_db_address(address.ip())),
              Device::Rtu { .. } => None,
            },
            match &device_match.destination.device {
              Device::Tcp(_) => None,
              Device::Rtu { path, .. } => Some(path.clone()),
            },
            match &device_match.destination.device {
              Device::Tcp(_) => None,
              Device::Rtu { baud_rate, .. } => Some(*baud_rate as i32),
            },
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
            address: match &device_match.destination.device {
              Device::Tcp(address) => Some(db::to_db_address(address.ip())),
              Device::Rtu { .. } => None,
            },
            path: match &device_match.destination.device {
              Device::Tcp(_) => None,
              Device::Rtu { path, .. } => Some(path.clone()),
            },
            baud_rate: match &device_match.destination.device {
              Device::Tcp(_) => None,
              Device::Rtu { baud_rate, .. } => Some(*baud_rate as i32),
            },
            slave: db::to_db_slave(device_match.destination.slave),
          })
          .await
        {
          tracing::error!("Failed inserting new device {}", error);

          return None;
        }
      }
    }

    self
      .services
      .modbus()
      .bind(device_match.id.clone(), device_match.destination.clone())
      .await;

    tracing::debug!("Matched device");

    Some(device_match)
  }

  async fn match_device(
    &self,
    device: config::Device,
    destination: modbus::Destination,
  ) -> Option<config::Device> {
    let registers = self
      .services
      .modbus()
      .read_from_destination(destination, device.detect.clone())
      .await
      .ok()?;

    let matches = registers
      .into_iter()
      .map(|register| register.matches())
      .collect::<Vec<_>>();

    matches
      .into_iter()
      .all(std::convert::identity)
      .then_some(device)
  }

  async fn match_id(
    &self,
    device: config::Device,
    destination: modbus::Destination,
  ) -> Option<DeviceMatch> {
    let matching_destination = destination.clone();
    let registers = self
      .services
      .modbus()
      .read_from_destination(matching_destination, device.id)
      .await;

    registers.ok().map(|id_registers| DeviceMatch {
      kind: device.kind.clone(),
      destination,
      id: modbus::make_id(device.kind, id_registers),
    })
  }
}

fn timeout_from_chrono(
  timeout: chrono::Duration,
) -> futures_time::time::Duration {
  futures_time::time::Duration::from_millis(timeout.num_milliseconds() as u64)
}
