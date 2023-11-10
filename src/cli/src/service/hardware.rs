use std::path::PathBuf;

pub struct Client {
  hwmon: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
  #[error("Reading from filesystem failed")]
  FilesystemError(#[from] std::io::Error),

  #[error("Reading from filesystem failed")]
  ParseError(#[from] core::num::ParseFloatError),
}

impl Client {
  pub fn new(hwmon: String) -> Self {
    Self {
      hwmon: hwmon.into(),
    }
  }

  pub async fn temperature(&self) -> Result<f32, ReadError> {
    let temperature = tokio::fs::read_to_string(self.hwmon.as_path()).await?;
    let temperature = temperature.parse::<f32>()? / 1000f32;
    Ok(temperature)
  }
}
