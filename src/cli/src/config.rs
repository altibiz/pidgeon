use clap::{command, Parser};
use ipnet::{IpAddrRange, Ipv4AddrRange};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::service::modbus;

// TODO: make unparsed private, parsed to just normal name without parsed prefix

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum Plural<T> {
  One(T),
  Many(Vec<T>),
}

impl<T: Clone> Plural<T> {
  fn normalize(&self) -> Vec<T> {
    match self {
      Plural::One(item) => vec![item.clone()],
      Plural::Many(items) => items.clone(),
    }
  }
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct UnparsedArgs {
  /// Run in development mode
  #[arg(short, long)]
  dev: bool,

  /// Alternative configuration location
  #[arg(short, long)]
  config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedHardwareFile {
  temperature_monitor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedNetworkFile {
  timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedDbFile {
  timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedMeasurementRegister {
  pub name: String,
  pub address: u16,
  pub kind: UnparsedRegisterKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct UnparsedStringRegisterKind {
  pub length: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct UnparsedNumericRegisterKind {
  pub multiplier: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum UnparsedRegisterKind {
  U16(UnparsedNumericRegisterKind),
  U32(UnparsedNumericRegisterKind),
  U64(UnparsedNumericRegisterKind),
  S16(UnparsedNumericRegisterKind),
  S32(UnparsedNumericRegisterKind),
  S64(UnparsedNumericRegisterKind),
  F32(UnparsedNumericRegisterKind),
  F64(UnparsedNumericRegisterKind),
  String(UnparsedStringRegisterKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedDetectRegister {
  pub address: u16,
  pub kind: UnparsedRegisterKind,
  pub r#match: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedIdRegister {
  pub address: u16,
  pub kind: UnparsedRegisterKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedDevice {
  pub detect: Plural<UnparsedDetectRegister>,
  pub id: Plural<UnparsedIdRegister>,
  pub measurement: Vec<UnparsedMeasurementRegister>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedModbusFile {
  initial_timeout: u64,
  initial_backoff: u64,
  initial_retries: u64,
  batch_threshold: usize,
  devices: HashMap<String, UnparsedDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedCloudFile {
  timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnparsedFile {
  log_level: Option<LogLevel>,
  discover_interval: Option<u64>,
  ping_interval: Option<u64>,
  measure_interval: Option<u64>,
  push_interval: Option<u64>,
  update_interval: Option<u64>,
  hardware: UnparsedHardwareFile,
  network: UnparsedNetworkFile,
  modbus: UnparsedModbusFile,
  cloud: UnparsedCloudFile,
  db: UnparsedDbFile,
}

#[derive(Debug, Clone)]
struct UnparsedCloudEnv {
  ssl: bool,
  domain: String,
  api_key: Option<String>,
  id: Option<String>,
}

#[derive(Debug, Clone)]
struct UnparsedDbEnv {
  ssl: bool,
  domain: String,
  port: Option<String>,
  user: String,
  password: Option<String>,
  name: String,
}

#[derive(Debug, Clone)]
struct UnparsedNetworkEnv {
  ip_range_start: String,
  ip_range_end: String,
}

#[derive(Debug, Clone)]
struct UnparsedEnv {
  cloud: UnparsedCloudEnv,
  db: UnparsedDbEnv,
  network: UnparsedNetworkEnv,
}

#[derive(Debug, Clone)]
struct Unparsed {
  from_args: UnparsedArgs,
  from_file: UnparsedFile,
  from_env: UnparsedEnv,
}

#[derive(Debug, Clone)]
pub struct Db {
  pub timeout: u64,
  pub ssl: bool,
  pub domain: String,
  pub port: Option<u16>,
  pub user: String,
  pub password: Option<String>,
  pub name: String,
}

#[derive(Debug, Clone)]
pub struct Network {
  pub timeout: u64,
  pub ip_range: IpAddrRange,
}

#[derive(Debug, Clone)]
pub struct Hardware {
  pub temperature_monitor: String,
}

#[derive(Debug, Clone)]
pub struct Cloud {
  pub timeout: u64,
  pub ssl: bool,
  pub domain: String,
  pub api_key: Option<String>,
  pub id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Device {
  pub kind: String,
  pub id: Vec<modbus::IdRegister<modbus::RegisterKind>>,
  pub detect: Vec<modbus::DetectRegister<modbus::RegisterKind>>,
  pub measurement: Vec<modbus::MeasurementRegister<modbus::RegisterKind>>,
}

#[derive(Debug, Clone)]
pub struct Modbus {
  pub initial_timeout: u64,
  pub initial_backoff: u64,
  pub initial_retries: u64,
  pub batching_threshold: usize,
  pub devices: HashMap<String, Device>,
}

#[derive(Debug, Clone)]
pub struct Values {
  pub cloud: Cloud,
  pub db: Db,
  pub network: Network,
  pub modbus: Modbus,
  pub hardware: Hardware,
  pub dev: bool,
  pub log_level: LogLevel,
  pub discover_interval: u64,
  pub ping_interval: u64,
  pub measure_interval: u64,
  pub push_interval: u64,
  pub update_interval: u64,
}

#[derive(Debug, Clone)]
pub struct Manager {
  values: Arc<Mutex<Unparsed>>,
}

#[derive(Debug, Error)]
pub enum ParseError {
  #[error("Failed parsing port")]
  PortParse(#[from] std::num::ParseIntError),

  #[error("Failed parsing ip range")]
  IpRangeParse,
}

#[derive(Debug, Error)]
pub enum ReadError {
  #[error("Failed creating project directories")]
  MissingProjectDirs,

  #[error("Failed reading config file")]
  FileConfigRead(#[from] std::io::Error),

  #[error("Failed serializing config from file")]
  FileConfigDeserialization(#[from] serde_yaml::Error),

  #[error("Failed reading environment variable")]
  Var(#[from] std::env::VarError),
}

#[derive(Debug, Error)]
pub enum RealoadError {
  #[error("Failed reading config")]
  ReadError(#[from] ReadError),

  #[error("Failed parsing config")]
  ParseError(#[from] ParseError),
}

impl Manager {
  pub fn new() -> Result<Self, ReadError> {
    let _ = dotenv::dotenv();

    let config = Self::read()?;

    let config_manager = Self {
      values: Arc::new(Mutex::new(config)),
    };

    Ok(config_manager)
  }

  #[allow(unused)]
  pub fn config(&self) -> Result<Values, ParseError> {
    let config = self.values.blocking_lock().clone();
    let parsed = Self::parse_config(config)?;
    Ok(parsed)
  }

  #[allow(unused)]
  pub async fn config_async(&self) -> Result<Values, ParseError> {
    let config = self.values.lock().await.clone();
    let parsed = Self::parse_config(config)?;
    Ok(parsed)
  }

  #[allow(unused)]
  pub fn reload(&self) -> Result<Values, RealoadError> {
    let config = {
      let mut values = self.values.blocking_lock();
      let from_file = Self::read_from_file(values.from_args.config.clone())?;
      values.from_file = from_file;
      values.clone()
    };

    let parsed = Self::parse_config(config)?;

    Ok(parsed)
  }

  #[allow(unused)]
  pub async fn reload_async(&self) -> Result<Values, RealoadError> {
    let config = {
      let mut values = self.values.lock().await;
      let from_file =
        Self::read_from_file_async(values.from_args.config.clone()).await?;
      values.from_file = from_file;
      values.clone()
    };

    let parsed = Self::parse_config(config)?;

    Ok(parsed)
  }

  fn parse_config(config: Unparsed) -> Result<Values, ParseError> {
    let parsed = Values {
      dev: config.from_args.dev,
      log_level: config.from_file.log_level.unwrap_or(
        if config.from_args.dev {
          LogLevel::Debug
        } else {
          LogLevel::Info
        },
      ),
      discover_interval: config.from_file.discover_interval.unwrap_or(60000),
      ping_interval: config.from_file.ping_interval.unwrap_or(60000),
      measure_interval: config.from_file.ping_interval.unwrap_or(60000),
      push_interval: config.from_file.push_interval.unwrap_or(60000),
      update_interval: config.from_file.update_interval.unwrap_or(60000),
      hardware: Hardware {
        temperature_monitor: config.from_file.hardware.temperature_monitor,
      },
      cloud: Cloud {
        timeout: config.from_file.cloud.timeout.unwrap_or(30000),
        ssl: config.from_env.cloud.ssl,
        domain: config.from_env.cloud.domain,
        api_key: config.from_env.cloud.api_key,
        id: config.from_env.cloud.id,
      },
      db: Db {
        timeout: config.from_file.db.timeout.unwrap_or(30000),
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
        timeout: config.from_file.network.timeout.unwrap_or(30000),
        ip_range: Self::make_ip_range(
          config.from_env.network.ip_range_start,
          config.from_env.network.ip_range_end,
        )?,
      },
      modbus: Modbus {
        initial_timeout: config.from_file.modbus.initial_timeout,
        initial_backoff: config.from_file.modbus.initial_backoff,
        initial_retries: config.from_file.modbus.initial_retries,
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
                  .map(Self::to_modbus_id_register)
                  .collect(),
                detect: device
                  .detect
                  .normalize()
                  .into_iter()
                  .map(Self::to_modbus_detect_register)
                  .collect(),
                measurement: device
                  .measurement
                  .into_iter()
                  .map(Self::to_modbus_measurement_register)
                  .collect(),
              },
            )
          })
          .collect::<HashMap<_, _>>(),
        batching_threshold: config.from_file.modbus.batch_threshold,
      },
    };

    Ok(parsed)
  }

  fn make_ip_range(
    start: String,
    end: String,
  ) -> Result<IpAddrRange, ParseError> {
    let (start, end) = match (start.parse(), end.parse()) {
      (Ok(start), Ok(end)) => (start, end),
      _ => match ("192.168.1.0".parse(), "192.168.1.255".parse()) {
        (Ok(start), Ok(end)) => (start, end),
        _ => return Err(ParseError::IpRangeParse),
      },
    };

    Ok(IpAddrRange::from(Ipv4AddrRange::new(start, end)))
  }

  fn to_modbus_measurement_register(
    register: UnparsedMeasurementRegister,
  ) -> modbus::MeasurementRegister<modbus::RegisterKind> {
    modbus::MeasurementRegister::<modbus::RegisterKind> {
      address: register.address,
      storage: Self::to_modbus_register_kind(register.kind),
      name: register.name,
    }
  }

  fn to_modbus_detect_register(
    register: UnparsedDetectRegister,
  ) -> modbus::DetectRegister<modbus::RegisterKind> {
    modbus::DetectRegister::<modbus::RegisterKind> {
      address: register.address,
      storage: Self::to_modbus_register_kind(register.kind),
      r#match: match regex::Regex::new(register.r#match.as_str()) {
        Ok(regex) => either::Either::Right(regex),
        _ => either::Either::Left(register.r#match),
      },
    }
  }

  fn to_modbus_id_register(
    register: UnparsedIdRegister,
  ) -> modbus::IdRegister<modbus::RegisterKind> {
    modbus::IdRegister::<modbus::RegisterKind> {
      address: register.address,
      storage: Self::to_modbus_register_kind(register.kind),
    }
  }

  fn to_modbus_register_kind(
    register: UnparsedRegisterKind,
  ) -> modbus::RegisterKind {
    match register {
      UnparsedRegisterKind::U16(UnparsedNumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::U16(modbus::NumericRegisterKind { multiplier })
      }
      UnparsedRegisterKind::U32(UnparsedNumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::U32(modbus::NumericRegisterKind { multiplier })
      }
      UnparsedRegisterKind::U64(UnparsedNumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::U64(modbus::NumericRegisterKind { multiplier })
      }
      UnparsedRegisterKind::S16(UnparsedNumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::S16(modbus::NumericRegisterKind { multiplier })
      }
      UnparsedRegisterKind::S32(UnparsedNumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::S32(modbus::NumericRegisterKind { multiplier })
      }
      UnparsedRegisterKind::S64(UnparsedNumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::S64(modbus::NumericRegisterKind { multiplier })
      }
      UnparsedRegisterKind::F32(UnparsedNumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::F32(modbus::NumericRegisterKind { multiplier })
      }
      UnparsedRegisterKind::F64(UnparsedNumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::F64(modbus::NumericRegisterKind { multiplier })
      }
      UnparsedRegisterKind::String(UnparsedStringRegisterKind { length }) => {
        modbus::RegisterKind::String(modbus::StringRegisterKind { length })
      }
    }
  }

  fn read() -> Result<Unparsed, ReadError> {
    let from_args = Self::read_from_args();
    let from_file = Self::read_from_file(from_args.config.clone())?;
    let from_env = Self::read_from_env()?;

    let config = Unparsed {
      from_args,
      from_file,
      from_env,
    };

    Ok(config)
  }

  fn read_from_file(
    location: Option<String>,
  ) -> Result<UnparsedFile, ReadError> {
    let location = match location {
      Some(location) => std::path::PathBuf::from(location),
      None => match directories::ProjectDirs::from("com", "altibiz", "pidgeon")
      {
        Some(project_dirs) => project_dirs.config_dir().join("config.yaml"),
        None => return Err(ReadError::MissingProjectDirs),
      },
    };
    let from_file = {
      let raw = fs::read_to_string(location)?;
      serde_yaml::from_str::<UnparsedFile>(raw.as_str())?
    };

    Ok(from_file)
  }

  async fn read_from_file_async(
    location: Option<String>,
  ) -> Result<UnparsedFile, ReadError> {
    let location = match location {
      Some(location) => std::path::PathBuf::from(location),
      None => match directories::ProjectDirs::from("com", "altibiz", "pidgeon")
      {
        Some(project_dirs) => project_dirs.config_dir().join("config.yaml"),
        None => return Err(ReadError::MissingProjectDirs),
      },
    };
    let from_file = {
      let raw = tokio::fs::read_to_string(location).await?;
      serde_yaml::from_str::<UnparsedFile>(raw.as_str())?
    };

    Ok(from_file)
  }

  fn read_from_env() -> Result<UnparsedEnv, env::VarError> {
    let from_env = UnparsedEnv {
      cloud: UnparsedCloudEnv {
        ssl: env::var("PIDGEON_CLOUD_SSL").map_or_else(|_| false, |_| true),
        domain: env::var("PIDGEON_CLOUD_DOMAIN")?,
        api_key: env::var("PIDGEON_CLOUD_API_KEY").ok(),
        id: env::var("PIDGEON_CLOUD_ID").ok(),
      },
      db: UnparsedDbEnv {
        ssl: env::var("PIDGEON_DB_SSL").map_or_else(|_| false, |_| true),
        domain: env::var("PIDGEON_DB_DOMAIN")?,
        port: env::var("PIDGEON_DB_PORT").ok(),
        user: env::var("PIDGEON_DB_USER")?,
        password: env::var("PIDGEON_DB_PASSWORD").ok(),
        name: env::var("PIDGEON_DB_NAME")?,
      },
      network: UnparsedNetworkEnv {
        ip_range_start: env::var("PIDGEON_NETWORK_IP_RANGE_START")?,
        ip_range_end: env::var("PIDGEON_NETWORK_IP_RANGE_END")?,
      },
    };

    Ok(from_env)
  }

  fn read_from_args() -> UnparsedArgs {
    UnparsedArgs::parse()
  }
}
