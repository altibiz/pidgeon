use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use futures::StreamExt;
use futures_core::Stream;
use tokio::sync::Mutex;

use super::batch::*;
use super::connection::{Destination, Params};
use super::span::*;
use super::worker::*;

// TODO: worker removal
// TODO: remove cloning

#[derive(Clone, Debug)]
pub struct Registry {
  devices: Arc<Mutex<HashMap<String, Device>>>,
  servers: Arc<Mutex<HashMap<SocketAddr, Server>>>,
  initial_params: Params,
  batch_threshold: usize,
}

pub type Response<TSpan: Span> = Vec<TSpan>;

#[derive(Debug, thiserror::Error)]
pub enum ServerReadError {
  #[error("Connection failed")]
  FailedToConnect(#[from] super::connection::ConnectError),

  #[error("Server failure")]
  ServerFailed(anyhow::Error),

  #[error("Parsing failure")]
  ParsingFailed(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ServerStreamError {
  #[error("Server failure")]
  ServerFailed(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceStreamError {
  #[error("No device in registry for given id")]
  DeviceNotFound(String),

  #[error("Failed streaming from worker")]
  ServerStream(#[from] ServerStreamError),
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceReadError {
  #[error("No device in registry for given id")]
  DeviceNotFound(String),

  #[error("Failed reading from worker")]
  ServerRead(#[from] ServerReadError),
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

impl Registry {
  pub fn new(initial_params: Params, batch_threshold: usize) -> Self {
    Self {
      devices: Arc::new(Mutex::new(HashMap::new())),
      servers: Arc::new(Mutex::new(HashMap::new())),
      initial_params,
      batch_threshold,
    }
  }

  pub async fn bind(&self, id: String, destination: Destination) {
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
    }
  }

  pub async fn stop_from_id(&self, id: &str) {
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
    }

    if let Some(server) = server_to_remove {
      self.stop_from_address(server);
    }
  }

  pub async fn stop_from_destination(&self, destination: Destination) {
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
      ids
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
        .collect::<Vec<_>>()
    };

    {
      let mut servers = self.servers.clone().lock_owned().await;
      servers.retain(|address, _| {
        !servers_to_remove
          .iter()
          .any(|address_to_remove| address_to_remove == address)
      });
    }
  }

  pub async fn stop_from_address(&self, address: SocketAddr) {
    {
      let mut devices = self.devices.clone().lock_owned().await;
      devices.retain(|_, device| device.destination.address != address);
    }

    {
      let mut servers = self.servers.clone().lock_owned().await;
      servers.remove(&address);
    }
  }

  pub async fn read_from_destination<
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
    Ok(response)
  }

  pub async fn stream_from_destination<
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
    Ok(stream)
  }

  pub async fn read_from_id<
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
    Ok(response)
  }

  pub async fn stream_from_id<
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
      let batches = batches.clone();
      Self::parse_worker_result(result, batches, len)
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
        worker: Worker::new(self.initial_params),
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
