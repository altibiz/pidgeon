use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client as HttpClient, Error as HttpError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CloudClient {
  push_endpoint: String,
  http: HttpClient,
}

#[derive(Debug, Error)]
pub enum CloudClientError {
  #[error("HTTP error")]
  HttpError(#[from] HttpError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudMeasurement {
  pub source: String,
  pub timestamp: DateTime<Utc>,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct CloudResponse {
  pub success: bool,
  pub text: String,
}

impl CloudClient {
  pub fn new(
    domain: String,
    ssl: bool,
    timeout: u64,
  ) -> Result<Self, CloudClientError> {
    let protocol = if ssl { "https" } else { "http" };
    let push_endpoint = format!("{protocol}://{domain}/push");

    let http = HttpClient::builder()
      .timeout(Duration::from_millis(timeout))
      .gzip(true)
      .build()?;

    let client = Self {
      push_endpoint,
      http,
    };

    Ok(client)
  }

  #[tracing::instrument(skip_all, fields(count = measurements.len()))]
  pub async fn push_measurements(
    &self,
    measurements: Vec<CloudMeasurement>,
  ) -> Result<CloudResponse, CloudClientError> {
    let response = self
      .http
      .post(self.push_endpoint.clone())
      .json(&measurements)
      .send()
      .await?;

    let success = response.status().is_success();
    let text = response.text().await?;

    Ok(CloudResponse { success, text })
  }
}
