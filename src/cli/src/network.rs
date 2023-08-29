use ipnet::IpAddrRange;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct NetworkScanner {
  ip_range: IpAddrRange,
  timeout: Duration,
  ips: Arc<Mutex<Vec<IpAddr>>>,
}

#[derive(Debug, Error)]
pub enum NetworkScannerError {
  #[allow(unused)]
  #[error("placeholder")]
  Placeholder,
}

impl NetworkScanner {
  pub fn new(
    ip_range: IpAddrRange,
    timeout: Duration,
  ) -> Result<Self, NetworkScannerError> {
    let network_scanner = Self {
      ip_range,
      timeout,
      ips: Arc::new(Mutex::new(Vec::new())),
    };

    Ok(network_scanner)
  }

  #[allow(unused)]
  pub async fn ips(&self) -> Vec<IpAddr> {
    self.ips.lock().await.clone()
  }

  pub async fn scan(&self) -> Vec<IpAddr> {
    let mut matched_ips: Vec<IpAddr> = Vec::new();
    let ip_scans = self
      .ip_range
      .map(|ip| (ip, tokio::spawn(Self::scan_port(ip, 502, self.timeout))))
      .collect::<Vec<(IpAddr, JoinHandle<bool>)>>();

    for (ip, scan) in ip_scans {
      if let Ok(true) = scan.await {
        matched_ips.push(ip)
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
    let timeout =
      tokio::time::timeout(timeout, TcpStream::connect(&socket_address)).await;
    matches!(timeout, Ok(Ok(_)))
  }
}
