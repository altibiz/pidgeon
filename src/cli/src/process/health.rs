use crate::{service::*, *};

pub(crate) struct Process {
  #[allow(unused)]
  config: config::Manager,

  #[allow(unused)]
  services: service::Container,
}

impl process::Process for Process {
  fn new(config: config::Manager, services: service::Container) -> Self {
    Self { config, services }
  }
}

#[async_trait::async_trait]
impl process::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let temperature = self.services.hardware().read_temperature().await?;

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
      }) => (db::LogStatus::Success, text),
      Ok(cloud::Response {
        success: false,
        text,
        ..
      }) => (db::LogStatus::Failure, text),
      Err(_) => (db::LogStatus::Failure, "connection error".to_string()),
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
