use std::net::SocketAddr;

use futures_time::future::FutureExt;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_modbus::{client::Context, prelude::Reader, Slave};

use super::span::SimpleSpan;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub(crate) struct Destination {
  pub(crate) address: SocketAddr,
  pub(crate) slave: Option<u8>,
}

impl Destination {
  pub(crate) fn slaves_for(
    address: SocketAddr,
  ) -> impl Iterator<Item = Destination> {
    (Slave::min_device().0..Slave::max_device().0).map(move |slave| {
      Destination {
        address,
        slave: Some(slave),
      }
    })
  }

  pub(crate) fn standalone_for(address: SocketAddr) -> Destination {
    Destination {
      address,
      slave: None,
    }
  }
}

pub(crate) type Response = Vec<u16>;

#[derive(Debug)]
pub(crate) struct Connection {
  destination: Destination,
  ctx: Option<Context>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectError {
  #[error("Failed to connect")]
  Connect(#[from] std::io::Error),

  #[error("Wrong slave number")]
  Slave,
}

impl Connection {
  pub(crate) fn new(destination: Destination) -> Self {
    Self {
      destination,
      ctx: None,
    }
  }

  pub(crate) async fn ensure_connected(&mut self) -> Result<(), ConnectError> {
    if self.ctx.is_none() {
      let _ = self.reconnect().await?;
    }

    Ok(())
  }

  async fn reconnect(&mut self) -> Result<&mut Context, ConnectError> {
    let ctx = match self.destination.slave {
      Some(slave) => {
        if Slave(slave) < Slave::min_device()
          || Slave(slave) > Slave::max_device()
        {
          return Err(ConnectError::Slave);
        }

        let stream = TcpStream::connect(self.destination.address).await?;
        
        tokio_modbus::prelude::tcp::attach_slave(stream, Slave(slave))
      }
      None => {
        let stream = TcpStream::connect(self.destination.address).await?;
        
        tokio_modbus::prelude::tcp::attach(stream)
      }
    };

    tracing::trace!("Connected");

    self.ctx = Some(ctx);

    #[allow(clippy::unwrap_used)] // NOTE: we just put it in
    Ok(self.ctx.as_mut().unwrap())
  }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Params {
  timeout: futures_time::time::Duration,
  backoff: tokio::time::Duration,
  retries: u32,
}

impl Params {
  pub(crate) fn new(
    timeout: chrono::Duration,
    backoff: chrono::Duration,
    retries: u32,
  ) -> Self {
    let timeout = timeout_from_chrono(timeout);
    let backoff = backoff_from_chrono(backoff);
    Self {
      timeout,
      backoff,
      retries,
    }
  }

  #[inline]
  pub(crate) fn timeout(self) -> chrono::Duration {
    timeout_to_chrono(self.timeout)
  }

  #[inline]
  pub(crate) fn backoff(self) -> chrono::Duration {
    backoff_to_chrono(self.backoff)
  }

  #[inline]
  pub(crate) fn retries(self) -> u32 {
    self.retries
  }
}

#[derive(Debug, Error)]
pub(crate) enum ReadError {
  #[error("Failed connecting")]
  Connection(#[from] ConnectError),

  #[error("Failed reading")]
  Read(std::io::Error),

  #[error("Connection timed out")]
  Timeout(std::io::Error),
}

impl Connection {
  #[tracing::instrument(skip(self), fields(destination = ?self.destination))]
  pub(crate) async fn parameterized_read(
    &mut self,
    span: SimpleSpan,
    params: Params,
  ) -> Result<Response, Vec<(String, ReadError)>> {
    let timeout = params.timeout;
    let backoff = params.backoff;
    let retries = params.retries;
    let mut errors = Vec::new();
    let mut retried = 0;
    let mut response = None;
    while response.is_none() && retried != retries {
      tokio::time::sleep(backoff).await;
      match self.simple_read_impl(span, timeout).await {
        Ok(data) => response = Some(data),
        Err(error) => errors.push((format!("{:?}", &error), error)),
      };
      retried += 1;
    }

    match response {
      Some(response) => {
        tracing::trace!(
          "Successful read with {:?} retries and {:?} errors",
          retried,
          errors.len()
        );
        Ok(response)
      }
      None => {
        tracing::trace!(
          "Failed read with {:?} retries and {:?} errors",
          retried,
          errors.len()
        );
        Err(errors)
      }
    }
  }

  #[tracing::instrument(skip(self), fields(destination = ?self.destination))]
  pub(crate) async fn simple_read(
    &mut self,
    span: SimpleSpan,
    timeout: chrono::Duration,
  ) -> Result<Response, ReadError> {
    let response = self
      .simple_read_impl(span, timeout_from_chrono(timeout))
      .await?;

    tracing::trace!("Simple read successful");

    Ok(response)
  }

  async fn simple_read_impl(
    &mut self,
    span: SimpleSpan,
    timeout: futures_time::time::Duration,
  ) -> Result<Response, ReadError> {
    let response = match &mut self.ctx {
      Some(ctx) => Self::simple_read_impl_connected(ctx, span, timeout).await,
      None => {
        let ctx = self.reconnect().await?;
        Self::simple_read_impl_connected(ctx, span, timeout).await
      }
    };

    if matches!(response, Err(ReadError::Connection(_) | ReadError::Read(_))) {
      self.ctx = None;
    }

    response
  }

  async fn simple_read_impl_connected(
    ctx: &mut Context,
    span: SimpleSpan,
    timeout: futures_time::time::Duration,
  ) -> Result<Response, ReadError> {
    

    match ctx
      .read_holding_registers(span.address, span.quantity)
      .timeout(timeout)
      .await
    {
      Err(timeout_error) => Err(ReadError::Timeout(timeout_error)),
      Ok(Err(connection_error)) => Err(ReadError::Read(connection_error)),
      Ok(Ok(response)) => Ok(response),
    }
  }
}

fn timeout_to_chrono(
  timeout: futures_time::time::Duration,
) -> chrono::Duration {
  chrono::Duration::milliseconds(timeout.as_millis() as i64)
}

fn timeout_from_chrono(
  timeout: chrono::Duration,
) -> futures_time::time::Duration {
  futures_time::time::Duration::from_millis(timeout.num_milliseconds() as u64)
}

fn backoff_to_chrono(backoff: tokio::time::Duration) -> chrono::Duration {
  chrono::Duration::milliseconds(backoff.as_millis() as i64)
}

fn backoff_from_chrono(backoff: chrono::Duration) -> tokio::time::Duration {
  tokio::time::Duration::from_millis(backoff.num_milliseconds() as u64)
}
