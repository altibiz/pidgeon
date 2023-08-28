use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client as HttpClient, Error as HttpError, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    pub fn new(domain: String, ssl: bool) -> Result<Self, CloudClientError> {
        let push_endpoint = match ssl {
            true => format!("https://{domain}/push"),
            false => format!("http://{domain}/push"),
        };

        let http = HttpClient::builder()
            .timeout(Duration::from_secs(10))
            .gzip(true)
            .build()?;

        let client = Self {
            push_endpoint,
            http,
        };

        Ok(client)
    }

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
