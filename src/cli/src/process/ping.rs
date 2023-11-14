use futures::future::join_all;

use crate::{service::*, *};

// TODO: add timeout
// TODO: add max unreachable till inactive

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

    let devices = self.services.db().get_devices().await?;

    let pinged_devices = join_all(devices.iter().cloned().map(move |device| {
      let config = config.clone();
      self.ping_device(config, device)
    }))
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
        .map(|(healthy, device)| self.consolidate(device, healthy)),
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
    config: config::Values,
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
          .await
          .ok()
        {
          Some(id_registers) => {
            if modbus::make_id(device.kind, id_registers) == device.id {
              tracing::debug!("Id match");
              true
            } else {
              tracing::debug!("Id mismatch");
              false
            }
          }
          None => {
            tracing::debug!("Id read failed");
            false
          }
        }
      }
      None => {
        tracing::debug!("Config not found");
        false
      }
    }
  }

  #[tracing::instrument(skip(self))]
  async fn consolidate(
    &self,
    device: db::Device,
    healthy: bool,
  ) -> anyhow::Result<(db::Device, db::DeviceStatus)> {
    let now = chrono::Utc::now();
    let status = if healthy {
      db::DeviceStatus::Healthy
    } else {
      db::DeviceStatus::Unreachable
    };
    let seen = if healthy { now } else { device.seen };
    let update = (healthy && (device.status != db::DeviceStatus::Healthy))
      || (!healthy && (device.status == db::DeviceStatus::Healthy));
    let remove = (status == db::DeviceStatus::Inactive)
      && (device.status != db::DeviceStatus::Unreachable);

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
