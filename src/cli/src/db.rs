use chrono::{DateTime, Utc};
use sqlx::{migrate::Migrator, Pool, Postgres, QueryBuilder};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct DbClient {
  pool: Pool<Postgres>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbMeasurement {
  pub id: i64,
  pub source: String,
  pub timestamp: DateTime<Utc>,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "log_kind", rename_all = "lowercase")]
pub enum DbLogKind {
  Success,
  Failure,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbLog {
  pub id: i64,
  pub timestamp: DateTime<Utc>,
  pub last_measurement: i64,
  pub kind: DbLogKind,
  pub response: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum DbClientError {
  #[error("Sqlx error")]
  Sqlx(#[from] sqlx::Error),

  #[error("Migration failed")]
  Migration(#[from] sqlx::migrate::MigrateError),
}

impl DbClient {
  pub fn new(
    timeout: u64,
    ssl: bool,
    domain: String,
    port: Option<u16>,
    user: String,
    password: Option<String>,
    name: String,
  ) -> Result<Self, DbClientError> {
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

    let db_client = Self { pool };

    Ok(db_client)
  }

  #[tracing::instrument(skip(self))]
  pub async fn migrate(&self) -> Result<(), DbClientError> {
    MIGRATOR.run(&self.pool).await?;

    Ok(())
  }

  #[tracing::instrument(skip_all, fields(count = measurements.len()))]
  pub async fn insert_measurements(
    &self,
    measurements: Vec<DbMeasurement>,
  ) -> Result<(), DbClientError> {
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
  ) -> Result<Vec<DbMeasurement>, DbClientError> {
    #[allow(clippy::panic)]
    let measurements = sqlx::query_as!(
      DbMeasurement,
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
  pub async fn insert_log(&self, log: DbLog) -> Result<(), DbClientError> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        insert into logs (timestamp, last_measurement, kind, response)
        values ($1, $2, $3, $4)
      "#,
      log.timestamp,
      log.last_measurement,
      log.kind as DbLogKind,
      log.response
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_last_successful_log(
    &self,
  ) -> Result<Option<DbLog>, DbClientError> {
    #[allow(clippy::panic)]
    let log = sqlx::query_as!(
      DbLog,
      r#"
        select id, timestamp, last_measurement, kind as "kind: DbLogKind", response
        from logs
        where logs.kind = 'success'::log_kind
        order by timestamp desc
        limit 1
      "#
    )
    .fetch_optional(&self.pool)
    .await?;

    Ok(log)
  }
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
