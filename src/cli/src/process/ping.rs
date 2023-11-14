use futures::future::{join_all, try_join_all};

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
  async fn execute(&self) -> anyhow::Result<()> {
    let devices = self.services.db().get_devices().await?;

    let config = self.config.reload_async().await;

    try_join_all(
      join_all(devices.iter().cloned().map(move |device| {
        let config = config.clone();
        self.ping_device(config, device)
      }))
      .await
      .into_iter()
      .zip(devices)
      .map(|(healthy, device)| self.consolidate(device, healthy)),
    )
    .await?;

    Ok(())
  }
}

impl Process {
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
            modbus::make_id(device.kind, id_registers) == device.id
          }
          None => false,
        }
      }
      None => false,
    }
  }

  async fn consolidate(
    &self,
    device: db::Device,
    healthy: bool,
  ) -> anyhow::Result<()> {
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

    self
      .services
      .db()
      .update_device_status(&device.id, status, seen, now)
      .await?;

    if remove {
      self.services.modbus().stop_from_id(&device.id).await;
    }

    if update {
      self
        .services
        .db()
        .insert_health(db::Health {
          id: 0,
          source: device.id.clone(),
          timestamp: seen,
          status,
          data: serde_json::Value::Object(serde_json::Map::new()),
        })
        .await?;
    }

    Ok(())
  }
}
