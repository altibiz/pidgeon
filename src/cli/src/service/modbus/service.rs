use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use futures::StreamExt;
use futures_core::Stream;
use tokio::sync::Mutex;

use crate::*;

use super::batch::*;
use super::connection::{Destination, Params};
use super::span::*;
use super::worker::*;

// OPTIMIZE: remove cloning and clone constraints

pub(crate) type Response<TSpan> = Vec<TSpan>;

#[derive(Clone, Debug)]
pub(crate) struct Service {
  devices: Arc<Mutex<HashMap<String, Device>>>,
  servers: Arc<Mutex<HashMap<SocketAddr, Server>>>,
  initial_params: Params,
  batch_threshold: u16,
  termination_timeout: chrono::Duration,
  metric_history_size: usize,
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

impl service::Service for Service {
  fn new(config: config::Values) -> Self {
    Self {
      devices: Arc::new(Mutex::new(HashMap::new())),
      servers: Arc::new(Mutex::new(HashMap::new())),
      initial_params: Params::new(
        config.modbus.initial_timeout,
        config.modbus.initial_backoff,
        config.modbus.initial_retries,
      ),
      batch_threshold: config.modbus.batch_threshold,
      termination_timeout: config.modbus.termination_timeout,
      metric_history_size: config.modbus.metric_history_size,
    }
  }
}

impl Service {
  #[tracing::instrument(skip(self))]
  pub(crate) async fn bind(&self, id: String, destination: Destination) {
    let server = self.get_server(destination).await;
    {
      let mut devices = self.devices.clone().lock_owned().await;
      devices.insert(
        id,
        Device {
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
          device.destination.address == removed.destination.address
        });

        if should_remove_server {
          server_to_remove = Some(removed.destination.address);
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
            let should_remove_server = !devices.values().any(|device| {
              device.destination.address == removed.destination.address
            });

            if should_remove_server {
              Some(removed.destination.address)
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
  pub(crate) async fn stop_from_address(&self, address: SocketAddr) {
    {
      let mut devices = self.devices.clone().lock_owned().await;
      devices.retain(|_, device| device.destination.address != address);
      tracing::trace!("Retained devices {:?}", devices.keys());
    }

    let server = {
      let mut servers = self.servers.clone().lock_owned().await;
      servers.remove(&address)
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
  ) -> Result<Response<TSpan>, ServerReadError> {
    let server = self.get_server(destination).await;
    let response = self
      .read_from_worker(server.worker, destination, spans)
      .await?;

    tracing::trace!("Read {:?} spans", response.len());

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
    let server = self.get_server(destination).await;
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
  ) -> Result<Response<TSpan>, DeviceReadError> {
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
  ) -> Result<Response<TSpan>, ServerReadError> {
    let iter = spans.into_iter();
    let len = iter.len();
    let batches = batch_spans(iter, self.batch_threshold);
    let result = worker.send(destination, batches.iter()).await;
    let response = Self::parse_worker_result(result, batches, len)?;
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
    impl Stream<Item = Result<Response<TSpan>, ServerReadError>>,
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
      Self::parse_worker_result(result, batches.clone(), len)
    });
    Ok(stream)
  }

  fn parse_worker_result<
    TSpan: Span,
    TSpanParser: Span + SpanParser<TSpan>,
    TIntoIterator: IntoIterator<Item = Batch<TSpanParser>>,
  >(
    result: Result<super::worker::Response, super::worker::SendError>,
    batches: TIntoIterator,
    len: usize,
  ) -> Result<Response<TSpan>, ServerReadError> {
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
      let mut parsed = match parser.parse(data) {
        Ok(parsed) => parsed,
        Err(error) => return Err(ServerReadError::ParsingFailed(error)),
      };
      response.append(&mut parsed.spans);
    }

    Ok(response)
  }

  async fn get_server(&self, destination: Destination) -> Server {
    let mut workers = self.servers.clone().lock_owned().await;
    let worker = workers
      .entry(destination.address)
      .or_insert_with(|| Server {
        worker: Worker::new(
          self.initial_params,
          self.termination_timeout,
          self.metric_history_size,
        ),
        address: destination.address,
      })
      .clone();
    worker
  }

  async fn get_device(&self, id: &str) -> Option<Device> {
    let devices = self.devices.clone().lock_owned().await;
    let device = devices.get(id).cloned();
    device
  }
}

#[derive(Clone, Debug)]
struct Server {
  worker: Worker,
  address: SocketAddr,
}

#[derive(Clone, Debug)]
struct Device {
  worker: Worker,
  destination: Destination,
}
