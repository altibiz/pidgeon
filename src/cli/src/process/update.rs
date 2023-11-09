use crate::*;

pub struct Process {
  db: db::Client,
  cloud: cloud::Client,
  modbus: modbus::Client,
}

impl Process {
  pub fn new(
    db: db::Client,
    cloud: cloud::Client,
    modbus: modbus::Client,
  ) -> Self {
    Self { db, cloud, modbus }
  }
}

impl super::Process for Process {}

#[async_trait::async_trait]
impl super::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let last_pushed_id = match self.db.get_last_successful_log().await? {
      Some(log) => log.last_measurement,
      None => 0,
    };

    let mut measurements_to_push =
      self.db.get_measurements(last_pushed_id, 1000).await?;
    let last_push_id =
      match measurements_to_push.iter().max_by(|x, y| x.id.cmp(&y.id)) {
        Some(measurement) => measurement.id,
        None => return Ok(()),
      };

    let result = self
      .cloud
      .push(
        measurements_to_push
          .drain(0..)
          .map(|measurement| cloud::Measurement {
            device_id: measurement.source,
            timestamp: measurement.timestamp,
            data: measurement.data.to_string(),
          })
          .collect(),
      )
      .await;

    let (log_kind, log_response) = match result {
      Ok(cloud::Response {
        success: true,
        text,
      }) => (db::LogKind::Success, text),
      Ok(cloud::Response {
        success: false,
        text,
      }) => (db::LogKind::Failure, text),
      Err(_) => (db::LogKind::Failure, "connection error".to_string()),
    };
    let log = db::Log {
      id: 0,
      timestamp: chrono::Utc::now(),
      last: last_push_id,
      kind: log_kind,
      response: serde_json::Value::String(log_response),
    };
    self.db.insert_log(log).await?;

    Ok(())
  }
}
