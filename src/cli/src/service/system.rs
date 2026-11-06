use std::{
  path::{Path, PathBuf},
  str::FromStr
};

use crate::*;

// TODO: instead of slice of &str use PathBuf for specific parsing functions down there

// NOTE: these var paths are mostly static and don't need dynamic config read
const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";
const UPTIME: &str = "/proc/uptime";
const STAT: &str = "/proc/stat";
const LOADAVG: &str = "/proc/loadavg";
const MEMINFO: &str = "/proc/meminfo";
const SELF_STATUS: &str = "/proc/self/status";

#[derive(Debug, Clone)]
pub(crate) struct Service {
  cpu_temperature: PathBuf,
  cpu_frequency: PathBuf,
  throttled: PathBuf
}

impl super::Service for Service {
  fn new(config: config::Values) -> Self {
    Self {
      cpu_temperature: config.hardware.cpu_temperature_path.into(),
      cpu_frequency: config.hardware.cpu_frequency_path.into(),
      throttled: config.hardware.throttled_path.into()
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CpuTimes {
  pub(crate) busy: u64,
  pub(crate) iowait: u64,
  pub(crate) total: u64,
}

impl CpuTimes {
  pub(crate) fn usage_since(&self, previous: &CpuTimes) -> Option<(f32, f32)> {

    let total = self.total.checked_sub(previous.total)?;
    let busy = self.busy.checked_sub(previous.busy)?;
    let iowait = self.iowait.checked_sub(previous.iowait)?;
    if total == 0 {
      return None;
    }

    Some((
      percent(busy, total),
      percent(iowait, total)
    ))
  }
}


#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
  pub(crate) boot_id: String,
  pub(crate) uptime_seconds: i64,
  pub(crate) cpu_temperature_celsius: Option<f32>,
  pub(crate) cpu_frequency_mhz: Option<i32>,
  pub(crate) throttled_flags: Option<i32>,
  pub(crate) cpu_times: Option<CpuTimes>,
  pub(crate) load: Option<(f32, f32, f32)>,
  pub(crate) memory_total_bytes: Option<i64>,
  pub(crate) memory_available_bytes: Option<i64>,
  pub(crate) swap_used_bytes: Option<i64>,
  pub(crate) pidgeon_rss_bytes: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReadError {
  #[error("Reading from filesystem failed")]
  FilesystemError(#[from] std::io::Error),

  #[error("Parsing {0} failed")]
  ParseError(&'static str),
}

impl Service {

  #[tracing::instrument(skip(self))]
  pub(crate) async fn read(&self) -> Result<Snapshot, ReadError> {

    let boot_id = tokio::fs::read_to_string(BOOT_ID).await?.trim().to_owned();

    let uptime_seconds = parse_uptime(&tokio::fs::read_to_string(UPTIME).await?)
      .ok_or(ReadError::ParseError(UPTIME))?;

    let meminfo = read_string(MEMINFO).await;
    let meminfo = meminfo.as_deref();

    let swap_total = meminfo.and_then(|m| parse_kb_field(m, "SwapTotal"));
    let swap_free = meminfo.and_then(|m| parse_kb_field(m, "SwapFree"));

    let snapshot = Snapshot {
      boot_id,
      uptime_seconds,
      cpu_temperature_celsius: read_number::<f32>(&self.cpu_temperature)
        .await
        .map(|milidegrees| milidegrees / 1000f32),
      cpu_frequency_mhz: read_number::<i32>(&self.cpu_frequency)
        .await
        .and_then(|khz| khz.checked_div(1000)),
      throttled_flags: read_string(&self.throttled)
        .await
        .and_then(|hex| i32::from_str_radix(&hex, 16).ok()),
      cpu_times: read_string(STAT)
        .await
        .and_then(|stat| parse_cpu_times(&stat)),
      load: read_string(LOADAVG)
        .await
        .and_then(|loadavg| parse_loadavg(&loadavg)),
      memory_total_bytes: meminfo.and_then(|m| parse_kb_field(m, "MemTotal")),
      memory_available_bytes: meminfo
        .and_then(|m| parse_kb_field(m, "MemAvailable")),
      swap_used_bytes: swap_total
        .zip(swap_free)
        .and_then(|(total, free)| total.checked_sub(free)),
      pidgeon_rss_bytes: read_string(SELF_STATUS)
        .await
        .and_then(|status| parse_kb_field(&status, "VmRSS")),
    };

    tracing::trace!("Read system snapshot {:?}", snapshot);

    return Ok(snapshot);
  }

}

async fn read_string(path: impl AsRef<Path>) -> Option<String> {
  let path = path.as_ref();
  match tokio::fs::read_to_string(path).await {
    Ok(content) => Some(content.trim().to_owned()),
    Err(error) => {
      tracing::debug!("Failed reading {} {}", path.display(), error);
      None
    }
  }
}

async fn read_number<T: FromStr>(path: impl AsRef<Path>) -> Option<T> {
  read_string(path).await?.parse::<T>().ok()
}

fn percent(part: u64, total: u64) -> f32 {
  (part as f64 / total as f64 * 100f64) as f32
}

fn parse_uptime(uptime: &str) -> Option<i64> {
  let seconds = uptime.split_whitespace().next()?.parse::<f64>().ok()?;
  Some(seconds as i64)
}

fn parse_loadavg(loadavg: &str) -> Option<(f32, f32, f32)> {
  let mut fields =
    loadavg.split_whitespace().map(|f| f.parse::<f32>().ok());
  Some((fields.next()??, fields.next()??, fields.next()??))
}

fn parse_cpu_times(stat: &str) -> Option<CpuTimes> {

  let mut fields =
    stat.lines().next()?.split_whitespace();

  if fields.next()? != "cpu" {
    return None;
  }

  let values = fields
    .take(8)
    .map(|field| field.parse::<u64>().ok())
    .collect::<Option<Vec<_>>>()?;

  let &[_user, _nice, _system, idle, iowait, _irq, _softirq, _steal] =
    values.as_slice()
  else {
    return None;
  };

  let total = values
    .iter()
    .try_fold(0u64, |sum, value| sum.checked_add(*value))?;

  let busy = total.checked_sub(idle)?.checked_sub(iowait)?;

  Some(CpuTimes{
    busy,
    iowait,
    total
  })
}

fn parse_kb_field(content: &str, key: &str) -> Option<i64> {
  let line = content.lines().find(|line| {
    line
      .split_once(":")
      .is_some_and(|(name, _)| name.trim() == key)
  })?;
  let kb = line.split_whitespace().nth(1)?.parse::<i64>().ok()?;
  return kb.checked_mul(1024);
}

