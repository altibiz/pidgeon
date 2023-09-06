use thiserror::Error;

use crate::{
  cloud::{CloudClient, CloudClientError, CloudMeasurement, CloudResponse},
  config::{self, ConfigManager, ConfigManagerError},
  db::{DbClient, DbClientError, DbLog, DbLogKind, DbMeasurement},
  modbus::{self},
  modbus::{ModbusClient, ModbusClientError},
  network::{NetworkScanner, NetworkScannerError},
};

#[derive(Debug, Clone)]
pub struct Services {
  #[allow(unused)]
  config_manager: ConfigManager,
  network_scanner: NetworkScanner,
  modbus_client: ModbusClient,
  db_client: DbClient,
  cloud_client: CloudClient,
}

#[derive(Debug, Error)]
pub enum ServiceError {
  #[error("Config error")]
  ConfigManager(#[from] ConfigManagerError),

  #[error("Network scanner error")]
  NetworkScanner(#[from] NetworkScannerError),

  #[error("Modbus error")]
  ModbusClient(#[from] ModbusClientError),

  #[error("Db error")]
  DbClient(#[from] DbClientError),

  #[error("Cloud error")]
  CloudClient(#[from] CloudClientError),
}

impl Services {
  pub async fn new(
    config_manager: ConfigManager,
  ) -> Result<Self, ServiceError> {
    let mut config = config_manager.config_async().await?;

    let network_scanner = NetworkScanner::new(
      config.network.ip_range,
      std::time::Duration::from_millis(config.network.timeout),
    )?;

    let modbus_client = ModbusClient::new(
      config.modbus.timeout,
      config
        .modbus
        .devices
        .drain()
        .map(|(kind, mut device)| modbus::DeviceConfig {
          detect: match device.detect {
            config::DeviceDetect::One(register) => modbus::DeviceDetect::One(
              Self::to_modbus_detect_register(register),
            ),
            config::DeviceDetect::Many(mut registers) => {
              modbus::DeviceDetect::Many(
                registers
                  .drain(0..)
                  .map(Self::to_modbus_detect_register)
                  .collect(),
              )
            }
          },
          kind: Self::to_modbus_device(kind),
          registers: device
            .registers
            .drain(0..)
            .map(|register| modbus::RegisterConfig {
              name: register.name,
              address: register.address,
              kind: Self::to_modbus_register(register.kind),
            })
            .collect(),
        })
        .collect(),
    )?;

    let db_client = DbClient::new(
      config.db.timeout,
      config.db.ssl,
      config.db.domain,
      config.db.port,
      config.db.user,
      config.db.password,
      config.db.name,
    )?;

    let cloud_client = CloudClient::new(
      config.cloud.domain,
      config.cloud.ssl,
      config.cloud.api_key,
      config.cloud.timeout,
    )?;

    let services = Services {
      config_manager,
      network_scanner,
      modbus_client,
      db_client,
      cloud_client,
    };

    Ok(services)
  }

  #[tracing::instrument(skip(self))]
  pub async fn on_setup(&self) -> Result<(), ServiceError> {
    self.db_client.migrate().await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn on_scan(&self) -> Result<(), ServiceError> {
    let ips = self.network_scanner.scan().await;
    self.modbus_client.detect(ips).await?;
    self.modbus_client.clean().await;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn on_pull(&self) -> Result<(), ServiceError> {
    let mut device_data = self.modbus_client.read().await?;
    let measurements = device_data
      .drain(..0)
      .map(|device_data| DbMeasurement {
        id: 0,
        source: "todo".to_string(),
        timestamp: chrono::Utc::now(),
        data: modbus::registers_to_json(device_data.registers),
      })
      .collect::<Vec<DbMeasurement>>();
    if measurements.len() <= 0 {
      return Ok(());
    }

    self.db_client.insert_measurements(measurements).await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn on_push(&self) -> Result<(), ServiceError> {
    let last_pushed_id = match self.db_client.get_last_successful_log().await? {
      Some(log) => log.last_measurement,
      None => 0,
    };

    let mut measurements_to_push = self
      .db_client
      .get_measurements(last_pushed_id, 1000)
      .await?;
    let last_push_id = match measurements_to_push.last() {
      Some(measurement) => measurement.id,
      None => return Ok(()),
    };

    let result = self
      .cloud_client
      .push_measurements(
        measurements_to_push
          .drain(0..)
          .map(|measurement| CloudMeasurement {
            source: measurement.source,
            timestamp: measurement.timestamp,
            data: measurement.data,
          })
          .collect(),
      )
      .await;

    let (log_kind, log_response) = match result {
      Ok(CloudResponse {
        success: true,
        text,
      }) => (DbLogKind::Success, text),
      Ok(CloudResponse {
        success: false,
        text,
      }) => (DbLogKind::Failure, text),
      Err(_) => (DbLogKind::Failure, "".to_string()),
    };
    let log = DbLog {
      id: 0,
      timestamp: chrono::Utc::now(),
      last_measurement: last_push_id,
      kind: log_kind,
      response: serde_json::Value::String(log_response),
    };
    self.db_client.insert_log(log).await?;

    Ok(())
  }

  fn to_modbus_detect_register(
    register: config::DetectRegister,
  ) -> modbus::DetectRegister {
    modbus::DetectRegister {
      address: register.address,
      kind: Self::to_modbus_register(register.kind),
      r#match: match regex::Regex::new(register.r#match.as_str()) {
        Ok(regex) => either::Either::Right(regex),
        _ => either::Either::Left(register.r#match),
      },
    }
  }

  fn to_modbus_register(
    register: config::RegisterKind,
  ) -> modbus::RegisterKind {
    match register {
      config::RegisterKind::U16 => modbus::RegisterKind::U16,
      config::RegisterKind::U32 => modbus::RegisterKind::U32,
      config::RegisterKind::S16 => modbus::RegisterKind::S16,
      config::RegisterKind::S32 => modbus::RegisterKind::S32,
      config::RegisterKind::String(config::StringRegisterKind { length }) => {
        modbus::RegisterKind::String(modbus::StringRegisterKind { length })
      }
    }
  }

  fn to_modbus_device(device: config::DeviceKind) -> modbus::DeviceKind {
    match device {
      config::DeviceKind::Abb => modbus::DeviceKind::Abb,
    }
  }
}
