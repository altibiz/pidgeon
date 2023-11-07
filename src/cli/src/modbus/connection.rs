use std::net::SocketAddr;

use futures_time::future::FutureExt;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_modbus::{client::Context, prelude::Reader, Slave};

use super::span::SimpleSpan;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Destination {
  pub socket: SocketAddr,
  pub slave: Option<u8>,
}

pub type Response = Vec<u16>;

#[derive(Debug)]
pub struct Connection {
  destination: Destination,
  ctx: Context,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
  #[error("Failed to connect")]
  Connect(#[from] std::io::Error),

  #[error("Wrong slave number")]
  Slave,
}

impl Connection {
  pub async fn connect(destination: Destination) -> Result<Self, ConnectError> {
    match destination.slave {
      Some(slave) => Self::connect_slave(destination.socket, slave).await,
      None => Self::connect_standalone(destination.socket).await,
    }
  }

  pub async fn connect_standalone(
    socket: SocketAddr,
  ) -> Result<Self, ConnectError> {
    let stream = TcpStream::connect(socket).await?;
    let ctx = tokio_modbus::prelude::tcp::attach(stream);
    Ok(Self {
      destination: Destination {
        socket,
        slave: None,
      },
      ctx,
    })
  }

  pub async fn connect_slave(
    socket: SocketAddr,
    slave: u8,
  ) -> Result<Self, ConnectError> {
    if Slave(slave) < Slave::min_device() || Slave(slave) > Slave::max_device()
    {
      return Err(ConnectError::Slave);
    }

    let stream = TcpStream::connect(socket).await?;
    let ctx = tokio_modbus::prelude::rtu::attach_slave(stream, Slave(slave));
    Ok(Self {
      destination: Destination {
        socket,
        slave: Some(slave),
      },
      ctx,
    })
  }

  pub fn socket(&self) -> SocketAddr {
    self.destination.socket
  }

  pub fn slave(&self) -> Option<u8> {
    self.destination.slave
  }
}

#[derive(Copy, Clone, Debug)]
pub struct Params {
  timeout: futures_time::time::Duration,
  backoff: tokio::time::Duration,
  retries: usize,
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum ConnectionReadParamsError {
  #[error("Failed converting timeout")]
  TimeoutConversion(#[from] std::num::TryFromIntError),

  #[error("Failed converting backoff")]
  BackoffConversoin(#[from] chrono::OutOfRangeError),
}

impl Params {
  pub fn new(
    timeout: chrono::Duration,
    backoff: chrono::Duration,
    retries: usize,
  ) -> Result<Self, ConnectionReadParamsError> {
    let timeout: futures_time::time::Duration =
      futures_time::time::Duration::from_millis(
        timeout.num_milliseconds() as u64
      );
    let backoff: std::time::Duration = backoff.to_std()?;
    Ok(Self {
      timeout,
      backoff,
      retries,
    })
  }

  pub fn timeout(self) -> Result<chrono::Duration, std::num::TryFromIntError> {
    Ok(chrono::Duration::milliseconds(
      self.timeout.as_millis().try_into()?,
    ))
  }

  pub fn backoff(self) -> Result<chrono::Duration, chrono::OutOfRangeError> {
    Ok(chrono::Duration::from_std(self.backoff)?)
  }

  pub fn retries(self) -> usize {
    self.retries
  }
}

#[derive(Debug, Error)]
pub enum Error {
  #[error("Failed connecting to device")]
  Connection(#[from] std::io::Error),

  #[error("Failed to parse response")]
  Parse(#[from] anyhow::Error),
}

impl Connection {
  pub async fn read(
    &mut self,
    span: SimpleSpan,
    params: Params,
  ) -> Result<Response, Error> {
    fn flatten_result<T, E1, E2>(
      result: Result<Result<T, E1>, E2>,
    ) -> Result<T, E1>
    where
      E1: From<E2>,
    {
      result?
    }

    let data = {
      let timeout = params.timeout;
      let backoff = params.backoff;
      let retries = params.retries;
      let mut retried = 0;
      let mut result = flatten_result(
        self
          .ctx
          .read_holding_registers(span.address, span.quantity)
          .timeout(timeout)
          .await,
      );
      while result.is_err() && retried != retries {
        tokio::time::sleep(backoff).await;
        result = flatten_result(
          self
            .ctx
            .read_holding_registers(span.address, span.quantity)
            .timeout(timeout)
            .await,
        );
        retried = retried + 1;
      }
      result
    }?;

    Ok(data)
  }
}
