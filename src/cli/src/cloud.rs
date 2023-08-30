use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{
  header::{HeaderMap, HeaderValue, InvalidHeaderValue},
  Client as HttpClient, Error as HttpError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

#[derive(Debug, Clone)]
pub struct CloudClient {
  push_endpoint: String,
  http: HttpClient,
}

#[derive(Debug, Error)]
pub enum CloudClientError {
  #[error("HTTP error")]
  HttpError(#[from] HttpError),

  #[error("Invalid header  error")]
  InvalidHeader(#[from] InvalidHeaderValue),
}

impl CloudClient {
  pub fn new(
    domain: String,
    ssl: bool,
    api_key: Option<String>,
    timeout: u64,
  ) -> Result<Self, CloudClientError> {
    let protocol = if ssl { "https" } else { "http" };
    let push_endpoint = format!("{protocol}://{domain}/push");

    let mut headers = HeaderMap::new();
    if let Some(api_key) = api_key {
      let value = HeaderValue::from_str(api_key.as_str())?;
      headers.insert("X-API-Key", value);
    }

    let builder = HttpClient::builder()
      .timeout(Duration::from_millis(timeout))
      .default_headers(headers)
      .gzip(true);

    let http = builder.build()?;

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
    let http_response = self
      .http
      .post(self.push_endpoint.clone())
      .json(&measurements)
      .send()
      .await?;

    let success = http_response.status().is_success();
    let text = http_response.text().await?;

    let response = CloudResponse { success, text };

    Ok(response)
  }
}
