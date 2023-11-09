use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::{
  migrate::Migrator, types::ipnetwork::IpNetwork, FromRow, Pool, Postgres,
  QueryBuilder, Type,
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Client {
  pool: Pool<Postgres>,
}

#[derive(Debug, Copy, Clone, Type, Eq, PartialEq)]
#[sqlx(type_name = "device_status", rename_all = "lowercase")]
pub enum DeviceStatus {
  Healthy,
  Unreachable,
  Inactive,
}

#[derive(Debug, Clone, FromRow)]
pub struct Device {
  pub id: String,
  pub kind: String,
  pub status: DeviceStatus,
  pub address: IpNetwork,
  pub seen: DateTime<Utc>,
  pub pinged: DateTime<Utc>,
  pub slave: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Measurement {
  pub id: i64,
  pub source: String,
  pub timestamp: DateTime<Utc>,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
pub struct Health {
  pub id: i64,
  pub source: String,
  pub timestamp: DateTime<Utc>,
  pub status: DeviceStatus,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, Type, Eq, PartialEq)]
#[sqlx(type_name = "log_status", rename_all = "lowercase")]
pub enum LogStatus {
  Success,
  Failure,
}

#[derive(Debug, Copy, Clone, Type, Eq, PartialEq)]
#[sqlx(type_name = "log_kind", rename_all = "lowercase")]
pub enum LogKind {
  Push,
  Update,
}

#[derive(Debug, Clone, FromRow)]
pub struct Log {
  pub id: i64,
  pub timestamp: DateTime<Utc>,
  pub last: i64,
  pub kind: LogKind,
  pub status: LogStatus,
  pub response: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum Error {
  #[error("Sqlx error")]
  Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Error)]
pub enum MigrateError {
  #[error("Migration failed")]
  Migration(#[from] sqlx::migrate::MigrateError),
}

impl Client {
  pub fn new(
    timeout: u64,
    ssl: bool,
    domain: String,
    port: Option<u16>,
    user: String,
    password: Option<String>,
    name: String,
  ) -> Self {
    let mut options = sqlx::postgres::PgConnectOptions::new()
      .host(domain.as_str())
      .username(user.as_str())
      .database(name.as_str())
      .options([("statement_timeout", timeout.to_string().as_str())]);

    if let Some(port) = port {
      options = options.port(port);
    }

    if let Some(password) = password {
      options = options.password(password.as_str());
    }

    options = options.ssl_mode(sqlx::postgres::PgSslMode::Disable);
    if ssl {
      options = options.ssl_mode(sqlx::postgres::PgSslMode::Require);
    }

    let pool = sqlx::Pool::connect_lazy_with(options);

    let client = Self { pool };

    client
  }

  #[tracing::instrument(skip(self))]
  pub async fn migrate(&self) -> Result<(), MigrateError> {
    MIGRATOR.run(&self.pool).await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_devices(&self) -> Result<Vec<Device>, Error> {
    let devices = sqlx::query_as!(
      Device,
      r#"
        select id, kind, status as "status: DeviceStatus", seen, pinged, address, slave
        from devices
      "#,
    )
    .fetch_all(&self.pool)
    .await?;

    Ok(devices)
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_device(&self, id: &str) -> Result<Option<Device>, Error> {
    let devices = sqlx::query_as!(
      Device,
      r#"
        select id, kind, status as "status: DeviceStatus", seen, pinged, address, slave
        from devices
        where id = $1
      "#,
      id
    )
    .fetch_optional(&self.pool)
    .await?;

    Ok(devices)
  }

  #[tracing::instrument(skip(self))]
  pub async fn insert_device(&self, device: Device) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        insert into devices (id, kind, status, seen, pinged, address, slave)
        values ($1, $2, $3, $4, $5, $6, $7)
      "#,
      device.id,
      device.kind,
      device.status as DeviceStatus,
      device.seen,
      device.pinged,
      device.address,
      device.slave
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn delete_device(&self, id: &str) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        delete from devices
        where id = $1
      "#,
      id,
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn update_device_status(
    &self,
    id: &str,
    status: DeviceStatus,
    seen: DateTime<Utc>,
    pinged: DateTime<Utc>,
  ) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        update devices
        set status = $2::device_status, seen = $3, pinged = $4
        where id = $1
      "#,
      id,
      status as DeviceStatus,
      seen,
      pinged
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn update_device_destination(
    &self,
    id: &str,
    address: IpNetwork,
    slave: Option<i32>,
    seen: DateTime<Utc>,
    pinged: DateTime<Utc>,
  ) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        update devices
        set address = $2, slave = $3, seen = $4, pinged = $5
        where id = $1
      "#,
      id,
      address,
      slave,
      seen,
      pinged
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip_all, fields(count = measurements.len()))]
  pub async fn insert_measurements(
    &self,
    measurements: Vec<Measurement>,
  ) -> Result<(), Error> {
    let mut query_builder =
      QueryBuilder::new("insert into measurements (source, timestamp, data)");

    query_builder.push_values(measurements, |mut builder, measurement| {
      builder.push_bind(measurement.source);
      builder.push_bind(measurement.timestamp);
      builder.push_bind(measurement.data);
    });

    let query = query_builder.build();

    query.execute(&self.pool).await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_measurements(
    &self,
    from: i64,
    limit: i64,
  ) -> Result<Vec<Measurement>, Error> {
    #[allow(clippy::panic)]
    let measurements = sqlx::query_as!(
      Measurement,
      r#"
        select id, source, timestamp, data
        from measurements
        where measurements.id > $1 
        limit $2
      "#,
      from,
      limit
    )
    .fetch_all(&self.pool)
    .await?;

    Ok(measurements)
  }

  #[tracing::instrument(skip(self))]
  pub async fn insert_health(&self, health: Health) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        insert into health (source, timestamp, status, data)
        values ($1, $2, $3, $4)
      "#,
      health.source,
      health.timestamp,
      health.status as DeviceStatus,
      health.data
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_health(
    &self,
    from: i64,
    limit: i64,
  ) -> Result<Vec<Health>, Error> {
    #[allow(clippy::panic)]
    let health = sqlx::query_as!(
      Health,
      r#"
        select id, source, timestamp, status as "status: DeviceStatus", data
        from health
        where health.id > $1 
        limit $2
      "#,
      from,
      limit
    )
    .fetch_all(&self.pool)
    .await?;

    Ok(health)
  }

  #[tracing::instrument(skip(self))]
  pub async fn insert_log(&self, log: Log) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        insert into logs (timestamp, last, status, kind, response)
        values ($1, $2, $3, $4, $5)
      "#,
      log.timestamp,
      log.last,
      log.status as LogStatus,
      log.kind as LogKind,
      log.response
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_last_successful_push_log(
    &self,
  ) -> Result<Option<Log>, Error> {
    #[allow(clippy::panic)]
    let log = sqlx::query_as!(
      Log,
      r#"
        select id, timestamp, last, kind as "kind: LogKind", status as "status: LogStatus", response
        from logs
        where status = 'success'::log_status and kind = 'push'::log_kind
        order by timestamp desc
        limit 1
      "#
    )
    .fetch_optional(&self.pool)
    .await?;

    Ok(log)
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_last_successful_update_log(
    &self,
  ) -> Result<Option<Log>, Error> {
    #[allow(clippy::panic)]
    let log = sqlx::query_as!(
      Log,
      r#"
        select id, timestamp, last, kind as "kind: LogKind", status as "status: LogStatus", response
        from logs
        where status = 'success'::log_status and kind = 'update'::log_kind
        order by timestamp desc
        limit 1
      "#
    )
    .fetch_optional(&self.pool)
    .await?;

    Ok(log)
  }
}

pub fn to_network(ip: IpAddr) -> IpNetwork {
  #[allow(clippy::unwrap_used)]
  IpNetwork::new(ip, 24).unwrap()
}

pub fn to_ip(ip: IpNetwork) -> IpAddr {
  ip.ip()
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
