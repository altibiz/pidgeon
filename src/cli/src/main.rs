#![deny(
  unsafe_code,
  // reason = "Let's just not do it"
)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  // reason = "We have to handle errors properly"
)]

use config::ReadConfigError;
use ipnet::{IpAddrRange, Ipv4AddrRange};
use modbus::ModbusError;
use scan::scan_range;
use std::net::{SocketAddr, SocketAddrV4};
use thiserror::Error;
use tokio_modbus::Slave;

use crate::{config::read_config, modbus::read};

mod config;
mod modbus;
mod scan;

#[derive(Debug, Error)]
pub enum PidgeonError {
    #[error("Failed connecting to device")]
    Modbus(#[from] ModbusError),

    #[error("Failed reading config")]
    ConfigRead(#[from] ReadConfigError),
}

#[tokio::main(worker_threads = 4)]
async fn main() -> Result<(), PidgeonError> {
    let ip_addresses = scan_range(
        IpAddrRange::from(Ipv4AddrRange::new(
            "192.168.1.0".parse().unwrap(),
            "192.168.1.255".parse().unwrap(),
        )),
        502,
        10000,
    )
    .await;

    dbg!(ip_addresses.clone());

    let config = read_config().await?;

    match ip_addresses.first() {
        Some(std::net::IpAddr::V4(ipv4_address)) => {
            dbg!(
                read(
                    SocketAddr::V4(SocketAddrV4::new(ipv4_address.clone(), 502)),
                    Slave(1),
                    config,
                )
                .await?
            );
        }
        _ => {}
    };

    Ok(())
}
