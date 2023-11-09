use clap::{command, Parser};
use ipnet::{IpAddrRange, Ipv4AddrRange};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::service::modbus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Plural<T> {
  One(T),
  Many(Vec<T>),
}

impl<T> Plural<T> {
  pub fn normalize(&self) -> Vec<T> {
    match self {
      Plural::One(item) => vec![*item],
      Plural::Many(items) => *items,
    }
  }
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct FromArgs {
  /// Run in development mode
  #[arg(short, long)]
  dev: bool,

  /// Alternative configuration location
  #[arg(short, long)]
  config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkFile {
  timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DbFile {
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
struct RuntimeFile {
  log_level: Option<LogLevel>,
  scan_interval: Option<u64>,
  pull_interval: Option<u64>,
  push_interval: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementRegister {
  pub name: String,
  pub address: u16,
  pub kind: RegisterKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StringRegisterKind {
  pub length: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NumericRegisterKind {
  pub multiplier: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegisterKind {
  U16(NumericRegisterKind),
  U32(NumericRegisterKind),
  U64(NumericRegisterKind),
  S16(NumericRegisterKind),
  S32(NumericRegisterKind),
  S64(NumericRegisterKind),
  F32(NumericRegisterKind),
  F64(NumericRegisterKind),
  String(StringRegisterKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectRegister {
  pub address: u16,
  pub kind: RegisterKind,
  pub r#match: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdRegister {
  pub address: u16,
  pub kind: RegisterKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
  pub detect: Plural<DetectRegister>,
  pub id: Plural<IdRegister>,
  pub measurement: Vec<MeasurementRegister>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModbusFile {
  timeout: u64,
  retries: u64,
  batch_threshold: usize,
  devices: HashMap<String, Device>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloudFile {
  timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FromFile {
  runtime: RuntimeFile,
  network: NetworkFile,
  modbus: ModbusFile,
  cloud: CloudFile,
  db: DbFile,
}

#[derive(Debug, Clone)]
struct CloudEnv {
  ssl: bool,
  domain: String,
  api_key: Option<String>,
  id: Option<String>,
}

#[derive(Debug, Clone)]
struct DbEnv {
  ssl: bool,
  domain: String,
  port: Option<String>,
  user: String,
  password: Option<String>,
  name: String,
}

#[derive(Debug, Clone)]
struct NetworkEnv {
  ip_range_start: String,
  ip_range_end: String,
}

#[derive(Debug, Clone)]
struct FromEnv {
  cloud: CloudEnv,
  db: DbEnv,
  network: NetworkEnv,
}

#[derive(Debug, Clone)]
struct Unparsed {
  from_args: FromArgs,
  from_file: FromFile,
  from_env: FromEnv,
}

#[derive(Debug, Clone)]
pub struct ParsedRuntime {
  pub dev: bool,
  pub log_level: LogLevel,
  pub scan_interval: u64,
  pub pull_interval: u64,
  pub push_interval: u64,
}

#[derive(Debug, Clone)]
pub struct ParsedDb {
  pub timeout: u64,
  pub ssl: bool,
  pub domain: String,
  pub port: Option<u16>,
  pub user: String,
  pub password: Option<String>,
  pub name: String,
}

#[derive(Debug, Clone)]
pub struct ParsedNetwork {
  pub timeout: u64,
  pub ip_range: IpAddrRange,
}

#[derive(Debug, Clone)]
pub struct ParsedCloud {
  pub timeout: u64,
  pub ssl: bool,
  pub domain: String,
  pub api_key: Option<String>,
  pub id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedDevice {
  pub kind: String,
  pub id: Vec<modbus::IdRegister<modbus::RegisterKind>>,
  pub detect: Vec<modbus::DetectRegister<modbus::RegisterKind>>,
  pub measurement: Vec<modbus::MeasurementRegister<modbus::RegisterKind>>,
}

#[derive(Debug, Clone)]
pub struct ParsedModbus {
  pub timeout: u64,
  pub retries: u64,
  pub batching_threshold: usize,
  pub devices: HashMap<String, ParsedDevice>,
}

#[derive(Debug, Clone)]
pub struct Parsed {
  pub cloud: ParsedCloud,
  pub db: ParsedDb,
  pub network: ParsedNetwork,
  pub modbus: ParsedModbus,
  pub runtime: ParsedRuntime,
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
  pub fn config(&self) -> Result<Parsed, ParseError> {
    let config = self.values.blocking_lock().clone();
    let parsed = Self::parse_config(config)?;
    Ok(parsed)
  }

  #[allow(unused)]
  pub async fn config_async(&self) -> Result<Parsed, ParseError> {
    let config = self.values.lock().await.clone();
    let parsed = Self::parse_config(config)?;
    Ok(parsed)
  }

  #[allow(unused)]
  pub fn reload(&self) -> Result<Parsed, RealoadError> {
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
  pub async fn reload_async(&self) -> Result<Parsed, RealoadError> {
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

  fn parse_config(config: Unparsed) -> Result<Parsed, ParseError> {
    let pull_interval = config.from_file.runtime.pull_interval.unwrap_or(1000);

    let parsed = Parsed {
      cloud: ParsedCloud {
        timeout: config.from_file.cloud.timeout.unwrap_or(10000),
        ssl: config.from_env.cloud.ssl,
        domain: config.from_env.cloud.domain,
        api_key: config.from_env.cloud.api_key,
        id: config.from_env.cloud.id,
      },
      db: ParsedDb {
        timeout: config.from_file.db.timeout.unwrap_or(pull_interval),
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
      network: ParsedNetwork {
        timeout: config.from_file.network.timeout.unwrap_or(pull_interval),
        ip_range: Self::make_ip_range(
          config.from_env.network.ip_range_start,
          config.from_env.network.ip_range_end,
        )?,
      },
      modbus: ParsedModbus {
        timeout: config.from_file.modbus.timeout,
        retries: config.from_file.modbus.retries,
        devices: config
          .from_file
          .modbus
          .devices
          .into_iter()
          .map(|(kind, device)| {
            (
              kind.clone(),
              ParsedDevice {
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
      runtime: ParsedRuntime {
        log_level: config.from_file.runtime.log_level.unwrap_or(
          if config.from_args.dev {
            LogLevel::Debug
          } else {
            LogLevel::Info
          },
        ),
        dev: config.from_args.dev,
        scan_interval: config.from_file.runtime.scan_interval.unwrap_or(60000),
        pull_interval,
        push_interval: config.from_file.runtime.push_interval.unwrap_or(60000),
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
    register: MeasurementRegister,
  ) -> modbus::MeasurementRegister<modbus::RegisterKind> {
    modbus::MeasurementRegister::<modbus::RegisterKind> {
      address: register.address,
      storage: Self::to_modbus_register_kind(register.kind),
      name: register.name,
    }
  }

  fn to_modbus_detect_register(
    register: DetectRegister,
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
    register: IdRegister,
  ) -> modbus::IdRegister<modbus::RegisterKind> {
    modbus::IdRegister::<modbus::RegisterKind> {
      address: register.address,
      storage: Self::to_modbus_register_kind(register.kind),
    }
  }

  fn to_modbus_register_kind(register: RegisterKind) -> modbus::RegisterKind {
    match register {
      RegisterKind::U16(NumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::U16(modbus::NumericRegisterKind { multiplier })
      }
      RegisterKind::U32(NumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::U32(modbus::NumericRegisterKind { multiplier })
      }
      RegisterKind::U64(NumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::U64(modbus::NumericRegisterKind { multiplier })
      }
      RegisterKind::S16(NumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::S16(modbus::NumericRegisterKind { multiplier })
      }
      RegisterKind::S32(NumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::S32(modbus::NumericRegisterKind { multiplier })
      }
      RegisterKind::S64(NumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::S64(modbus::NumericRegisterKind { multiplier })
      }
      RegisterKind::F32(NumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::F32(modbus::NumericRegisterKind { multiplier })
      }
      RegisterKind::F64(NumericRegisterKind { multiplier }) => {
        modbus::RegisterKind::F64(modbus::NumericRegisterKind { multiplier })
      }
      RegisterKind::String(StringRegisterKind { length }) => {
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

  fn read_from_file(location: Option<String>) -> Result<FromFile, ReadError> {
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
      serde_yaml::from_str::<FromFile>(raw.as_str())?
    };

    Ok(from_file)
  }

  async fn read_from_file_async(
    location: Option<String>,
  ) -> Result<FromFile, ReadError> {
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
      serde_yaml::from_str::<FromFile>(raw.as_str())?
    };

    Ok(from_file)
  }

  fn read_from_env() -> Result<FromEnv, env::VarError> {
    let from_env = FromEnv {
      cloud: CloudEnv {
        ssl: env::var("PIDGEON_CLOUD_SSL").map_or_else(|_| false, |_| true),
        domain: env::var("PIDGEON_CLOUD_DOMAIN")?,
        api_key: env::var("PIDGEON_CLOUD_API_KEY").ok(),
        id: env::var("PIDGEON_CLOUD_ID").ok(),
      },
      db: DbEnv {
        ssl: env::var("PIDGEON_DB_SSL").map_or_else(|_| false, |_| true),
        domain: env::var("PIDGEON_DB_DOMAIN")?,
        port: env::var("PIDGEON_DB_PORT").ok(),
        user: env::var("PIDGEON_DB_USER")?,
        password: env::var("PIDGEON_DB_PASSWORD").ok(),
        name: env::var("PIDGEON_DB_NAME")?,
      },
      network: NetworkEnv {
        ip_range_start: env::var("PIDGEON_NETWORK_IP_RANGE_START")?,
        ip_range_end: env::var("PIDGEON_NETWORK_IP_RANGE_END")?,
      },
    };

    Ok(from_env)
  }

  fn read_from_args() -> FromArgs {
    FromArgs::parse()
  }
}
