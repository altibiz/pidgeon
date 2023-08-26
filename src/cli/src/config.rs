use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use tokio_modbus::Address;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub registers: Vec<Register>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Register {
    pub name: String,
    pub address: Address,
    pub kind: RegisterKind,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegisterKind {
    U16,
    U32,
    S16,
    S32,
}

#[derive(Debug, Error)]
pub enum ReadConfigError {
    #[error("Missing project directories")]
    MissingProjectDirs,

    #[error("Failed reading config file")]
    Read(#[from] std::io::Error),

    #[error("Failed serializing config")]
    Deserialization(#[from] serde_yaml::Error),
}

pub async fn read_config() -> Result<Config, ReadConfigError> {
    match directories::ProjectDirs::from("com", "altibiz", "pidgeon") {
        Some(project_dirs) => {
            let path = project_dirs.config_dir().join("config.yaml");
            let raw = fs::read_to_string(path).await?;
            let config = serde_yaml::from_str::<Config>(raw.as_str())?;
            Ok(config)
        }
        _ => Err(ReadConfigError::MissingProjectDirs),
    }
}
