use std::net::SocketAddr;

use futures_time::future::FutureExt;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_modbus::{client::Context, prelude::Reader, Slave};

use super::span::*;

#[derive(Debug)]
pub struct Connection {
  ctx: Context,
}

impl Connection {
  pub async fn connect(socket: SocketAddr) -> Result<Self, std::io::Error> {
    let stream = TcpStream::connect(socket).await?;
    let ctx = tokio_modbus::prelude::tcp::attach(stream);
    Ok(Self { ctx })
  }

  pub async fn connect_slave(
    socket: SocketAddr,
    slave: Slave,
  ) -> Result<Self, std::io::Error> {
    let stream = TcpStream::connect(socket).await?;
    let ctx = tokio_modbus::prelude::rtu::attach_slave(stream, slave);
    Ok(Self { ctx })
  }
}

#[derive(Copy, Clone, Debug)]
pub struct ConnectionReadParams {
  timeout: futures_time::time::Duration,
  backoff: tokio::time::Duration,
  retries: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionReadParamsError {
  #[error("Failed converting timeout")]
  TimeoutConversion(#[from] std::num::TryFromIntError),

  #[error("Failed converting backoff")]
  BackoffConversoin(#[from] chrono::OutOfRangeError),
}

impl ConnectionReadParams {
  pub fn new(
    timeout: chrono::Duration,
    backoff: chrono::Duration,
    retries: usize,
  ) -> Result<Self, ConnectionReadParamsError> {
    let timeout: futures_time::time::Duration =
      futures_time::time::Duration::from_millis(
        timeout.num_milliseconds().try_into()?,
      );
    let backoff: std::time::Duration = backoff.to_std()?;
    Ok(Self {
      timeout,
      backoff,
      retries,
    })
  }
}

#[derive(Debug, Error)]
pub enum ConnectionReadError {
  #[error("Failed connecting to device")]
  Connection(#[from] std::io::Error),

  #[error("Failed to parse response")]
  Parse,
}

impl Connection {
  pub async fn read_spans<
    TSpan: Span,
    TSpanParser: SpanParser<TSpan>,
    TIntoIterator,
  >(
    &mut self,
    spans: TIntoIterator,
    params: ConnectionReadParams,
  ) -> Vec<Result<TSpan, ConnectionReadError>>
  where
    for<'a> &'a TIntoIterator: IntoIterator<Item = &'a TSpanParser>,
  {
    let mut results = Vec::new();
    let backoff = params.backoff;
    for span in spans.into_iter() {
      let parsed = self.read_span(span, params).await;
      results.push(parsed);
      tokio::time::sleep(backoff).await;
    }
    results
  }

  pub async fn read_span<TSpan: Span, TSpanParser: SpanParser<TSpan>>(
    &mut self,
    register: &TSpanParser,
    params: ConnectionReadParams,
  ) -> Result<TSpan, ConnectionReadError> {
    fn flatten_result<T, E1, E2>(
      result: Result<Result<T, E1>, E2>,
    ) -> Result<T, E1>
    where
      E1: From<E2>,
    {
      result?
    }

    let data = {
      let address = register.address();
      let quantity = register.quantity();
      let timeout = params.timeout;
      let backoff = params.backoff;
      let retries = params.retries;
      let mut retried = 0;
      let mut result = flatten_result(
        self
          .ctx
          .read_holding_registers(address, quantity)
          .timeout(timeout)
          .await,
      );
      while result.is_err() && retried != retries {
        tokio::time::sleep(backoff).await;
        result = flatten_result(
          self
            .ctx
            .read_holding_registers(address, quantity)
            .timeout(timeout)
            .await,
        );
        retried = retried + 1;
      }
      result
    }?;
    let parsed = register.parse(data.iter().cloned());
    parsed.ok_or_else(|| ConnectionReadError::Parse)
  }
}
