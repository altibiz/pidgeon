#[allow(unused_imports, reason = "services")]
use crate::{service::*, *};

pub(crate) struct Process {
  #[allow(dead_code, reason = "process")]
  config: config::Manager,

  #[allow(dead_code, reason = "process")]
  services: service::Container,
}

impl Process {
  pub(crate) fn new(
    config: config::Manager,
    services: service::Container,
  ) -> Self {
    Self { config, services }
  }
}

impl super::Process for Process {}

#[async_trait::async_trait]
impl process::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let temperature = self.services.i2c().read_temperature().await?;

    let result = self
      .services
      .cloud()
      .update(serde_json::json!(Health { temperature }), vec![])
      .await;

    let (log_status, log_response) = match result {
      Ok(cloud::Response {
        success: true,
        text,
        ..
      }) => {
        tracing::info!("Successfully updated pidgeon health");
        (db::LogStatus::Success, text)
      }
      Ok(cloud::Response {
        success: false,
        text,
        code,
      }) => {
        tracing::error!("Failed updating pidgeon health with code {:?}", code);
        (db::LogStatus::Failure, text)
      }
      Err(error) => {
        tracing::error!("Failed updating pidgeon health {}", error);
        (db::LogStatus::Failure, error.to_string())
      }
    };
    let log = db::Log {
      id: 0,
      timestamp: chrono::Utc::now(),
      last: None,
      status: log_status,
      kind: db::LogKind::Update,
      response: serde_json::Value::String(log_response),
    };
    self.services.db().insert_log(log).await?;

    Ok(())
  }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Health {
  temperature: f32,
}
