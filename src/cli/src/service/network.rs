use std::net::{IpAddr, SocketAddr};

use ipnet::IpAddrRange;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use crate::*;

#[derive(Debug, Clone)]
pub struct Client {
  ip_range: IpAddrRange,
  timeout: std::time::Duration,
}

impl Client {
  pub fn new(config: config::Values) -> Self {
    Self {
      ip_range: config.network.ip_range,
      timeout: std::time::Duration::from_millis(config.network.timeout),
    }
  }

  #[tracing::instrument(skip(self))]
  pub async fn scan(&self) -> Vec<SocketAddr> {
    let timeout = self.timeout;
    let mut matched_ips = Vec::new();
    let ip_scans = self
      .ip_range
      .map(|ip| {
        let socket_address = to_socket(ip);
        (
          socket_address,
          tokio::spawn(async move {
            let timeout = tokio::time::timeout(
              timeout,
              TcpStream::connect(&socket_address),
            )
            .await;
            matches!(timeout, Ok(Ok(_)))
          }),
        )
      })
      .collect::<Vec<(SocketAddr, JoinHandle<bool>)>>();

    for (ip, scan) in ip_scans {
      if let Ok(true) = scan.await {
        matched_ips.push(ip)
      }
    }

    tracing::debug! {
      "Found {:?} ips",
      matched_ips.len()
    };

    matched_ips
  }
}

pub fn to_socket(ip: IpAddr) -> SocketAddr {
  SocketAddr::new(ip, 502)
}

pub fn to_ip(socket: SocketAddr) -> IpAddr {
  socket.ip()
}
