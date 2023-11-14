mod args;
mod env;
mod file;

use std::{collections::HashMap, sync::Arc};

use ipnet::IpAddrRange;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::service::modbus;

#[derive(Debug, Clone)]
pub(crate) struct Db {
  pub(crate) timeout: chrono::Duration,
  pub(crate) ssl: bool,
  pub(crate) domain: String,
  pub(crate) port: Option<u16>,
  pub(crate) user: String,
  pub(crate) password: Option<String>,
  pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Network {
  pub(crate) timeout: chrono::Duration,
  pub(crate) ip_range: IpAddrRange,
}

#[derive(Debug, Clone)]
pub(crate) struct Hardware {
  pub(crate) temperature_monitor: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Cloud {
  pub(crate) timeout: chrono::Duration,
  pub(crate) ssl: bool,
  pub(crate) domain: String,
  pub(crate) api_key: Option<String>,
  pub(crate) id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Device {
  pub(crate) kind: String,
  pub(crate) id: Vec<modbus::IdRegister<modbus::RegisterKind>>,
  pub(crate) detect: Vec<modbus::DetectRegister<modbus::RegisterKind>>,
  pub(crate) measurement:
    Vec<modbus::MeasurementRegister<modbus::RegisterKind>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Modbus {
  pub(crate) initial_timeout: chrono::Duration,
  pub(crate) initial_backoff: chrono::Duration,
  pub(crate) initial_retries: u32,
  pub(crate) batch_threshold: u32,
  pub(crate) termination_timeout: chrono::Duration,
  pub(crate) devices: HashMap<String, Device>,
}

#[derive(Debug, Clone)]
pub(crate) struct Values {
  pub(crate) cloud: Cloud,
  pub(crate) db: Db,
  pub(crate) network: Network,
  pub(crate) modbus: Modbus,
  pub(crate) hardware: Hardware,
  pub(crate) dev: bool,
  pub(crate) log_level: tracing::level_filters::LevelFilter,
  pub(crate) discover_interval: chrono::Duration,
  pub(crate) ping_interval: chrono::Duration,
  pub(crate) measure_interval: chrono::Duration,
  pub(crate) push_interval: chrono::Duration,
  pub(crate) update_interval: chrono::Duration,
  pub(crate) health_interval: chrono::Duration,
}

#[derive(Debug, Clone)]
struct Unparsed {
  from_args: args::Values,
  from_env: env::Values,
  from_file: file::Values,
}

#[derive(Debug, Clone)]
pub(crate) struct Manager {
  values: Arc<Mutex<Unparsed>>,
}

#[derive(Debug, Error)]
pub(crate) enum ReadError {
  #[error("Failed reading file")]
  FileReadError(#[from] file::ParseError),

  #[error("Failed reading env")]
  EnvReadError(#[from] env::ParseError),
}

#[derive(Debug, Error)]
pub(crate) enum ReloadError {
  #[error("Failed reading file")]
  FileReadError(#[from] file::ParseError),
}

impl Manager {
  pub(crate) fn new() -> Result<Self, ReadError> {
    let config = Self::read()?;

    let config_manager = Self {
      values: Arc::new(Mutex::new(config)),
    };

    Ok(config_manager)
  }

  pub(crate) async fn new_async() -> Result<Self, ReadError> {
    let config = Self::read_async().await?;

    let config_manager = Self {
      values: Arc::new(Mutex::new(config)),
    };

    Ok(config_manager)
  }

  pub(crate) fn values(&self) -> Values {
    let config = self.values.blocking_lock().clone();

    Self::parse(config)
  }

  pub(crate) async fn values_async(&self) -> Values {
    let config = self.values.lock().await.clone();

    Self::parse(config)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) fn reload(&self) -> Values {
    let config = {
      let mut values = self.values.blocking_lock();
      let from_file = file::parse(values.from_args.config.as_deref());
      match from_file {
        Ok(from_file) => values.from_file = from_file,
        Err(error) => {
          tracing::error!("Failed parsing config file {}", error)
        }
      }

      values.clone()
    };

    Self::parse(config)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn reload_async(&self) -> Values {
    let config = {
      let mut values = self.values.lock().await;
      let from_file =
        file::parse_async(values.from_args.config.as_deref()).await;
      match from_file {
        Ok(from_file) => values.from_file = from_file,
        Err(error) => {
          tracing::error!("Failed parsing config file {}", error)
        }
      }
      values.clone()
    };

    Self::parse(config)
  }

  fn parse(config: Unparsed) -> Values {
    Values {
      dev: config.from_args.dev,
      log_level: config.from_file.log_level.map_or_else(
        || {
          if config.from_args.dev {
            tracing::level_filters::LevelFilter::DEBUG
          } else {
            tracing::level_filters::LevelFilter::INFO
          }
        },
        |log_level| match log_level {
          file::LogLevel::Trace => tracing::level_filters::LevelFilter::TRACE,
          file::LogLevel::Debug => tracing::level_filters::LevelFilter::DEBUG,
          file::LogLevel::Info => tracing::level_filters::LevelFilter::INFO,
          file::LogLevel::Warn => tracing::level_filters::LevelFilter::WARN,
          file::LogLevel::Error => tracing::level_filters::LevelFilter::ERROR,
        },
      ),
      discover_interval: file::milliseconds_to_chrono(
        config.from_file.discover_interval.unwrap_or(60000),
      ),
      ping_interval: file::milliseconds_to_chrono(
        config.from_file.ping_interval.unwrap_or(60000),
      ),
      measure_interval: file::milliseconds_to_chrono(
        config.from_file.ping_interval.unwrap_or(60000),
      ),
      push_interval: file::milliseconds_to_chrono(
        config.from_file.push_interval.unwrap_or(60000),
      ),
      update_interval: file::milliseconds_to_chrono(
        config.from_file.update_interval.unwrap_or(60000),
      ),
      health_interval: file::milliseconds_to_chrono(
        config.from_file.health_interval.unwrap_or(60000),
      ),
      hardware: Hardware {
        temperature_monitor: config.from_file.hardware.temperature_monitor,
      },
      cloud: Cloud {
        timeout: file::milliseconds_to_chrono(
          config.from_file.cloud.timeout.unwrap_or(30000),
        ),
        ssl: config.from_env.cloud.ssl,
        domain: config.from_env.cloud.domain,
        api_key: config.from_env.cloud.api_key,
        id: config.from_env.cloud.id,
      },
      db: Db {
        timeout: file::milliseconds_to_chrono(
          config.from_file.db.timeout.unwrap_or(30000),
        ),
        ssl: config.from_env.db.ssl,
        domain: config.from_env.db.domain,
        port: config
          .from_env
          .db
          .port
          .and_then(|port| port.parse::<u16>().ok()),
        user: config.from_env.db.user,
        password: config.from_env.db.password,
        name: config.from_env.db.name,
      },
      network: Network {
        timeout: file::milliseconds_to_chrono(
          config.from_file.network.timeout.unwrap_or(30000),
        ),
        ip_range: file::make_ip_range(
          config.from_env.network.ip_range_start,
          config.from_env.network.ip_range_end,
        ),
      },
      modbus: Modbus {
        initial_timeout: file::milliseconds_to_chrono(
          config.from_file.modbus.initial_timeout,
        ),
        initial_backoff: file::milliseconds_to_chrono(
          config.from_file.modbus.initial_backoff,
        ),
        initial_retries: config.from_file.modbus.initial_retries,
        batch_threshold: config.from_file.modbus.batch_threshold,
        termination_timeout: file::milliseconds_to_chrono(
          config.from_file.modbus.termination_timeout,
        ),
        devices: config
          .from_file
          .modbus
          .devices
          .into_iter()
          .map(|(kind, device)| {
            (
              kind.clone(),
              Device {
                kind,
                id: device
                  .id
                  .normalize()
                  .into_iter()
                  .map(file::to_modbus_id_register)
                  .collect(),
                detect: device
                  .detect
                  .normalize()
                  .into_iter()
                  .map(file::to_modbus_detect_register)
                  .collect(),
                measurement: device
                  .measurement
                  .into_iter()
                  .map(file::to_modbus_measurement_register)
                  .collect(),
              },
            )
          })
          .collect::<HashMap<_, _>>(),
      },
    }
  }

  fn read() -> Result<Unparsed, ReadError> {
    let from_args = args::parse();
    let from_env = env::parse()?;
    let from_file = file::parse(from_args.config.as_deref())?;

    Ok(Unparsed {
      from_args,
      from_env,
      from_file,
    })
  }

  async fn read_async() -> Result<Unparsed, ReadError> {
    let from_args = args::parse();
    let from_env = env::parse()?;
    let from_file = file::parse_async(from_args.config.as_deref()).await?;

    Ok(Unparsed {
      from_args,
      from_env,
      from_file,
    })
  }
}