use futures::future::join_all;

use modbus::Time;

#[allow(unused_imports, reason = "services")]
use crate::{service::*, *};

pub(crate) struct Process {
  #[allow(dead_code, reason = "process")]
  config: config::Manager,

  #[allow(dead_code, reason = "process")]
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
impl process::Recurring for Process {
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let config = self.config.values().await;
    let timeout = config.modbus.time_timeout;

    let db_devices = self.services.db().get_devices().await?;

    join_all(
      db_devices
        .into_iter()
        .filter_map(|device| {
          config
            .modbus
            .devices
            .values()
            .filter(|device_config| device_config.time.is_some())
            .find(|device_config| device_config.kind == device.kind)
            .map(|config| Device {
              id: device.id,
              #[allow(clippy::unwrap_used, reason = "filtered by is_some")]
              time: config.time.unwrap(),
            })
        })
        .map(|device| async move {
          match self.write_to_device(&device, timeout).await {
            Err(error) => {
              tracing::error! {
                %error,
                "Failed writing nightly to device {}",
                &device.id
              }
            }
            Ok(_) => {
              tracing::info! {
                "Wrote nightly to device {}",
                &device.id
              }
            }
          }
        }),
    )
    .await;

    Ok(())
  }
}

#[derive(Debug, thiserror::Error)]
enum TimeWriteError {
  #[error("Failed writing to device")]
  DeviceWrite(#[from] modbus::DeviceWriteError),

  #[error("Writing to device timed out")]
  Timeout(#[from] std::io::Error),
}

impl Process {
  async fn write_to_device(
    &self,
    device: &Device,
    timeout: chrono::Duration,
  ) -> Result<(), TimeWriteError> {
    self
      .services
      .modbus()
      .write_to_id(
        &device.id,
        [Box::new(
          modbus::time::implementation_for(device.time).create(),
        )],
      )
      .timeout(timeout_from_chrono(timeout))
      .await??;

    Ok(())
  }
}

#[derive(Clone, Debug)]
struct Device {
  id: String,
  time: modbus::TimeImplementation,
}

fn timeout_from_chrono(
  timeout: chrono::Duration,
) -> futures_time::time::Duration {
  futures_time::time::Duration::from_millis(timeout.num_milliseconds() as u64)
}
