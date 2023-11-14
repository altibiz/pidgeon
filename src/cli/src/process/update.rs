use crate::{service::*, *};

pub(crate) struct Process {
  config: config::Manager,
  services: service::Container,
}

impl process::Process for Process {
  fn new(config: config::Manager, services: service::Container) -> Self {
    Self { config, services }
  }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct PidgeonHealth {
  temperature: f32,
}

#[async_trait::async_trait]
impl process::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let temperature = self.services.hardware().read_temperature().await?;

    let last_pushed_id =
      match self.services.db().get_last_successful_update_log().await? {
        Some(log) => log.last,
        None => 0,
      };

    let mut health_to_update =
      self.services.db().get_health(last_pushed_id, 1000).await?;
    let last_push_id =
      match health_to_update.iter().max_by(|x, y| x.id.cmp(&y.id)) {
        Some(measurement) => measurement.id,
        None => return Ok(()),
      };

    let result = self
      .services
      .cloud()
      .update(
        serde_json::json!(PidgeonHealth { temperature }),
        health_to_update
          .drain(0..)
          .map(|health| cloud::Health {
            device_id: health.source,
            timestamp: health.timestamp,
            data: serde_json::json!(health.data),
          })
          .collect(),
      )
      .await;

    let (log_status, log_response) = match result {
      Ok(cloud::Response {
        success: true,
        text,
      }) => (db::LogStatus::Success, text),
      Ok(cloud::Response {
        success: false,
        text,
      }) => (db::LogStatus::Failure, text),
      Err(_) => (db::LogStatus::Failure, "connection error".to_string()),
    };
    let log = db::Log {
      id: 0,
      timestamp: chrono::Utc::now(),
      last: last_push_id,
      status: log_status,
      kind: db::LogKind::Update,
      response: serde_json::Value::String(log_response),
    };
    self.services.db().insert_log(log).await?;

    Ok(())
  }
}
