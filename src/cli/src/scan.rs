use ipnet::IpAddrRange;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

pub async fn scan_range(ip_address_range: IpAddrRange, port: u16, timeout: u32) -> Vec<IpAddr> {
    let mut matched_ip_adresses: Vec<IpAddr> = Vec::new();
    let ip_address_scans = ip_address_range
        .map(|ip_address| (ip_address, tokio::spawn(scan(ip_address, port, timeout))))
        .collect::<Vec<(IpAddr, JoinHandle<bool>)>>();

    for (ip_address, scan) in ip_address_scans {
        match scan.await {
            Ok(true) => matched_ip_adresses.push(ip_address),
            _ => {}
        }
    }

    matched_ip_adresses
}

pub async fn scan(ip_address: IpAddr, port: u16, timeout: u32) -> bool {
    let timeout_duration = Duration::from_millis(timeout.into());
    let socket_address = SocketAddr::new(ip_address, port);

    match tokio::time::timeout(timeout_duration, TcpStream::connect(&socket_address)).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}
