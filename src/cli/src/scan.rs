use ipnet::IpAddrRange;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct NetworkScanner {
    range: IpAddrRange,
    timeout: Duration,
    ips: Arc<Mutex<Vec<IpAddr>>>,
}

#[derive(Debug, Error)]
pub enum NetworkScannerError {
    #[error("placeholder")]
    Placeholder,
}

impl NetworkScanner {
    pub fn new(range: IpAddrRange, timeout: Duration) -> Result<Self, NetworkScannerError> {
        let network_scanner = Self {
            range,
            timeout,
            ips: Arc::new(Mutex::new(Vec::new())),
        };

        Ok(network_scanner)
    }

    pub async fn ips(&self) -> Vec<IpAddr> {
        self.ips.lock().await.clone()
    }

    pub async fn scan(&self) -> Vec<IpAddr> {
        let mut matched_ips: Vec<IpAddr> = Vec::new();
        let ip_scans = self
            .range
            .map(|ip| {
                (
                    ip,
                    tokio::spawn(Self::scan_port(ip, 502, self.timeout.clone())),
                )
            })
            .collect::<Vec<(IpAddr, JoinHandle<bool>)>>();

        for (ip, scan) in ip_scans {
            match scan.await {
                Ok(true) => matched_ips.push(ip),
                _ => {}
            }
        }

        {
            let mut ips = self.ips.lock().await;
            let previous = ips.clone();
            *ips = matched_ips;
            previous
        }
    }

    async fn scan_port(ip_address: IpAddr, port: u16, timeout: Duration) -> bool {
        let socket_address = SocketAddr::new(ip_address, port);
        match tokio::time::timeout(timeout, TcpStream::connect(&socket_address)).await {
            Ok(Ok(_)) => true,
            _ => false,
        }
    }
}
