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

use ipnet::{IpAddrRange, Ipv4AddrRange};
use scan::scan_range;

mod scan;

#[tokio::main(worker_threads = 4)]
async fn main() {
    let ip_addresses = scan_range(
        IpAddrRange::from(Ipv4AddrRange::new(
            "192.168.1.0".parse().unwrap(),
            "192.168.1.255".parse().unwrap(),
        )),
        502,
        10000,
    )
    .await;

    dbg!(ip_addresses);
}
