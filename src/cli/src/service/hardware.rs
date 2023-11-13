use std::path::PathBuf;

use crate::*;

pub struct Service {
  temperature_monitor: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
  #[error("Reading from filesystem failed")]
  FilesystemError(#[from] std::io::Error),

  #[error("Reading from filesystem failed")]
  ParseError(#[from] core::num::ParseFloatError),
}

impl super::Service for Service {
  fn new(config: config::Values) -> Self {
    Self {
      temperature_monitor: config.hardware.temperature_monitor.into(),
    }
  }
}

impl Service {
  pub async fn temperature(&self) -> Result<f32, ReadError> {
    let temperature =
      tokio::fs::read_to_string(self.temperature_monitor.as_path()).await?;
    let temperature = temperature.parse::<f32>()? / 1000f32;
    Ok(temperature)
  }
}
