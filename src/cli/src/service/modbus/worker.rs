use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::IndexMut;
use std::sync::Arc;

use either::Either;
use futures::Stream;
use futures_time::future::FutureExt;
use tokio::sync::Mutex;

use super::connection::*;
use super::span::{SimpleSpan, Span};

// OPTIMIZE: remove copying when reading
// OPTIMIZE: check bounded channel length - maybe config?

#[derive(Debug, Clone)]
pub(crate) struct SpanResponse {
  pub(crate) span: super::connection::Response,
  pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
}

pub(crate) type Response = Vec<SpanResponse>;

#[derive(Debug, Clone)]
pub(crate) struct Worker {
  sender: RequestSender,
  handle: Arc<Mutex<Option<TaskHandle>>>,
  termination_timeout: futures_time::time::Duration,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SendError {
  #[error("Failed to connect")]
  FailedToConnect(#[from] ConnectError),

  #[error("Channel was disconnected before the request could be finished")]
  ChannelDisconnected(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StreamError {
  #[error("Channel was disconnected before the request could be finished")]
  ChannelDisconnected(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TerminateError {
  #[error("Channel was disconnected before the request could be finished")]
  ChannelDisconnected(anyhow::Error),

  #[error("Termination timed out")]
  Timeout(anyhow::Error),

  #[error("Failed joining inner handle")]
  Join(anyhow::Error),
}

impl Worker {
  pub(crate) fn new(
    read_timeout: chrono::Duration,
    termination_timeout: chrono::Duration,
  ) -> Self {
    let (sender, receiver) = flume::unbounded();
    let task = Task::new(read_timeout, receiver);
    let handle = tokio::spawn(task.execute());
    Self {
      sender,
      handle: Arc::new(Mutex::new(Some(handle))),
      termination_timeout: futures_time::time::Duration::from_millis(
        termination_timeout.num_milliseconds() as u64,
      ),
    }
  }
}

impl Worker {
  pub(crate) async fn send<
    TSpan: Span,
    TIntoIterator: IntoIterator<Item = TSpan>,
  >(
    &self,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<Response, SendError> {
    let (sender, receiver) = flume::bounded(1);
    if let Err(error) = self
      .sender
      .send_async(TaskRequest::Carrier(Carrier::new(
        destination,
        spans,
        RequestKind::Oneshot,
        sender,
      )))
      .await
    {
      return Err(SendError::ChannelDisconnected(error.into()));
    };
    let response = match receiver.recv_async().await {
      Ok(response) => response,
      Err(error) => return Err(SendError::ChannelDisconnected(error.into())),
    }?;

    Ok(response)
  }

  pub(crate) async fn stream<
    TSpan: Span,
    TIntoIterator: IntoIterator<Item = TSpan>,
  >(
    &self,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<
    impl Stream<Item = Result<Response, SendError>> + Send + Sync,
    StreamError,
  > {
    let (sender, receiver) = flume::bounded(1024);
    if let Err(error) = self
      .sender
      .send_async(TaskRequest::Carrier(Carrier::new(
        destination,
        spans,
        RequestKind::Stream,
        sender,
      )))
      .await
    {
      return Err(StreamError::ChannelDisconnected(error.into()));
    };
    let stream = receiver.into_stream();
    Ok(stream)
  }

  pub(crate) async fn terminate(&self) -> Result<(), TerminateError> {
    let result = self.sender.send_async(TaskRequest::Terminate).await;

    let handle = {
      let mut handle = self.handle.clone().lock_owned().await;
      (*handle).take()
    };
    if let Some(handle) = handle {
      let abort_handle = handle.abort_handle();
      match handle.timeout(self.termination_timeout).await {
        Ok(Ok(_)) => {}
        Err(error) => {
          abort_handle.abort();
          return Err(TerminateError::Timeout(error.into()));
        }
        Ok(Err(error)) => {
          abort_handle.abort();
          return Err(TerminateError::Join(error.into()));
        }
      };
    }

    result.map_err(|error| TerminateError::ChannelDisconnected(error.into()))
  }
}

type TaskHandle = tokio::task::JoinHandle<()>;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct SimpleRequest {
  destination: Destination,
  spans: Vec<SimpleSpan>,
}

#[derive(Clone, Debug)]
enum RequestKind {
  Oneshot,
  Stream,
}

type SimpleSpans = Vec<SimpleSpan>;

#[derive(Clone, Debug)]
struct Carrier {
  destination: Destination,
  spans: SimpleSpans,
  kind: RequestKind,
  sender: ResponseSender,
}

#[derive(Clone, Debug)]
enum TaskRequest {
  Carrier(Carrier),
  Terminate,
}

impl Carrier {
  fn new<TSpan: Span, TIntoIterator: IntoIterator<Item = TSpan>>(
    destination: Destination,
    spans: TIntoIterator,
    kind: RequestKind,
    sender: ResponseSender,
  ) -> Self {
    Self {
      destination,
      spans: spans
        .into_iter()
        .map(|span| SimpleSpan {
          address: span.address(),
          quantity: span.quantity(),
        })
        .collect::<Vec<_>>(),
      kind,
      sender,
    }
  }
}

type RequestSender = flume::Sender<TaskRequest>;
type RequestReceiver = flume::Receiver<TaskRequest>;
type ResponseSender = flume::Sender<Result<Response, SendError>>;

type Partial = Vec<Option<SpanResponse>>;
type Id = uuid::Uuid;

#[derive(Debug, Clone)]
struct Storage {
  id: Id,
  sender: ResponseSender,
  destination: Destination,
  spans: SimpleSpans,
  partial: Partial,
}

#[derive(Debug)]
struct Task {
  connections: HashMap<SocketAddr, Connection>,
  receiver: RequestReceiver,
  oneshots: Vec<Storage>,
  streams: Vec<Storage>,
  terminate: bool,
  read_timeout: chrono::Duration,
}

impl Task {
  pub(crate) fn new(
    read_timeout: chrono::Duration,
    receiver: RequestReceiver,
  ) -> Self {
    Self {
      connections: HashMap::new(),
      receiver,
      oneshots: Vec::new(),
      streams: Vec::new(),
      terminate: false,
      read_timeout,
    }
  }

  pub(crate) async fn execute(mut self) {
    loop {
      if self.oneshots.is_empty() && self.streams.is_empty() {
        if let Err(error) = self.recv_async_new_request().await {
          match error {
            flume::RecvError::Disconnected => return,
          }
        }
      }

      loop {
        if let Err(error) = self.try_recv_new_request() {
          match error {
            flume::TryRecvError::Empty => break,
            flume::TryRecvError::Disconnected => return,
          }
        }
      }

      let mut metrics = Metrics::new();

      let mut oneshots_to_remove = Vec::new();
      for index in 0..self.oneshots.len() {
        let oneshot = self.oneshots.index_mut(index);
        if oneshot.sender.is_disconnected() {
          oneshots_to_remove.push(oneshot.id);
          continue;
        }

        let connection = match Self::attempt_connection(
          &mut self.connections,
          oneshot,
        )
        .await
        {
          ConnectionAttempt::Existing(connection) => connection,
          ConnectionAttempt::New(connection) => self
            .connections
            .entry(oneshot.destination.address)
            .or_insert(connection),
          ConnectionAttempt::Fail => {
            oneshots_to_remove.push(oneshot.id);
            continue;
          }
        };

        match Self::read(oneshot, &mut metrics, connection, self.read_timeout)
          .await
        {
          Either::Left(partial) => {
            oneshot.partial = partial;
          }
          Either::Right(response) => {
            if let Err(error) = oneshot.sender.try_send(Ok(response)) {
              // NOTE: error -> trace because this should fail when we already cancelled the future from caller
              tracing::trace!(
                "Failed sending oneshot response to {:?} {}",
                oneshot.destination,
                error,
              )
            }

            oneshots_to_remove.push(oneshot.id);
          }
        };
      }
      self.oneshots.retain(|oneshot| {
        !oneshots_to_remove.iter().any(|id| *id == oneshot.id)
      });

      tracing::trace!(
        "Removed oneshots {:?} - retained {:?}",
        oneshots_to_remove,
        self
          .oneshots
          .iter()
          .map(|oneshot| oneshot.id)
          .collect::<Vec<_>>()
      );

      if self.terminate {
        if !self.streams.is_empty() {
          self.streams = Vec::new();
        }
      } else {
        let mut streams_to_remove = Vec::new();
        for index in 0..self.streams.len() {
          let stream = self.streams.index_mut(index);
          if stream.sender.is_disconnected() {
            streams_to_remove.push(stream.id);
          }

          let connection =
            match Self::attempt_connection(&mut self.connections, stream).await
            {
              ConnectionAttempt::Existing(connection) => connection,
              ConnectionAttempt::New(connection) => self
                .connections
                .entry(stream.destination.address)
                .or_insert(connection),
              ConnectionAttempt::Fail => {
                oneshots_to_remove.push(stream.id);
                continue;
              }
            };

          match Self::read(stream, &mut metrics, connection, self.read_timeout)
            .await
          {
            Either::Left(partial) => {
              stream.partial = partial;
            }
            Either::Right(response) => {
              match stream.sender.try_send(Ok(response)) {
                Ok(()) => {
                  stream.partial = vec![None; stream.spans.len()];
                }
                Err(error) => {
                  // NOTE: error -> trace because this should fail when we already cancelled the future from caller
                  tracing::trace!(
                    "Failed sending stream response to {:?} {}",
                    stream.destination,
                    error,
                  );

                  streams_to_remove.push(stream.id);
                }
              }
            }
          };
        }
        self.streams.retain(|stream| {
          !streams_to_remove.iter().any(|id| *id == stream.id)
        });

        tracing::trace!(
          "Removed streams {:?} - retained {:?}",
          streams_to_remove,
          self
            .streams
            .iter()
            .map(|stream| stream.id)
            .collect::<Vec<_>>()
        );
      }

      if !self.terminate {
        tracing::trace!("{:#?}", metrics);
      }
    }
  }

  #[tracing::instrument(skip_all)]
  fn try_recv_new_request(&mut self) -> Result<(), flume::TryRecvError> {
    match self.receiver.try_recv()? {
      TaskRequest::Carrier(carrier) => {
        if !self.terminate {
          self.add_new_request(carrier);
        }
      }
      TaskRequest::Terminate => {
        self.terminate = true;
        self.streams = Vec::new();
        tracing::trace!(
          "Terminating {:?}",
          self
            .oneshots
            .first()
            .map(|oneshot| oneshot.destination.address)
        );
      }
    }

    Ok(())
  }

  #[tracing::instrument(skip_all)]
  async fn recv_async_new_request(&mut self) -> Result<(), flume::RecvError> {
    match self.receiver.recv_async().await? {
      TaskRequest::Carrier(carrier) => {
        if !self.terminate {
          self.add_new_request(carrier);
        }
      }
      TaskRequest::Terminate => {
        self.terminate = true;
        self.streams = Vec::new();
        tracing::trace!(
          "Terminating {:?}",
          self
            .oneshots
            .first()
            .map(|oneshot| oneshot.destination.address)
        );
      }
    }

    Ok(())
  }

  #[tracing::instrument(skip_all, fields(
    destination = ?carrier.destination,
    kind = ?carrier.kind
  ))]
  fn add_new_request(&mut self, carrier: Carrier) {
    let Carrier {
      destination,
      spans,
      kind,
      sender,
    } = carrier;
    let spans_len = spans.len();
    let storage = Storage {
      id: Id::new_v4(),
      sender,
      destination,
      spans,
      partial: vec![None; spans_len],
    };

    match kind {
      RequestKind::Oneshot => self.oneshots.push(storage),
      RequestKind::Stream => self.streams.push(storage),
    };

    tracing::trace!("Added request");
  }
}

enum ConnectionAttempt<'a> {
  Existing(&'a mut Connection),
  New(Connection),
  Fail,
}

impl Task {
  #[tracing::instrument(skip_all, fields(address = ?storage.destination))]
  async fn attempt_connection<'a>(
    connections: &'a mut HashMap<SocketAddr, Connection>,
    storage: &Storage,
  ) -> ConnectionAttempt<'a> {
    match connections.get_mut(&storage.destination.address) {
      Some(connection) => {
        tracing::trace!("Connected to existing connection");

        ConnectionAttempt::Existing(connection)
      }
      None => {
        let mut connection = Connection::new(storage.destination.address);
        match connection.ensure_connected().await {
          Ok(()) => {
            tracing::trace!("Connected to new connection");

            ConnectionAttempt::New(connection)
          }
          Err(error) => {
            tracing::trace!("Failed connecting");

            if let Err(error) = storage.sender.try_send(Err(error.into())) {
              // NOTE: error -> trace because this should fail when we already cancelled the future from caller
              tracing::trace!(
                "Failed sending connection fail from worker task {}",
                error
              );
            }

            ConnectionAttempt::Fail
          }
        }
      }
    }
  }
}

impl Task {
  #[tracing::instrument(skip_all, fields(address = ?storage.destination))]
  async fn read(
    storage: &Storage,
    metrics: &mut Metrics,
    connection: &mut Connection,
    timeout: chrono::Duration,
  ) -> Either<Partial, Response> {
    let partial = {
      let mut data = Vec::new();
      for (span, partial) in
        storage.spans.iter().cloned().zip(storage.partial.iter())
      {
        let read = match partial {
          Some(partial) => Some(partial.clone()),
          None => {
            if let Err(error) = connection.ensure_connected().await {
              metrics
                .reads
                .entry(storage.destination)
                .or_insert_with(Vec::new)
                .push(ReadMetric {
                  message: format!(
                    "Failed reading span {:?} {:?}",
                    span, &error
                  ),
                  error: true,
                  span,
                  time: None,
                });

              None
            } else {
              let start = chrono::Utc::now();
              let data = (*connection)
                .read(storage.destination.slave, span, timeout)
                .await;
              let end = chrono::Utc::now();

              match data {
                Ok(data) => {
                  metrics
                    .reads
                    .entry(storage.destination)
                    .or_insert_with(Vec::new)
                    .push(ReadMetric {
                      message: format!("Successfully read span {:?}", span),
                      error: true,
                      span,
                      time: Some(end.signed_duration_since(start)),
                    });

                  Some(SpanResponse {
                    span: data,
                    timestamp: chrono::Utc::now(),
                  })
                }
                Err(error) => {
                  metrics
                    .reads
                    .entry(storage.destination)
                    .or_insert_with(Vec::new)
                    .push(ReadMetric {
                      message: format!(
                        "Failed reading span {:?} {:?}",
                        span, &error
                      ),
                      error: true,
                      span,
                      time: Some(end.signed_duration_since(start)),
                    });
                  None
                }
              }
            }
          }
        };

        data.push(read);
      }

      data
    };

    if partial.iter().all(|x| x.is_some()) {
      tracing::trace!("Fully read");
      Either::Right(partial.iter().flatten().cloned().collect::<Vec<_>>())
    } else {
      tracing::trace!("Partially read");
      Either::Left(partial)
    }
  }
}

#[derive(Debug, Clone)]
#[allow(unused)]
struct ReadMetric {
  message: String,
  error: bool,
  span: SimpleSpan,
  time: Option<chrono::Duration>,
}

#[derive(Debug, Clone)]
struct Metrics {
  reads: HashMap<Destination, Vec<ReadMetric>>,
}

impl Metrics {
  fn new() -> Self {
    Self {
      reads: HashMap::new(),
    }
  }
}
