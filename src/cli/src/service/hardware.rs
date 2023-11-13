use std::path::PathBuf;

use crate::*;

pub struct Client {
  temperature_monitor: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
  #[error("Reading from filesystem failed")]
  FilesystemError(#[from] std::io::Error),

  #[error("Reading from filesystem failed")]
  ParseError(#[from] core::num::ParseFloatError),
}

impl Client {
  pub fn new(config: config::Parsed) -> Self {
    Self {
      temperature_monitor: config.hardware.temperature_monitor,
    }
  }

  pub async fn temperature(&self) -> Result<f32, ReadError> {
    let temperature =
      tokio::fs::read_to_string(self.temperature_monitor.as_path()).await?;
    let temperature = temperature.parse::<f32>()? / 1000f32;
    Ok(temperature)
  }
}
