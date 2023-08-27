use chrono::{DateTime, Utc};
use sqlx::{migrate::Migrator, Pool, Postgres, QueryBuilder};
use thiserror::Error;

// bulk insert issue https://github.com/launchbadge/sqlx/issues/294

pub struct Client {
    pool: Pool<Postgres>,
}

#[derive(sqlx::FromRow)]
pub struct Measurement {
    pub id: i64,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "log_kind", rename_all = "lowercase")]
pub enum LogKind {
    Success,
    Failure,
}

#[derive(sqlx::FromRow)]
pub struct Log {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub last_measurement: i64,
    pub kind: LogKind,
    pub response: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Sqlx error")]
    Sqlx(#[from] sqlx::Error),

    #[error("Migration failed")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

impl Client {
    pub async fn new() -> Result<Self, ClientError> {
        let pool = Pool::connect("postgres://pidgeon@localhost/pidgeon?sslmode=disable").await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), ClientError> {
        MIGRATOR.run(&self.pool).await?;

        Ok(())
    }

    pub async fn insert_measurements(
        &self,
        measurements: Vec<Measurement>,
    ) -> Result<(), ClientError> {
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

    pub async fn get_measurements(&self, from: i64) -> Result<Vec<Measurement>, ClientError> {
        let measurements = sqlx::query_as!(
            Measurement,
            r#"
               select *
               from measurements
               where measurements.id > $1 
            "#,
            from
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(measurements)
    }

    pub async fn insert_log(&self, log: Log) -> Result<(), ClientError> {
        sqlx::query!(
            r#"
                insert into logs (timestamp, last_measurement, kind, response)
                values ($1, $2, $3, $4)
            "#,
            log.timestamp,
            log.last_measurement,
            log.kind as LogKind,
            log.response
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_last_successful_log(&self) -> Result<Log, ClientError> {
        let log = sqlx::query_as!(
            Log,
            r#"
                select id, timestamp, last_measurement, kind as "kind: LogKind", response
                from logs
                where logs.kind = 'success'::log_kind
                order by timestamp desc
                limit 1
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(log)
    }
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
