use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::sync::Mutex;

use super::batch::batch_spans;
use super::connection::{Destination, Params};
use super::span::*;
use super::worker::*;

#[derive(Clone, Debug)]
pub struct Registry {
  devices: Arc<Mutex<HashMap<String, Device>>>,
  workers: Arc<Mutex<HashMap<SocketAddr, Worker>>>,
  initial_params: Params,
  batch_threshold: usize,
}

pub type Response<TSpan: Span> = Vec<TSpan>;

#[derive(Debug, thiserror::Error)]
pub enum WorkerReadError {
  #[error("Connection failed")]
  FailedToConnect(#[from] super::connection::ConnectError),

  #[error("Worker failure")]
  WorkerFailed(anyhow::Error),

  #[error("Parsing failure")]
  ParsingFailed(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceReadError {
  #[error("No device in registry for given id")]
  DeviceNotFound(String),

  #[error("Failed reading from worker")]
  WorkerRead(#[from] WorkerReadError),
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
      workers: Arc::new(Mutex::new(HashMap::new())),
      initial_params,
      batch_threshold,
    }
  }

  pub async fn read_from_destination<
    TSpan: Span,
    TSpanParser: Span + SpanParser<TSpan>,
  >(
    &self,
    destination: Destination,
    spans: Vec<TSpanParser>,
  ) -> Result<Response<TSpan>, WorkerReadError> {
    let worker = self.get_worker(destination).await;
    let response = self.read_from_worker(worker, destination, spans).await?;
    Ok(response)
  }

  pub async fn stream_from_destination() {}

  pub async fn read_from_id<
    TSpan: Span,
    TSpanParser: Span + SpanParser<TSpan>,
  >(
    &self,
    id: &str,
    spans: Vec<TSpanParser>,
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

  pub async fn stream_from_id() {}

  async fn read_from_worker<
    TSpan: Span,
    TSpanParser: Span + SpanParser<TSpan>,
  >(
    &self,
    worker: Worker,
    destination: Destination,
    spans: Vec<TSpanParser>,
  ) -> Result<Response<TSpan>, WorkerReadError> {
    let spans_len = spans.len();
    let batches = batch_spans(spans.into_iter(), self.batch_threshold);
    let data = match worker.send(destination, batches.iter()).await {
      Ok(response) => response,
      Err(error) => match error {
        super::worker::Error::FailedToConnect(error) => {
          return Err(WorkerReadError::FailedToConnect(error))
        }
        super::worker::Error::ChannelDisconnected(error) => {
          return Err(WorkerReadError::WorkerFailed(error))
        }
      },
    };

    let mut response = Vec::with_capacity(spans_len);
    for (parser, data) in batches.into_iter().zip(data.into_iter()) {
      let mut parsed = match parser.parse(data) {
        Ok(parsed) => parsed,
        Err(error) => return Err(WorkerReadError::ParsingFailed(error)),
      };
      response.append(&mut parsed.spans);
    }

    Ok(response)
  }

  async fn get_worker(&self, destination: Destination) -> Worker {
    let mut workers = self.workers.clone().lock_owned().await;
    let worker = workers
      .entry(destination.socket)
      .or_insert_with(|| Worker::new(self.initial_params))
      .clone();
    worker
  }

  async fn get_device(&self, id: &str) -> Option<Device> {
    let devices = self.devices.clone().lock_owned().await;
    let device = devices.get(id).cloned();
    device
  }
}
