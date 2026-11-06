
use tokio::sync::Mutex;

#[allow(unused_imports, reason = "services")]
use crate::{service::*, *};

pub(crate) struct Process {
  #[allow(dead_code, reason = "process")]
  config: config::Manager,

  #[allow(dead_code, reason = "process")]
  services: service::Container,

  previous_cpu: Mutex<Option<system::CpuTimes>>
}

impl Process {
  pub(crate) fn new(
    config: config::Manager,
    services: service::Container,
  ) -> Self {
    Self {
      config,
      services,
      previous_cpu: Mutex::new(None)
    }
  }
}

impl super::Process for Process {}

#[async_trait::async_trait]
impl process::Recurring for Process {
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let snapshot = self.services.system().read().await?;

    let (cpu_usage_percent, cpu_iowait_percent ) = {
      let mut previous = self.previous_cpu.lock().await;
      let usage = match (*previous, snapshot.cpu_times) {
        (Some(previous), Some(current)) => current.usage_since(&previous),
        _ => None
      };
      *previous = snapshot.cpu_times;
      usage.unzip()
    };

    let (load_1m, load_5m, load_15m) = match snapshot.load {
      Some((load_1m, load_5m, load_15m))
        => (Some(load_1m), Some(load_5m), Some(load_15m)),
      None => (None, None, None)
    };

    let database_bytes = match self.services.db().get_database_size().await {
      Ok(size) => Some(size),
      Err(error) => {
        tracing::warn!("Failed reading database size {}", error);
        None
      }
    };

    let timestamp = chrono::Utc::now();

    self
      .services
      .db()
      .insert_system_health(db::SystemHealth {
        timestamp: timestamp,
        boot_id: snapshot.boot_id,
        uptime_seconds: snapshot.uptime_seconds,
        cpu_temperature_celsius: snapshot.cpu_temperature_celsius,
        cpu_frequency_mhz: snapshot.cpu_frequency_mhz,
        throttled_flags: snapshot.throttled_flags,
        cpu_usage_percent,
        cpu_iowait_percent,
        load_1m,
        load_5m,
        load_15m,
        memory_total_bytes: snapshot.memory_total_bytes,
        memory_available_bytes: snapshot.memory_available_bytes,
        swap_used_bytes: snapshot.swap_used_bytes,
        pidgeon_rss_bytes: snapshot.pidgeon_rss_bytes,
        database_bytes,
      })
      .await?;

    tracing::debug!("Successfully written health_data {:?}", timestamp);

    Ok(())
  }
}
