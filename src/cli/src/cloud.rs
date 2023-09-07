use std::{fs, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::{
  header::{HeaderMap, HeaderValue, InvalidHeaderValue},
  Client as HttpClient, Error as HttpError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudMeasurement {
  pub device_id: String,
  pub timestamp: DateTime<Utc>,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PushRequest {
  timestamp: DateTime<Utc>,
  measurements: Vec<CloudMeasurement>,
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

  #[error("Invalid header error")]
  InvalidHeader(#[from] InvalidHeaderValue),

  #[error("IO error")]
  IO(#[from] std::io::Error),
}

impl CloudClient {
  pub fn new(
    domain: String,
    ssl: bool,
    api_key: Option<String>,
    timeout: u64,
    id: Option<String>,
  ) -> Result<Self, CloudClientError> {
    let id = match id {
      Some(id) => id,
      None => {
        "raspberry-pi-".to_string()
          + fs::read_to_string("/sys/firmware/devicetree/base/serial-number")?
            .as_str()
      }
    };

    let protocol = if ssl { "https" } else { "http" };

    let push_endpoint = format!("{protocol}://{domain}/push/{id}");

    let mut headers = HeaderMap::new();
    match api_key {
      Some(api_key) => {
        let value = HeaderValue::from_str(api_key.as_str())?;
        headers.insert("X-API-Key", value);
      }
      None => {
        let value = HeaderValue::from_str((id + "-oil-rulz-5000").as_str())?;
        headers.insert("X-API-Key", value);
      }
    };

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
    let request = PushRequest {
      timestamp: chrono::offset::Utc::now(),
      measurements,
    };

    let http_response = self
      .http
      .post(self.push_endpoint.clone())
      .json(&request)
      .send()
      .await?;

    let status_code = http_response.status();
    let success = status_code.is_success();
    let text = http_response.text().await?;

    if success {
      tracing::debug! {
        "Successfully pushed {:?} measurements",
        request.measurements.len()
      };
    } else {
      tracing::warn! {
        "Failed pushing {:?} measurements: {:?} {:?}",
        request.measurements.len(),
        status_code,
        text.clone()
      };
    }

    let response = CloudResponse { success, text };

    Ok(response)
  }
}
