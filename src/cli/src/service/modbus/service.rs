use std::{collections::HashMap, sync::Arc};

use futures::StreamExt;
use futures_core::Stream;
use tokio::sync::Mutex;

use crate::*;

use super::batch::*;
use super::connection::Destination;
use super::connection::Device;
use super::record::Record;
use super::span::*;
use super::worker::*;

// OPTIMIZE: remove cloning and clone constraints

pub(crate) type ReadResponse<TSpan> = Vec<TSpan>;
pub(crate) type WriteResponse = Vec<chrono::DateTime<chrono::Utc>>;

#[derive(Clone, Debug)]
pub(crate) struct Service {
  devices: Arc<Mutex<HashMap<String, Designation>>>,
  servers: Arc<Mutex<HashMap<Device, Server>>>,
  request_timeout: chrono::Duration,
  batch_threshold: u16,
  termination_timeout: chrono::Duration,
  congestion_backoff: chrono::Duration,
  partial_retries: u32,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerReadError {
  #[error("Connection failed")]
  FailedToConnect(#[from] super::connection::ConnectError),

  #[error("Server failure")]
  ServerFailed(anyhow::Error),

  #[error("Parsing failure")]
  ParsingFailed(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerWriteError {
  #[error("Connection failed")]
  FailedToConnect(#[from] super::connection::ConnectError),

  #[error("Server failure")]
  ServerFailed(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerStreamError {
  #[error("Server failure")]
  ServerFailed(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeviceStreamError {
  #[error("No device in registry for given id")]
  DeviceNotFound(String),

  #[error("Failed streaming from worker")]
  ServerStream(#[from] ServerStreamError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeviceReadError {
  #[error("No device in registry for given id")]
  DeviceNotFound(String),

  #[error("Failed reading from worker")]
  ServerRead(#[from] ServerReadError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeviceWriteError {
  #[error("No device in registry for given id")]
  DeviceNotFound(String),

  #[error("Failed reading from worker")]
  ServerWrite(#[from] ServerWriteError),
}

impl service::Service for Service {
  fn new(config: config::Values) -> Self {
    Self {
      devices: Arc::new(Mutex::new(HashMap::new())),
      servers: Arc::new(Mutex::new(HashMap::new())),
      request_timeout: config.modbus.request_timeout,
      batch_threshold: config.modbus.batch_threshold,
      termination_timeout: config.modbus.termination_timeout,
      congestion_backoff: config.modbus.congestion_backoff,
      partial_retries: config.modbus.partial_retries,
    }
  }
}

impl Service {
  #[tracing::instrument(skip(self))]
  pub(crate) async fn bind(&self, id: String, destination: Destination) {
    let server = self.get_server(destination.clone()).await;
    {
      let mut devices = self.devices.clone().lock_owned().await;
      devices.insert(
        id,
        Designation {
          worker: server.worker,
          destination,
        },
      );

      tracing::trace!("Bound - current ids {:?}", devices.keys());
    }
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn stop_from_id(&self, id: &str) {
    let mut server_to_remove = None;

    {
      let mut devices = self.devices.clone().lock_owned().await;
      let device = devices.remove(id);
      if let Some(removed) = device {
        let should_remove_server = !devices.values().any(|device| {
          device.destination.device == removed.destination.device
        });

        if should_remove_server {
          server_to_remove = Some(removed.destination.device);
        }
      }

      tracing::trace!(
        "Stopped {:?} worker - current ids {:?}",
        id,
        devices.keys()
      );
    }

    if let Some(server) = server_to_remove {
      self.stop_from_address(server).await;
    }
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn stop_from_destination(&self, destination: Destination) {
    let ids = {
      let devices = self.devices.clone().lock_owned().await;
      devices
        .iter()
        .filter(|(_, device)| device.destination == destination)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>()
    };

    let servers_to_remove = {
      let mut devices = self.devices.clone().lock_owned().await;
      let addresses = ids
        .iter()
        .filter_map(|id| {
          devices.remove(id).and_then(|removed| {
            let should_remove_server = !devices.values().any(|designation| {
              designation.destination.device == removed.destination.device
            });

            if should_remove_server {
              Some(removed.destination.device)
            } else {
              None
            }
          })
        })
        .collect::<Vec<_>>();

      tracing::trace!("Removed devices - remaining ids {:?}", devices.keys());

      addresses
    };

    tracing::trace!("Removed {:?} ids", ids);

    let mut removed_servers = Vec::new();
    {
      let mut servers = self.servers.clone().lock_owned().await;
      for address in servers_to_remove {
        let server = servers.remove(&address);
        if let Some(server) = server {
          removed_servers.push(server);
        }
      }

      tracing::trace!(
        "Removed servers - remaining addresses {:?}",
        servers.keys(),
      );
    }

    for server in removed_servers {
      if let Err(error) = server.worker.terminate().await {
        // NOTE: error -> trace because this means it already terminated and disconnected
        tracing::trace!("Failed terminating server worker {}", error)
      }
    }
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn stop_from_address(&self, device: Device) {
    {
      let mut devices = self.devices.clone().lock_owned().await;
      devices.retain(|_, designation| designation.destination.device != device);
      tracing::trace!("Retained devices {:?}", devices.keys());
    }

    let server = {
      let mut servers = self.servers.clone().lock_owned().await;
      servers.remove(&device)
    };

    if let Some(server) = server {
      tracing::trace!("Removed {:?} server", server);

      if let Err(error) = server.worker.terminate().await {
        // NOTE: error -> trace because this means it already terminated and disconnected
        tracing::trace!("Failed terminating server worker {}", error)
      }
    }
  }

  #[tracing::instrument(skip(self, spans))]
  pub(crate) async fn read_from_destination<
    TSpan: Span,
    TSpanParser: Span + SpanParser<TSpan>,
    TIterator: ExactSizeIterator<Item = TSpanParser>,
    TIntoIterator: IntoIterator<Item = TSpanParser, IntoIter = TIterator>,
  >(
    &self,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<ReadResponse<TSpan>, ServerReadError> {
    let server = self.get_server(destination.clone()).await;
    let response = self
      .read_from_worker(server.worker, destination, spans)
      .await?;

    tracing::trace!("Read {:?} spans", response.len());

    Ok(response)
  }

  #[tracing::instrument(skip(self, records))]
  pub(crate) async fn write_to_destination<
    TRecord: Record,
    TIterator: Iterator<Item = TRecord>,
    TIntoIterator: IntoIterator<Item = TRecord, IntoIter = TIterator>,
  >(
    &self,
    destination: Destination,
    records: TIntoIterator,
  ) -> Result<WriteResponse, ServerWriteError> {
    let server = self.get_server(destination.clone()).await;
    let response = self
      .write_to_worker(server.worker, destination, records)
      .await?;

    tracing::trace!("Wrote {:?} records", response.len());

    Ok(response)
  }

  #[tracing::instrument(skip(self, spans))]
  pub(crate) async fn stream_from_destination<
    TSpan: Span,
    TSpanParser: Clone + Span + SpanParser<TSpan>,
    TIterator: ExactSizeIterator<Item = TSpanParser>,
    TIntoIterator: IntoIterator<Item = TSpanParser, IntoIter = TIterator>,
  >(
    &self,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<
    impl Stream<Item = Result<Vec<TSpan>, ServerReadError>>,
    ServerStreamError,
  > {
    let server = self.get_server(destination.clone()).await;
    let stream = self
      .stream_from_worker(server.worker, destination, spans)
      .await?;

    tracing::trace!("Streaming spans");

    Ok(stream)
  }

  #[tracing::instrument(skip(self, spans))]
  pub(crate) async fn read_from_id<
    TSpan: Span,
    TSpanParser: Span + SpanParser<TSpan>,
    TIterator: ExactSizeIterator<Item = TSpanParser>,
    TIntoIterator: IntoIterator<Item = TSpanParser, IntoIter = TIterator>,
  >(
    &self,
    id: &str,
    spans: TIntoIterator,
  ) -> Result<ReadResponse<TSpan>, DeviceReadError> {
    let device = match self.get_device(id).await {
      Some(device) => device,
      None => return Err(DeviceReadError::DeviceNotFound(id.to_string())),
    };
    let response = self
      .read_from_worker(device.worker, device.destination, spans)
      .await?;

    tracing::trace!("Read {:?} spans", response.len());

    Ok(response)
  }

  #[tracing::instrument(skip(self, records))]
  pub(crate) async fn write_to_id<
    TRecord: Record,
    TIterator: Iterator<Item = TRecord>,
    TIntoIterator: IntoIterator<Item = TRecord, IntoIter = TIterator>,
  >(
    &self,
    id: &str,
    records: TIntoIterator,
  ) -> Result<WriteResponse, DeviceWriteError> {
    let device = match self.get_device(id).await {
      Some(device) => device,
      None => return Err(DeviceWriteError::DeviceNotFound(id.to_string())),
    };
    let response = self
      .write_to_worker(device.worker, device.destination, records)
      .await?;

    tracing::trace!("Wrote {:?} records", response.len());

    Ok(response)
  }

  #[tracing::instrument(skip(self, spans))]
  pub(crate) async fn stream_from_id<
    TSpan: Span,
    TSpanParser: Clone + Span + SpanParser<TSpan>,
    TIterator: ExactSizeIterator<Item = TSpanParser>,
    TIntoIterator: IntoIterator<Item = TSpanParser, IntoIter = TIterator>,
  >(
    &self,
    id: &str,
    spans: TIntoIterator,
  ) -> Result<
    impl Stream<Item = Result<Vec<TSpan>, ServerReadError>>,
    DeviceStreamError,
  > {
    let device = match self.get_device(id).await {
      Some(device) => device,
      None => return Err(DeviceStreamError::DeviceNotFound(id.to_string())),
    };
    let stream = self
      .stream_from_worker(device.worker, device.destination, spans)
      .await?;

    tracing::trace!("Streaming spans");

    Ok(stream)
  }

  async fn read_from_worker<
    TSpan: Span,
    TSpanParser: Span + SpanParser<TSpan>,
    TIterator: ExactSizeIterator<Item = TSpanParser>,
    TIntoIterator: IntoIterator<Item = TSpanParser, IntoIter = TIterator>,
  >(
    &self,
    worker: Worker,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<ReadResponse<TSpan>, ServerReadError> {
    let iter = spans.into_iter();
    let len = iter.len();
    let batches = batch_spans(iter, self.batch_threshold);
    let result = worker.read(destination, batches.iter()).await;
    let response = Self::parse_worker_read_response(result, batches, len)?;
    Ok(response)
  }

  async fn write_to_worker<
    TRecord: Record,
    TIterator: Iterator<Item = TRecord>,
    TIntoIterator: IntoIterator<Item = TRecord, IntoIter = TIterator>,
  >(
    &self,
    worker: Worker,
    destination: Destination,
    records: TIntoIterator,
  ) -> Result<WriteResponse, ServerWriteError> {
    let iter = records.into_iter();
    let result = worker.write(destination, iter).await;
    let response = Self::parse_worker_write_response(result)?;
    Ok(response)
  }

  async fn stream_from_worker<
    TSpan: Span,
    TSpanParser: Clone + Span + SpanParser<TSpan>,
    TIterator: ExactSizeIterator<Item = TSpanParser>,
    TIntoIterator: IntoIterator<Item = TSpanParser, IntoIter = TIterator>,
  >(
    &self,
    worker: Worker,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<
    impl Stream<Item = Result<ReadResponse<TSpan>, ServerReadError>>,
    ServerStreamError,
  > {
    let iter = spans.into_iter();
    let len = iter.len();
    let batches = batch_spans(iter, self.batch_threshold);
    let stream = match worker
      .stream(destination, batches.clone().into_iter())
      .await
    {
      Ok(stream) => stream,
      Err(error) => return Err(ServerStreamError::ServerFailed(error.into())),
    };
    let stream = stream.map(move |result| {
      Self::parse_worker_read_response(result, batches.clone(), len)
    });
    Ok(stream)
  }

  fn parse_worker_read_response<
    TSpan: Span,
    TSpanParser: Span + SpanParser<TSpan>,
    TIntoIterator: IntoIterator<Item = Batch<TSpanParser>>,
  >(
    result: Result<super::worker::ReadResponse, super::worker::SendError>,
    batches: TIntoIterator,
    len: usize,
  ) -> Result<ReadResponse<TSpan>, ServerReadError> {
    let data = match result {
      Ok(response) => response,
      Err(error) => match error {
        super::worker::SendError::FailedToConnect(error) => {
          return Err(ServerReadError::FailedToConnect(error))
        }
        super::worker::SendError::ChannelDisconnected(error) => {
          return Err(ServerReadError::ServerFailed(error))
        }
      },
    };

    let mut response = Vec::with_capacity(len);
    for (parser, data) in batches.into_iter().zip(data.into_iter()) {
      let mut parsed =
        match parser.parse_with_timestamp(data.inner, data.timestamp) {
          Ok(parsed) => parsed,
          Err(error) => return Err(ServerReadError::ParsingFailed(error)),
        };
      response.append(&mut parsed.inner);
    }

    Ok(response)
  }

  fn parse_worker_write_response(
    result: Result<super::worker::WriteResponse, super::worker::SendError>,
  ) -> Result<WriteResponse, ServerWriteError> {
    let data = match result {
      Ok(response) => response,
      Err(error) => match error {
        super::worker::SendError::FailedToConnect(error) => {
          return Err(ServerWriteError::FailedToConnect(error))
        }
        super::worker::SendError::ChannelDisconnected(error) => {
          return Err(ServerWriteError::ServerFailed(error))
        }
      },
    };

    Ok(data.iter().map(|entry| entry.timestamp).collect::<Vec<_>>())
  }

  async fn get_server(&self, destination: Destination) -> Server {
    let mut workers = self.servers.clone().lock_owned().await;
    let worker = workers
      .entry(destination.device)
      .or_insert_with(|| Server {
        worker: Worker::new(
          self.request_timeout,
          self.termination_timeout,
          self.congestion_backoff,
          self.partial_retries,
        ),
      })
      .clone();
    worker
  }

  async fn get_device(&self, id: &str) -> Option<Designation> {
    let devices = self.devices.clone().lock_owned().await;
    let device = devices.get(id).cloned();
    device
  }
}

#[derive(Clone, Debug)]
struct Server {
  worker: Worker,
}

#[derive(Clone, Debug)]
struct Designation {
  worker: Worker,
  destination: Destination,
}
