use std::net::SocketAddr;

use futures_time::future::FutureExt;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_modbus::{
  client::{Context, Writer},
  prelude::Reader,
  slave::SlaveContext,
  Slave,
};
use tokio_serial::SerialPortBuilderExt;

use super::{record::SimpleRecord, span::SimpleSpan};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum Device {
  Tcp(SocketAddr),
  Rtu { path: String, baud_rate: u32 },
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct Destination {
  pub(crate) device: Device,
  pub(crate) slave: Option<u8>,
}

impl Destination {
  pub(crate) fn slaves_for(
    device: Device,
  ) -> impl Iterator<Item = Destination> {
    let device = device.clone();
    (Slave::min_device().0..Slave::max_device().0).map(move |slave| {
      let device = device.clone();
      Destination {
        device,
        slave: Some(slave),
      }
    })
  }

  pub(crate) fn standalone_for(device: Device) -> Destination {
    Destination {
      device,
      slave: None,
    }
  }
}

pub(crate) type ReadResponse = Vec<u16>;
pub(crate) type WriteResponse = ();

#[derive(Debug)]
pub(crate) struct Connection {
  device: Device,
  ctx: Option<Context>,
}

impl Connection {
  pub(crate) fn new(device: Device) -> Self {
    Self { device, ctx: None }
  }

  pub(crate) async fn ensure_connected(&mut self) -> Result<(), ConnectError> {
    if self.ctx.is_none() {
      let _ = self.reconnect().await?;
    }

    Ok(())
  }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectError {
  #[error("Failed to connect TCP")]
  TcpConnect(#[from] std::io::Error),

  #[error("Failed to connect RTU")]
  RtuConnect(#[from] serialport::Error),

  #[error("Wrong slave number")]
  Slave,
}

impl Connection {
  async fn reconnect(&mut self) -> Result<&mut Context, ConnectError> {
    let ctx = match self.device {
      Device::Tcp(socket) => {
        let stream = TcpStream::connect(socket).await?;
        tokio_modbus::prelude::tcp::attach(stream)
      }
      Device::Rtu { path, baud_rate } => {
        let stream = tokio_serial::new(path, baud_rate).open_native_async()?;
        tokio_modbus::prelude::rtu::attach(stream)
      }
    };

    tracing::trace!("Connected");

    self.ctx = Some(ctx);

    #[allow(clippy::unwrap_used)] // NOTE: we just put it in
    Ok(self.ctx.as_mut().unwrap())
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

#[derive(Debug, Error)]
pub(crate) enum WriteError {
  #[error("Failed connecting")]
  Connection(#[from] ConnectError),

  #[error("Failed reading")]
  Read(std::io::Error),

  #[error("Connection timed out")]
  Timeout(std::io::Error),
}

impl Connection {
  #[tracing::instrument(skip(self), fields(address = ?self.device))]
  pub(crate) async fn read(
    &mut self,
    slave: Option<u8>,
    span: SimpleSpan,
    timeout: chrono::Duration,
  ) -> Result<ReadResponse, ReadError> {
    let response = self
      .simple_read_impl(slave, span, timeout_from_chrono(timeout))
      .await?;

    tracing::trace!("Simple read successful");

    Ok(response)
  }

  #[tracing::instrument(skip(self), fields(address = ?self.device))]
  pub(crate) async fn write(
    &mut self,
    slave: Option<u8>,
    record: SimpleRecord,
    timeout: chrono::Duration,
  ) -> Result<WriteResponse, WriteError> {
    self
      .simple_write_impl(slave, record, timeout_from_chrono(timeout))
      .await?;

    tracing::trace!("Simple read successful");

    Ok(())
  }

  async fn simple_read_impl(
    &mut self,
    slave: Option<u8>,
    span: SimpleSpan,
    timeout: futures_time::time::Duration,
  ) -> Result<ReadResponse, ReadError> {
    let response = match &mut self.ctx {
      Some(ctx) => {
        Self::simple_read_impl_connected(ctx, slave, span, timeout).await
      }
      None => {
        let ctx = self.reconnect().await?;
        Self::simple_read_impl_connected(ctx, slave, span, timeout).await
      }
    };

    if matches!(response, Err(ReadError::Connection(_) | ReadError::Read(_))) {
      self.ctx = None;
    }

    response
  }

  async fn simple_write_impl(
    &mut self,
    slave: Option<u8>,
    record: SimpleRecord,
    timeout: futures_time::time::Duration,
  ) -> Result<WriteResponse, WriteError> {
    let response = match &mut self.ctx {
      Some(ctx) => {
        Self::simple_write_impl_connected(ctx, slave, record, timeout).await
      }
      None => {
        let ctx = self.reconnect().await?;
        Self::simple_write_impl_connected(ctx, slave, record, timeout).await
      }
    };

    if matches!(
      response,
      Err(WriteError::Connection(_) | WriteError::Read(_))
    ) {
      self.ctx = None;
    }

    response
  }

  async fn simple_read_impl_connected(
    ctx: &mut Context,
    slave: Option<u8>,
    span: SimpleSpan,
    timeout: futures_time::time::Duration,
  ) -> Result<ReadResponse, ReadError> {
    if let Some(slave) = slave {
      if slave < Slave::min_device().0 || slave > Slave::max_device().0 {
        return Err(ReadError::Connection(ConnectError::Slave));
      }

      ctx.set_slave(Slave(slave))
    } else {
      ctx.set_slave(Slave::tcp_device())
    }

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

  async fn simple_write_impl_connected(
    ctx: &mut Context,
    slave: Option<u8>,
    record: SimpleRecord,
    timeout: futures_time::time::Duration,
  ) -> Result<WriteResponse, WriteError> {
    if let Some(slave) = slave {
      if slave < Slave::min_device().0 || slave > Slave::max_device().0 {
        return Err(WriteError::Connection(ConnectError::Slave));
      }

      ctx.set_slave(Slave(slave))
    } else {
      ctx.set_slave(Slave::tcp_device())
    }

    match ctx
      .write_multiple_registers(record.address, &record.values)
      .timeout(timeout)
      .await
    {
      Err(timeout_error) => Err(WriteError::Timeout(timeout_error)),
      Ok(Err(connection_error)) => Err(WriteError::Read(connection_error)),
      Ok(Ok(_)) => Ok(()),
    }
  }
}

fn timeout_from_chrono(
  timeout: chrono::Duration,
) -> futures_time::time::Duration {
  futures_time::time::Duration::from_millis(timeout.num_milliseconds() as u64)
}
