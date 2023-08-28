use clap::{command, Parser};
use ipnet::{IpAddrRange, Ipv4AddrRange};
use serde::{Deserialize, Serialize};
use std::{
    env::{self, VarError},
    fs,
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::modbus::Register;

#[derive(Default, Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct ConfigFromArgs {}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct ConfigFromFile {
    pub registers: Vec<Register>,
    pub timeout: u64,
}

#[derive(Default, Debug, Clone)]
struct ConfigFromEnv {
    pub cloud_ssl: bool,
    pub cloud_domain: String,
    pub db_connection_string: String,
    pub scan_ip_range_start: String,
    pub scan_ip_range_end: String,
}

#[derive(Debug, Clone)]
struct Config {
    #[allow(unused)]
    from_args: ConfigFromArgs,
    from_file: ConfigFromFile,
    from_env: ConfigFromEnv,
}

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
    Var(#[from] VarError),
}

impl ConfigManager {
    pub fn new() -> Result<Self, ConfigManagerError> {
        dotenv::dotenv()?;

        let from_args = Self::read_from_args().unwrap_or_default();
        let from_file = Self::read_from_file().unwrap_or_default();
        let from_env = Self::read_from_env().unwrap_or_default();

        let config_manager = Self {
            values: Arc::new(Mutex::new(Config {
                from_args: from_args.clone(),
                from_file: from_file.clone(),
                from_env: from_env.clone(),
            })),
        };

        Ok(config_manager)
    }

    pub async fn registers(&self) -> Vec<Register> {
        self.values.lock().await.from_file.registers.clone()
    }

    pub fn scan_timeout(&self) -> Duration {
        Duration::from_millis(self.values.blocking_lock().from_file.timeout)
    }

    pub fn scan_ip_range(&self) -> IpAddrRange {
        let parsed_ip_range_start_end = {
            let values = self.values.blocking_lock();

            (
                values.from_env.scan_ip_range_start.parse(),
                values.from_env.scan_ip_range_end.parse(),
            )
        };
        let mut ip_range_start_end = (
            "192.168.1.0".parse().unwrap(),
            "192.168.1.255".parse().unwrap(),
        );
        if parsed_ip_range_start_end.0.is_ok() || parsed_ip_range_start_end.1.is_ok() {
            ip_range_start_end = (
                parsed_ip_range_start_end.0.unwrap(),
                parsed_ip_range_start_end.1.unwrap(),
            );
        }

        IpAddrRange::from(Ipv4AddrRange::new(
            ip_range_start_end.0,
            ip_range_start_end.1,
        ))
    }

    pub fn db_connection_string(&self) -> String {
        self.values
            .blocking_lock()
            .from_env
            .db_connection_string
            .clone()
    }

    pub fn cloud_ssl(&self) -> bool {
        self.values.blocking_lock().from_env.cloud_ssl.clone()
    }

    pub fn cloud_domain(&self) -> String {
        self.values.blocking_lock().from_env.cloud_domain.clone()
    }

    fn read_from_file() -> Result<ConfigFromFile, ConfigManagerError> {
        let from_file = match directories::ProjectDirs::from("com", "altibiz", "pidgeon") {
            Some(project_dirs) => {
                let path = project_dirs.config_dir().join("config.yaml");
                let raw = fs::read_to_string(path)?;
                let config = serde_yaml::from_str::<ConfigFromFile>(raw.as_str())?;
                config
            }
            _ => return Err(ConfigManagerError::MissingProjectDirs),
        };

        Ok(from_file)
    }

    fn read_from_env() -> Result<ConfigFromEnv, ConfigManagerError> {
        let from_env = ConfigFromEnv {
            cloud_ssl: env::var("PIDGEON_CLOUD_SSL").map_or_else(|e| false, |v| true),
            cloud_domain: env::var("PIDGEON_CLOUD_DOMAIN")?,
            db_connection_string: env::var("PIDGEON_DB_CONNECTION_STRING")?,
            scan_ip_range_start: env::var("PIDGEON_SCAN_IP_RANGE_START")?,
            scan_ip_range_end: env::var("PIDGEON_SCAN_IP_RANGE_END")?,
        };

        Ok(from_env)
    }

    fn read_from_args() -> Result<ConfigFromArgs, ConfigManagerError> {
        let from_args = ConfigFromArgs::parse();

        Ok(from_args)
    }
}
