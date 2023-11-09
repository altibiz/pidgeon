use thiserror::Error;

use crate::{
  cloud::{Client, ConstructionError, Measurement, Response},
  config::{self, Manager, ParseError},
  db::{Client, Error, Log, LogKind, Measurement},
  modbus::{self},
  modbus::{ModbusClient, ModbusClientError},
  network::{NetworkScanner, NetworkScannerError},
};

#[derive(Debug, Clone)]
pub struct Services {
  #[allow(unused)]
  config_manager: Manager,
  network_scanner: NetworkScanner,
  modbus_client: ModbusClient,
  db_client: Client,
  cloud_client: Client,
}

#[derive(Debug, Error)]
pub enum ServiceError {
  #[error("Config error")]
  ConfigManager(#[from] ParseError),

  #[error("Network scanner error")]
  NetworkScanner(#[from] NetworkScannerError),

  #[error("Modbus error")]
  ModbusClient(#[from] ModbusClientError),

  #[error("Db error")]
  DbClient(#[from] Error),

  #[error("Cloud error")]
  CloudClient(#[from] ConstructionError),
}

impl Services {
  pub async fn new(config_manager: Manager) -> Result<Self, ServiceError> {
    let mut config = config_manager.config_async().await?;

    let network_scanner = NetworkScanner::new(
      config.network.ip_range,
      std::time::Duration::from_millis(config.network.timeout),
    )?;

    let modbus_client = ModbusClient::new(
      config.modbus.timeout,
      config.modbus.retries,
      config.modbus.batching_threshold,
      config
        .modbus
        .devices
        .drain()
        .map(|(kind, mut device)| modbus::DeviceConfig {
          detect: match device.detect {
            config::DeviceDetect::One(register) => {
              vec![Self::to_modbus_detect_register(register)]
            }
            config::DeviceDetect::Many(mut registers) => registers
              .drain(0..)
              .map(Self::to_modbus_detect_register)
              .collect(),
          },
          id: match device.id {
            config::DeviceId::One(register) => {
              vec![Self::to_modbus_id_register(register)]
            }
            config::DeviceId::Many(mut registers) => registers
              .drain(0..)
              .map(Self::to_modbus_id_register)
              .collect(),
          },
          kind,
          measurement: device
            .measurement
            .drain(0..)
            .map(Self::to_modbus_measurement_register)
            .collect(),
        })
        .collect(),
    )?;

    let db_client = Client::new(
      config.db.timeout,
      config.db.ssl,
      config.db.domain,
      config.db.port,
      config.db.user,
      config.db.password,
      config.db.name,
    )?;

    let cloud_client = Client::new(
      config.cloud.domain,
      config.cloud.ssl,
      config.cloud.api_key,
      config.cloud.timeout,
      config.cloud.id,
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
      .drain(0..)
      .map(|device_data| Measurement {
        id: 0,
        source: device_data.id,
        timestamp: chrono::Utc::now(),
        data: modbus::register::serialize_registers(device_data.registers),
      })
      .collect::<Vec<Measurement>>();
    if measurements.is_empty() {
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
    let last_push_id =
      match measurements_to_push.iter().max_by(|x, y| x.id.cmp(&y.id)) {
        Some(measurement) => measurement.id,
        None => return Ok(()),
      };

    let result = self
      .cloud_client
      .push(
        measurements_to_push
          .drain(0..)
          .map(|measurement| Measurement {
            device_id: measurement.source,
            timestamp: measurement.timestamp,
            data: measurement.data.to_string(),
          })
          .collect(),
      )
      .await;

    let (log_kind, log_response) = match result {
      Ok(Response {
        success: true,
        text,
      }) => (LogKind::Success, text),
      Ok(Response {
        success: false,
        text,
      }) => (LogKind::Failure, text),
      Err(_) => (LogKind::Failure, "connection error".to_string()),
    };
    let log = Log {
      id: 0,
      timestamp: chrono::Utc::now(),
      last_measurement: last_push_id,
      kind: log_kind,
      response: serde_json::Value::String(log_response),
    };
    self.db_client.insert_log(log).await?;

    Ok(())
  }

  fn to_modbus_measurement_register(
    register: config::MeasurementRegister,
  ) -> modbus::register::MeasurementRegister<modbus::register::RegisterKind> {
    modbus::register::MeasurementRegister::<modbus::register::RegisterKind> {
      address: register.address,
      storage: Self::to_modbus_register_kind(register.kind),
      name: register.name,
    }
  }

  fn to_modbus_detect_register(
    register: config::DetectRegister,
  ) -> modbus::register::DetectRegister<modbus::register::RegisterKind> {
    modbus::register::DetectRegister::<modbus::register::RegisterKind> {
      address: register.address,
      storage: Self::to_modbus_register_kind(register.kind),
      r#match: match regex::Regex::new(register.r#match.as_str()) {
        Ok(regex) => either::Either::Right(regex),
        _ => either::Either::Left(register.r#match),
      },
    }
  }

  fn to_modbus_id_register(
    register: config::IdRegister,
  ) -> modbus::register::IdRegister<modbus::register::RegisterKind> {
    modbus::register::IdRegister::<modbus::register::RegisterKind> {
      address: register.address,
      storage: Self::to_modbus_register_kind(register.kind),
    }
  }

  fn to_modbus_register_kind(
    register: config::RegisterKind,
  ) -> modbus::register::RegisterKind {
    match register {
      config::RegisterKind::U16(config::NumericRegisterKind { multiplier }) => {
        modbus::register::RegisterKind::U16(
          modbus::register::NumericRegisterKind { multiplier },
        )
      }
      config::RegisterKind::U32(config::NumericRegisterKind { multiplier }) => {
        modbus::register::RegisterKind::U32(
          modbus::register::NumericRegisterKind { multiplier },
        )
      }
      config::RegisterKind::U64(config::NumericRegisterKind { multiplier }) => {
        modbus::register::RegisterKind::U64(
          modbus::register::NumericRegisterKind { multiplier },
        )
      }
      config::RegisterKind::S16(config::NumericRegisterKind { multiplier }) => {
        modbus::register::RegisterKind::S16(
          modbus::register::NumericRegisterKind { multiplier },
        )
      }
      config::RegisterKind::S32(config::NumericRegisterKind { multiplier }) => {
        modbus::register::RegisterKind::S32(
          modbus::register::NumericRegisterKind { multiplier },
        )
      }
      config::RegisterKind::S64(config::NumericRegisterKind { multiplier }) => {
        modbus::register::RegisterKind::S64(
          modbus::register::NumericRegisterKind { multiplier },
        )
      }
      config::RegisterKind::F32(config::NumericRegisterKind { multiplier }) => {
        modbus::register::RegisterKind::F32(
          modbus::register::NumericRegisterKind { multiplier },
        )
      }
      config::RegisterKind::F64(config::NumericRegisterKind { multiplier }) => {
        modbus::register::RegisterKind::F64(
          modbus::register::NumericRegisterKind { multiplier },
        )
      }
      config::RegisterKind::String(config::StringRegisterKind { length }) => {
        modbus::register::RegisterKind::String(
          modbus::register::StringRegisterKind { length },
        )
      }
    }
  }
}
