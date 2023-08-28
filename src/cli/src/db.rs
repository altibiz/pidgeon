use chrono::{DateTime, Utc};
use sqlx::{migrate::Migrator, Pool, Postgres, QueryBuilder};
use thiserror::Error;

pub struct DbClient {
    pool: Pool<Postgres>,
}

#[derive(sqlx::FromRow)]
pub struct DbMeasurement {
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "log_kind", rename_all = "lowercase")]
pub enum DbLogKind {
    Success,
    Failure,
}

#[derive(sqlx::FromRow)]
pub struct DbLog {
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
    pub fn new(connection_string: String) -> Result<Self, DbClientError> {
        let pool = Pool::connect_lazy(connection_string.as_str())?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), DbClientError> {
        MIGRATOR.run(&self.pool).await?;

        Ok(())
    }

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

    pub async fn get_measurements(
        &self,
        from: i64,
        limit: i64,
    ) -> Result<Vec<DbMeasurement>, DbClientError> {
        let measurements = sqlx::query_as!(
            DbMeasurement,
            r#"
               select source, timestamp, data
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

    pub async fn insert_log(&self, log: DbLog) -> Result<(), DbClientError> {
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

    pub async fn get_last_successful_log(&self) -> Result<DbLog, DbClientError> {
        let log = sqlx::query_as!(
            DbLog,
            r#"
                select timestamp, last_measurement, kind as "kind: DbLogKind", response
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
