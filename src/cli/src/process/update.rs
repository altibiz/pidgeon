use crate::{service::*, *};

pub(crate) struct Process {
  #[allow(unused)]
  config: config::Manager,

  #[allow(unused)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct PidgeonHealth {
  temperature: f32,
}

#[async_trait::async_trait]
impl process::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let config = self.config.values().await;

    let last_pushed_id =
      match self.services.db().get_last_successful_update_log().await? {
        Some(db::Log {
          last: Some(last), ..
        }) => last,
        _ => 0,
      };

    let mut health_to_update = self
      .services
      .db()
      .get_health(last_pushed_id, config.cloud.message_limit)
      .await?;
    let health_len = health_to_update.len();

    let last_push_id =
      match health_to_update.iter().max_by(|x, y| x.id.cmp(&y.id)) {
        Some(health) => health.id,
        None => return Ok(()),
      };

    let result = self
      .services
      .cloud()
      .update(
        serde_json::Value::Null,
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
        ..
      }) => {
        tracing::info!(
          "Successfully updated {:?} health from {:?} to {:?}",
          health_len,
          last_pushed_id,
          last_push_id
        );
        (db::LogStatus::Success, text)
      }
      Ok(cloud::Response {
        success: false,
        text,
        code,
      }) => {
        tracing::error!(
          "Failed updating {:?} health from {:?} to {:?} with code {:?}",
          health_len,
          last_pushed_id,
          last_push_id,
          code
        );
        (db::LogStatus::Failure, text)
      }
      Err(error) => {
        tracing::error!(
          "Failed pushing {:?} health from {:?} to {:?} {}",
          health_len,
          last_pushed_id,
          last_push_id,
          error
        );
        (db::LogStatus::Failure, error.to_string())
      }
    };
    let log = db::Log {
      id: 0,
      timestamp: chrono::Utc::now(),
      last: Some(last_push_id),
      status: log_status,
      kind: db::LogKind::Update,
      response: serde_json::Value::String(log_response),
    };
    self.services.db().insert_log(log).await?;

    Ok(())
  }
}
