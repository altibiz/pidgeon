use std::{future::Future, time::Duration};
use thiserror::Error;
use tokio::{runtime::Runtime as AsyncRuntime, sync::oneshot, task::JoinHandle, time::interval};

use crate::{
    cloud::{CloudClient, CloudClientError, CloudMeasurement},
    config::{ConfigManager, ConfigManagerError},
    db::{DbClient, DbClientError, DbMeasurement},
    modbus,
    modbus::{ModbusClient, ModbusClientError},
    scan::{NetworkScanner, NetworkScannerError},
};

pub struct Runtime {
    config_manager: ConfigManager,
    network_scanner: NetworkScanner,
    modbus_client: ModbusClient,
    db_client: DbClient,
    cloud_client: CloudClient,
    r#async: AsyncRuntime,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Config error")]
    ConfigManager(#[from] ConfigManagerError),

    #[error("Network scanner error")]
    NetworkScanner(#[from] NetworkScannerError),

    #[error("Modbus error")]
    ModbusClient(#[from] ModbusClientError),

    #[error("Db error")]
    DbClient(#[from] DbClientError),

    #[error("Cloud error")]
    CloudClient(#[from] CloudClientError),
}

impl Runtime {
    pub fn new() -> Result<Self, RuntimeError> {
        let config_manager = ConfigManager::new()?;

        let scan_ip_range = config_manager.scan_ip_range();
        let scan_timeout = config_manager.scan_timeout();
        let network_scanner = NetworkScanner::new(scan_ip_range, scan_timeout)?;

        let modbus_client = ModbusClient::new()?;

        let db_connection_string = config_manager.db_connection_string();
        let db_client = DbClient::new(db_connection_string)?;

        let cloud_domain = config_manager.cloud_domain();
        let cloud_ssl = config_manager.cloud_ssl();
        let cloud_client = CloudClient::new(cloud_domain, cloud_ssl)?;

        let r#async = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();

        let runtime = Self {
            config_manager,
            network_scanner,
            modbus_client,
            db_client,
            cloud_client,
            r#async,
        };

        Ok(runtime)
    }

    pub fn start(&self) -> Result<(), RuntimeError> {
        self.r#async.block_on(async { self.start_async().await })
    }

    // TODO: intervals with macro
    async fn start_async(&self) -> Result<(), RuntimeError> {
        self.on_setup().await?;

        self.register_interval(Self::on_scan, Duration::from_secs(60));
        self.register_interval(Self::on_pull, Duration::from_secs(1));
        self.register_interval(Self::on_push, Duration::from_secs(60));

        let _ = tokio::signal::ctrl_c().await;

        Ok(())
    }

    async fn on_setup(&self) -> Result<(), RuntimeError> {
        self.db_client.migrate().await?;

        Ok(())
    }

    async fn on_scan(&self) -> Result<(), RuntimeError> {
        let _ = self.network_scanner.scan().await;

        Ok(())
    }

    async fn on_pull(&self) -> Result<(), RuntimeError> {
        let ips = self.network_scanner.ips().await;
        let registers = self.config_manager.registers().await;

        let mut measurements = Vec::<DbMeasurement>::new();
        for ip in ips {
            let values = self.modbus_client.read(ip, 1, registers.clone()).await?;
            let json = modbus::registers_to_json(values);

            measurements.push(DbMeasurement {
                source: "todo".to_string(),
                timestamp: chrono::Utc::now(),
                data: json,
            });
        }

        self.db_client.insert_measurements(measurements).await?;

        Ok(())
    }

    async fn on_push(&self) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn register_interval<T, F>(
        &self,
        task: T,
        duration: Duration,
    ) -> (oneshot::Sender<()>, JoinHandle<()>)
    where
        T: FnMut(&Self) -> F + Send + 'static,
        F: Future<Output = Result<(), RuntimeError>> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let wrapped_task = move |this| Box::pin(task(this));
        let handle = self.r#async.spawn(async move {
            let mut interval = interval(duration);
            loop {
                if let Ok(_) = rx.try_recv() {
                    return;
                }

                interval.tick().await;

                if let Err(error) = wrapped_task(self).await {}
            }
        });

        (tx, handle)
    }
}
