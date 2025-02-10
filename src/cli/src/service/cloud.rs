use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{
  header::{HeaderMap, HeaderValue, InvalidHeaderValue},
  Client as HttpClient, Error as HttpError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::*;

// TODO: check if lists are empty before sending requests

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Measurement {
  pub(crate) meter_id: String,
  pub(crate) timestamp: DateTime<Utc>,
  pub(crate) data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Health {
  pub(crate) device_id: String,
  pub(crate) timestamp: DateTime<Utc>,
  pub(crate) data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub(crate) struct Response {
  pub(crate) success: bool,
  pub(crate) text: String,
  pub(crate) code: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct Service {
  push_endpoint: String,
  update_endpoint: String,
  poll_endpoint: String,
  http: HttpClient,
}

#[derive(Debug, Error)]
pub(crate) enum ConstructionError {
  #[error("HTTP client construction error")]
  HttpError(#[from] HttpError),

  #[error("Invalid header error")]
  InvalidHeader(#[from] InvalidHeaderValue),

  #[error("IO error")]
  IO(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub(crate) enum RequestError {
  #[error("HTTP Post error")]
  HttpError(#[from] HttpError),
}

#[async_trait::async_trait]
impl service::Service for Service {
  fn new(config: config::Values) -> Self {
    let id = config.cloud.id;

    let protocol = if config.cloud.ssl { "https" } else { "http" };

    let domain = config.cloud.domain;

    let push_endpoint = format!("{protocol}://{domain}/iot/push/{id}");
    let update_endpoint = format!("{protocol}://{domain}/iot/update/{id}");
    let poll_endpoint = format!("{protocol}://{domain}/iot/poll/{id}");

    let mut headers = HeaderMap::new();
    match config.cloud.api_key {
      Some(api_key) => {
        #[allow(clippy::unwrap_used, reason = "it shouldn't panic")]
        let value = HeaderValue::from_str(api_key.as_str()).unwrap();
        headers.insert("X-API-Key", value);
      }
      None => {
        #[allow(clippy::unwrap_used, reason = "it shouldn't panic")]
        let value = HeaderValue::from_str(id.as_str()).unwrap();
        headers.insert("X-API-Key", value);
      }
    };
    #[allow(clippy::unwrap_used, reason = "it shouldn't panic")]
    let buffer_behavior = HeaderValue::from_str("buffer").unwrap();
    headers.insert("X-Buffer-Behavior", buffer_behavior);

    let builder = HttpClient::builder()
      .timeout(Duration::from_millis(
        config.cloud.timeout.num_milliseconds() as u64,
      ))
      .default_headers(headers)
      .gzip(true);

    #[allow(
      clippy::unwrap_used,
      reason = "works with our system configuration"
    )]
    let http = builder.build().unwrap();

    Self {
      push_endpoint,
      update_endpoint,
      poll_endpoint,
      http,
    }
  }
}

impl Service {
  #[tracing::instrument(skip_all, fields(count = measurements.len()))]
  pub(crate) async fn push(
    &self,
    measurements: Vec<Measurement>,
  ) -> Result<Response, RequestError> {
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
        "Failed pushing {:?} measurements: {:?}",
        request.measurements.len(),
        error,
      }
    }
    let http_response = http_response?;

    let status_code = http_response.status();
    let success = status_code.is_success();
    let text = http_response.text().await?;

    tracing::trace!(
      "Pushed {:?} measurements {:?}",
      request.measurements.len(),
      status_code
    );

    let response = Response {
      success,
      text,
      code: status_code.as_u16(),
    };

    Ok(response)
  }

  #[tracing::instrument(skip_all, fields(count = health.len()))]
  pub(crate) async fn update(
    &self,
    pidgeon: serde_json::Value,
    health: Vec<Health>,
  ) -> Result<Response, RequestError> {
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
        "Failed pushing {:?} health: {:?}",
        request.health.len(),
        error,
      }
    }
    let http_response = http_response?;

    let status_code = http_response.status();
    let success = status_code.is_success();
    let text = http_response.text().await?;

    tracing::trace!(
      "Updated {:?} health {:?}",
      request.health.len(),
      status_code
    );

    let response = Response {
      success,
      text,
      code: status_code.as_u16(),
    };

    Ok(response)
  }

  #[tracing::instrument(skip_all)]
  pub(crate) async fn poll(&self) -> Result<Response, RequestError> {
    let http_response = self.http.get(self.poll_endpoint.clone()).send().await;
    if let Err(error) = &http_response {
      tracing::warn! {
        %error,
        "Failed polling config: {:?}",
        error,
      }
    }
    let http_response = http_response?;

    let status_code = http_response.status();
    let success = status_code.is_success();
    let text = http_response.text().await?;

    tracing::trace!("Polled config {:?}", status_code);

    let response = Response {
      success,
      text,
      code: status_code.as_u16(),
    };

    Ok(response)
  }
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
