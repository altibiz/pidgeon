use clap::{command, Parser};
use ipnet::{IpAddrRange, Ipv4AddrRange};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct ConfigFromArgs {
  /// Run in development mode
  #[arg(short, long)]
  dev: bool,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
  Abb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Register {
  pub name: String,
  pub address: u16,
  pub kind: RegisterKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StringRegisterKind {
  pub length: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegisterKind {
  U16,
  U32,
  S16,
  S32,
  String(StringRegisterKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectRegister {
  pub address: u16,
  pub kind: RegisterKind,
  pub r#match: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeviceDetect {
  One(DetectRegister),
  Many(Vec<DetectRegister>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
  pub detect: DeviceDetect,
  pub registers: Vec<Register>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModbusFile {
  timeout: u64,
  devices: HashMap<DeviceKind, DeviceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloudFile {
  timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigFromFile {
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
struct ConfigFromEnv {
  cloud: CloudEnv,
  db: DbEnv,
  network: NetworkEnv,
}

#[derive(Debug, Clone)]
struct Config {
  from_args: ConfigFromArgs,
  from_file: ConfigFromFile,
  from_env: ConfigFromEnv,
}

#[derive(Debug, Clone)]
pub struct ParsedRuntimeConfig {
  pub dev: bool,
  pub log_level: LogLevel,
  pub scan_interval: u64,
  pub pull_interval: u64,
  pub push_interval: u64,
}

#[derive(Debug, Clone)]
pub struct ParsedDbConfig {
  pub timeout: u64,
  pub ssl: bool,
  pub domain: String,
  pub port: Option<u16>,
  pub user: String,
  pub password: Option<String>,
  pub name: String,
}

#[derive(Debug, Clone)]
pub struct ParsedNetworkConfig {
  pub timeout: u64,
  pub ip_range: IpAddrRange,
}

#[derive(Debug, Clone)]
pub struct ParsedCloudConfig {
  pub timeout: u64,
  pub ssl: bool,
  pub domain: String,
  pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedModbusConfig {
  pub timeout: u64,
  pub devices: HashMap<DeviceKind, DeviceConfig>,
}

#[derive(Debug, Clone)]
pub struct ParsedConfig {
  pub cloud: ParsedCloudConfig,
  pub db: ParsedDbConfig,
  pub network: ParsedNetworkConfig,
  pub modbus: ParsedModbusConfig,
  pub runtime: ParsedRuntimeConfig,
}

#[derive(Debug, Clone)]
pub struct ConfigManager {
  values: Arc<Mutex<Config>>,
}

#[derive(Debug, Error)]
pub enum ConfigManagerError {
  #[error("Failed loading dotenv file")]
  Dotenv(#[from] dotenv::Error),

  #[error("Failed creating project directories")]
  MissingProjectDirs,

  #[error("Failed reading config file")]
  FileConfigRead(#[from] std::io::Error),

  #[error("Failed serializing config")]
  FileConfigDeserialization(#[from] serde_yaml::Error),

  #[error("Failed reading environment variable")]
  Var(#[from] std::env::VarError),

  #[error("Failed parsing port")]
  PortParse(#[from] std::num::ParseIntError),
}

impl ConfigManager {
  pub fn new() -> Result<Self, ConfigManagerError> {
    dotenv::dotenv()?;

    let config = Self::read()?;

    let config_manager = Self {
      values: Arc::new(Mutex::new(config)),
    };

    Ok(config_manager)
  }

  #[allow(unused)]
  pub fn config(&self) -> Result<ParsedConfig, ConfigManagerError> {
    let config = self.values.blocking_lock().clone();
    let parsed = Self::parse_config(config)?;
    Ok(parsed)
  }

  #[allow(unused)]
  pub async fn config_async(&self) -> Result<ParsedConfig, ConfigManagerError> {
    let config = self.values.lock().await.clone();
    let parsed = Self::parse_config(config)?;
    Ok(parsed)
  }

  #[allow(unused)]
  pub fn reload(&self) -> Result<ParsedConfig, ConfigManagerError> {
    let from_file = Self::read_from_file()?;

    let config = {
      let mut values = self.values.blocking_lock();
      values.from_file = from_file;
      values.clone()
    };

    let parsed = Self::parse_config(config)?;

    Ok(parsed)
  }

  #[allow(unused)]
  pub async fn reload_async(&self) -> Result<ParsedConfig, ConfigManagerError> {
    let from_file = Self::read_from_file_async().await?;

    let config = {
      let mut values = self.values.lock().await;
      values.from_file = from_file;
      values.clone()
    };

    let parsed = Self::parse_config(config)?;

    Ok(parsed)
  }

  fn parse_config(config: Config) -> Result<ParsedConfig, ConfigManagerError> {
    let pull_interval = config.from_file.runtime.pull_interval.unwrap_or(1000);

    let parsed = ParsedConfig {
      cloud: ParsedCloudConfig {
        timeout: config.from_file.cloud.timeout.unwrap_or(10000),
        ssl: config.from_env.cloud.ssl,
        domain: config.from_env.cloud.domain,
        api_key: config.from_env.cloud.api_key,
      },
      db: ParsedDbConfig {
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
      network: ParsedNetworkConfig {
        timeout: config.from_file.network.timeout.unwrap_or(pull_interval),
        ip_range: Self::make_ip_range(
          config.from_env.network.ip_range_start,
          config.from_env.network.ip_range_end,
        ),
      },
      modbus: ParsedModbusConfig {
        timeout: config.from_file.modbus.timeout,
        devices: config.from_file.modbus.devices,
      },
      runtime: ParsedRuntimeConfig {
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

  fn make_ip_range(start: String, end: String) -> IpAddrRange {
    let parsed_ip_range_start_end = (start.parse(), end.parse());
    let mut ip_range_start_end = (
      #[allow(clippy::unwrap_used)]
      "192.168.1.0".parse().unwrap(),
      #[allow(clippy::unwrap_used)]
      "192.168.1.255".parse().unwrap(),
    );
    if parsed_ip_range_start_end.0.is_ok()
      || parsed_ip_range_start_end.1.is_ok()
    {
      ip_range_start_end = (
        #[allow(clippy::unwrap_used)]
        parsed_ip_range_start_end.0.unwrap(),
        #[allow(clippy::unwrap_used)]
        parsed_ip_range_start_end.1.unwrap(),
      );
    }

    IpAddrRange::from(Ipv4AddrRange::new(
      ip_range_start_end.0,
      ip_range_start_end.1,
    ))
  }

  fn read() -> Result<Config, ConfigManagerError> {
    let from_args = Self::read_from_args()?;
    let from_file = Self::read_from_file()?;
    let from_env = Self::read_from_env()?;

    let config = Config {
      from_args,
      from_file,
      from_env,
    };

    Ok(config)
  }

  fn read_from_file() -> Result<ConfigFromFile, ConfigManagerError> {
    let from_file =
      match directories::ProjectDirs::from("com", "altibiz", "pidgeon") {
        Some(project_dirs) => {
          let path = project_dirs.config_dir().join("config.yaml");
          let raw = fs::read_to_string(path)?;
          serde_yaml::from_str::<ConfigFromFile>(raw.as_str())?
        }
        _ => return Err(ConfigManagerError::MissingProjectDirs),
      };

    Ok(from_file)
  }

  async fn read_from_file_async() -> Result<ConfigFromFile, ConfigManagerError>
  {
    let from_file =
      match directories::ProjectDirs::from("com", "altibiz", "pidgeon") {
        Some(project_dirs) => {
          let path = project_dirs.config_dir().join("config.yaml");
          let raw = tokio::fs::read_to_string(path).await?;
          serde_yaml::from_str::<ConfigFromFile>(raw.as_str())?
        }
        _ => return Err(ConfigManagerError::MissingProjectDirs),
      };

    Ok(from_file)
  }

  fn read_from_env() -> Result<ConfigFromEnv, ConfigManagerError> {
    let from_env = ConfigFromEnv {
      cloud: CloudEnv {
        ssl: env::var("PIDGEON_CLOUD_SSL").map_or_else(|_| false, |_| true),
        domain: env::var("PIDGEON_CLOUD_DOMAIN")?,
        api_key: env::var("PIDGEON_CLOUD_API_KEY").ok(),
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

  fn read_from_args() -> Result<ConfigFromArgs, ConfigManagerError> {
    let from_args = ConfigFromArgs::parse();

    Ok(from_args)
  }
}
