use std::{fs, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::{
  header::{HeaderMap, HeaderValue, InvalidHeaderValue},
  Client as HttpClient, Error as HttpError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::*;

// TODO: pidgeon diag in health

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
  pub device_id: String,
  pub timestamp: DateTime<Utc>,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
  pub device_id: String,
  pub timestamp: DateTime<Utc>,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushRequest {
  timestamp: DateTime<Utc>,
  measurements: Vec<Measurement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRequest {
  timestamp: DateTime<Utc>,
  pidgeon: serde_json::Value,
  health: Vec<Health>,
}

#[derive(Debug, Clone)]
pub struct Response {
  pub success: bool,
  pub text: String,
}

#[derive(Debug, Clone)]
pub struct Service {
  push_endpoint: String,
  update_endpoint: String,
  http: HttpClient,
}

#[derive(Debug, Error)]
pub enum ConstructionError {
  #[error("HTTP client construction error")]
  HttpError(#[from] HttpError),

  #[error("Invalid header error")]
  InvalidHeader(#[from] InvalidHeaderValue),

  #[error("IO error")]
  IO(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum PushError {
  #[error("HTTP Post error")]
  HttpError(#[from] HttpError),
}

impl service::Service for Service {
  fn new(config: config::Values) -> Self {
    #[allow(clippy::unwrap_used)]
    let id = match config.cloud.id {
      Some(id) => id,
      None => {
        "pidgeon-".to_string()
          + fs::read_to_string("/sys/firmware/devicetree/base/serial-number")
            .unwrap()
            .as_str()
      }
    };

    let protocol = if config.cloud.ssl { "https" } else { "http" };

    let domain = config.cloud.domain;

    let push_endpoint = format!("{protocol}://{domain}/push/{id}");
    let update_endpoint = format!("{protocol}://{domain}/update/{id}");

    let mut headers = HeaderMap::new();
    match config.cloud.api_key {
      Some(api_key) => {
        #[allow(clippy::unwrap_used)]
        let value = HeaderValue::from_str(api_key.as_str()).unwrap();
        headers.insert("X-API-Key", value);
      }
      None => {
        #[allow(clippy::unwrap_used)]
        let value =
          HeaderValue::from_str((id + "-oil-rulz-5000").as_str()).unwrap();
        headers.insert("X-API-Key", value);
      }
    };

    let builder = HttpClient::builder()
      .timeout(Duration::from_millis(
        config.cloud.timeout.num_milliseconds() as u64,
      ))
      .default_headers(headers)
      .gzip(true);

    #[allow(clippy::unwrap_used)]
    let http = builder.build().unwrap();

    let client = Self {
      push_endpoint,
      update_endpoint,
      http,
    };

    client
  }
}

impl Service {
  #[tracing::instrument(skip_all, fields(count = measurements.len()))]
  pub async fn push(
    &self,
    measurements: Vec<Measurement>,
  ) -> Result<Response, PushError> {
    let request = PushRequest {
      timestamp: chrono::offset::Utc::now(),
      measurements,
    };

    let http_response = self
      .http
      .post(self.push_endpoint.clone())
      .json(&request)
      .send()
      .await;
    if let Err(error) = &http_response {
      tracing::warn! {
        %error,
        "Failed pushing {:?} measurements: connection error",
        request.measurements.len(),
      }
    }
    let http_response = http_response?;

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

    let response = Response { success, text };

    Ok(response)
  }

  #[tracing::instrument(skip(self))]
  pub async fn update(
    &self,
    pidgeon: serde_json::Value,
    health: Vec<Health>,
  ) -> Result<Response, PushError> {
    let request = UpdateRequest {
      timestamp: chrono::offset::Utc::now(),
      pidgeon,
      health,
    };

    let http_response = self
      .http
      .post(self.update_endpoint.clone())
      .json(&request)
      .send()
      .await;
    if let Err(error) = &http_response {
      tracing::warn! {
        %error,
        "Failed pushing {:?} measurements: connection error",
        request.health.len(),
      }
    }
    let http_response = http_response?;

    let status_code = http_response.status();
    let success = status_code.is_success();
    let text = http_response.text().await?;

    if success {
      tracing::debug! {
        "Successfully updated {:?} health",
        request.health.len()
      };
    } else {
      tracing::warn! {
        "Failed updating {:?} health: {:?} {:?}",
        request.health.len(),
        status_code,
        text.clone()
      };
    }

    let response = Response { success, text };

    Ok(response)
  }
}
