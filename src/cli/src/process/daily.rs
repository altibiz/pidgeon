use futures::future::join_all;

use crate::{service::*, *};

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
impl process::Recurring for Process {
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let config = self.config.values().await;
    let timeout = config.modbus.tariff_timeout;

    let db_devices = self.services.db().get_devices().await?;

    join_all(
      db_devices
        .into_iter()
        .filter_map(|device| {
          config
            .modbus
            .devices
            .values()
            .find(|device_config| device_config.kind == device.kind)
            .map(|config| Device {
              id: device.id,
              configuration: config.configuration.clone(),
              daily: config.daily.clone(),
            })
        })
        .map(|device| async move {
          match self.write_to_device(&device, timeout).await {
            Err(error) => {
              tracing::error! {
                %error,
                "Failed writing daily to device {}",
                &device.id
              }
            }
            Ok(_) => {
              tracing::info! {
                "Wrote daily to device {}",
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
enum TariffWriteError {
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
  ) -> Result<(), TariffWriteError> {
    self
      .services
      .modbus()
      .write_to_id(&device.id, &device.configuration)
      .timeout(timeout_from_chrono(timeout))
      .await??;

    self
      .services
      .modbus()
      .write_to_id(&device.id, &device.daily)
      .timeout(timeout_from_chrono(timeout))
      .await??;

    Ok(())
  }
}

#[derive(Clone, Debug)]
struct Device {
  id: String,
  configuration: Vec<modbus::ValueRegister<modbus::RegisterValueStorage>>,
  daily: Vec<modbus::ValueRegister<modbus::RegisterValueStorage>>,
}

fn timeout_from_chrono(
  timeout: chrono::Duration,
) -> futures_time::time::Duration {
  futures_time::time::Duration::from_millis(timeout.num_milliseconds() as u64)
}
