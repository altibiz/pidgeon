use futures::future::join_all;
use futures_time::future::FutureExt;

use crate::{service::*, *};

pub(crate) struct Process {
  #[allow(unused)]
  config: config::Manager,

  #[allow(unused)]
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

    let devices = self.services.db().get_devices().await?;

    let pinged_devices = join_all(
      devices
        .iter()
        .cloned()
        .map(|device| self.ping_device(&config, device)),
    )
    .await;

    tracing::info!(
      "Pinged {:?} devices of which {:?} are healthy and {:?} unreachable",
      pinged_devices.len(),
      pinged_devices.iter().filter(|pinged| **pinged).count(),
      pinged_devices.iter().filter(|pinged| !**pinged).count(),
    );

    let consolidated_devices = join_all(
      pinged_devices
        .into_iter()
        .zip(devices)
        .map(|(pinged, device)| self.consolidate(&config, device, pinged)),
    )
    .await;

    tracing::info!(
      "Consolidated {:?} pinged devices of which {:?} are healthy, {:?} unreachable, {:?} inactive, and {:?} failed",
      consolidated_devices.len(),
      consolidated_devices.iter().filter(|consolidated| matches!(**consolidated, Ok((_, db::DeviceStatus::Healthy)))).count(),
      consolidated_devices.iter().filter(|consolidated| matches!(**consolidated, Ok((_, db::DeviceStatus::Unreachable)))).count(),
      consolidated_devices.iter().filter(|consolidated| matches!(**consolidated, Ok((_, db::DeviceStatus::Inactive)))).count(),
      consolidated_devices.iter().filter(|consolidated| matches!(**consolidated, Err(_))).count(),
    );

    Ok(())
  }
}

impl Process {
  #[tracing::instrument(skip(self, config))]
  async fn ping_device(
    &self,
    config: &config::Values,
    device: db::Device,
  ) -> bool {
    match config
      .modbus
      .devices
      .values()
      .find(|device_config| device_config.kind == device.kind)
    {
      Some(device_config) => {
        match self
          .services
          .modbus()
          .read_from_id(&device.id, device_config.id.clone())
          .timeout(timeout_from_chrono(config.modbus.ping_timeout))
          .await
        {
          Err(error) => {
            tracing::warn!("Getting id timed out {}", error);
            return false;
          }
          Ok(Err(error)) => {
            tracing::warn!("Getting id failed {}", error);
            return false;
          }
          Ok(Ok(id_registers)) => {
            if modbus::make_id(device.kind, id_registers) == device.id {
              tracing::debug!("Id match");
            } else {
              tracing::debug!("Id mismatch");
              return false;
            }
          }
        }
      }
      None => {
        tracing::debug!("Config not found");
        return false;
      }
    }

    true
  }

  #[tracing::instrument(skip(self))]
  async fn consolidate(
    &self,
    config: &config::Values,
    device: db::Device,
    pinged: bool,
  ) -> anyhow::Result<(db::Device, db::DeviceStatus)> {
    let now = chrono::Utc::now();
    let status = if pinged {
      db::DeviceStatus::Healthy
    } else {
      if now - device.seen > config.modbus.inactive_timeout {
        db::DeviceStatus::Inactive
      } else {
        db::DeviceStatus::Unreachable
      }
    };
    let seen = if pinged { now } else { device.seen };
    let update = device.status != status;
    let remove = (status == db::DeviceStatus::Inactive)
      && (device.status != db::DeviceStatus::Inactive);

    if let Err(error) = self
      .services
      .db()
      .update_device_status(&device.id, status, seen, now)
      .await
    {
      tracing::error!("Failed updating device status {}", error);
      return Err(error.into());
    };

    if remove {
      self.services.modbus().stop_from_id(&device.id).await;
      tracing::debug!("Stopped worker")
    }

    if update {
      if let Err(error) = self
        .services
        .db()
        .insert_health(db::Health {
          id: 0,
          source: device.id.clone(),
          timestamp: seen,
          status,
          data: serde_json::Value::Object(serde_json::Map::new()),
        })
        .await
      {
        tracing::error!("Failed inserting health {}", error);
      }
    }

    tracing::debug!("Updated device status and health");

    Ok((device, status))
  }
}

fn timeout_from_chrono(
  timeout: chrono::Duration,
) -> futures_time::time::Duration {
  futures_time::time::Duration::from_millis(timeout.num_milliseconds() as u64)
}
