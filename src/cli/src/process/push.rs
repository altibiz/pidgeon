#[allow(unused_imports)]
use crate::{service::*, *};
use either::Either;

// TODO: store push time in log

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

#[async_trait::async_trait]
impl process::Recurring for Process {
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let config = self.config.values().await;

    let mut last_pushed_id =
      match self.services.db().get_last_successful_push_log().await? {
        Some(db::Log {
          last: Some(last), ..
        }) => last,
        _ => 0,
      };

    let mut limit = config.cloud.message_limit;
    loop {
      match self.try_push(last_pushed_id, limit).await? {
        Either::Left(TryPushResponse {
          log_status: db::LogStatus::Failure,
          log_response,
          last_push_id,
        }) => {
          limit /= 2;
          if limit == 0 {
            last_pushed_id = last_pushed_id
              .checked_add(1)
              .ok_or_else(|| anyhow::anyhow!("Last pushed id overflow"))?;
            limit = config.cloud.message_limit;
          }
          let log = db::Log {
            id: 0,
            timestamp: chrono::Utc::now(),
            last: Some(last_push_id),
            status: db::LogStatus::Failure,
            kind: db::LogKind::Push,
            response: serde_json::Value::String(log_response),
          };
          self.services.db().insert_log(log).await?;
        }
        Either::Left(TryPushResponse {
          log_status: db::LogStatus::Success,
          last_push_id,
          log_response,
        }) => {
          let log = db::Log {
            id: 0,
            timestamp: chrono::Utc::now(),
            last: Some(last_push_id),
            status: db::LogStatus::Success,
            kind: db::LogKind::Push,
            response: serde_json::Value::String(log_response),
          };
          self.services.db().insert_log(log).await?;
          break;
        }
        Either::Right(()) => return Ok(()),
      };
    }

    Ok(())
  }
}

struct TryPushResponse {
  last_push_id: i64,
  log_status: db::LogStatus,
  log_response: String,
}

impl Process {
  async fn try_push(
    &self,
    from_id: i64,
    limit: i64,
  ) -> anyhow::Result<either::Either<TryPushResponse, ()>> {
    let mut measurements_to_push =
      self.services.db().get_measurements(from_id, limit).await?;
    let measurements_len = measurements_to_push.len();

    let last_push_id =
      match measurements_to_push.iter().max_by(|x, y| x.id.cmp(&y.id)) {
        Some(measurement) => measurement.id,
        None => return Ok(Either::Right(())),
      };

    let start = chrono::Utc::now();
    let result = self
      .services
      .cloud()
      .push(
        measurements_to_push
          .drain(0..)
          .map(|measurement| cloud::Measurement {
            meter_id: measurement.source,
            timestamp: measurement.timestamp,
            data: serde_json::json!(measurement.data),
          })
          .collect(),
      )
      .await;
    let end = chrono::Utc::now();
    let took = end.signed_duration_since(start).num_milliseconds();

    let (log_status, log_response) = match result {
      Ok(cloud::Response {
        success: true,
        text,
        ..
      }) => {
        tracing::info!(
          "Successfully pushed {:?} measurements from {:?} to {:?} took {} ms",
          measurements_len,
          from_id,
          last_push_id,
          took,
        );
        (db::LogStatus::Success, text)
      }
      Ok(cloud::Response {
        success: false,
        text,
        code,
      }) => {
        tracing::error!(
          "Failed pushing {:?} measurements from {:?} to {:?} with code {:?} took {} ms",
          measurements_len,
          from_id,
          last_push_id,
          code,
          took,
        );
        (db::LogStatus::Failure, text)
      }
      Err(error) => {
        tracing::error!(
          "Failed pushing {:?} measurements from {:?} to {:?} took {} ms {}",
          measurements_len,
          from_id,
          last_push_id,
          took,
          error,
        );
        (db::LogStatus::Failure, error.to_string())
      }
    };

    Ok(Either::Left(TryPushResponse {
      last_push_id,
      log_status,
      log_response,
    }))
  }
}
