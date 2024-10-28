use std::collections::HashMap;
use std::ops::IndexMut;
use std::sync::Arc;

use either::Either;
use futures::Stream;
use futures_time::future::FutureExt;
use tokio::sync::Mutex;

use super::connection::*;
use super::record::{Record, SimpleRecord};
use super::span::{SimpleSpan, Span};

// NOTE: discovery Read(Custom { kind: Other, error: ExceptionResponse { function: 3, exception: IllegalDataAddress } })
// NOTE: timeout clog Read(Custom { kind: InvalidData, error: \"Invalid response header: expected/request = Header { transaction_id: 0, unit_id: 255 }, actual/response = Header { transaction_id: 0, unit_id: 2 }\" })
// NOTE: timeout Timeout(Custom { kind: TimedOut, error: \"future timed out\" })

// TODO: shorten this thing - 1k lines is insane
// OPTIMIZE: remove copying when reading
// OPTIMIZE: check bounded channel length - maybe config?

#[derive(Debug, Clone)]
pub(crate) struct ReadResponseEntry {
  pub(crate) inner: super::connection::ReadResponse,
  pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub(crate) struct WriteResponseEntry {
  #[allow(unused)]
  pub(crate) inner: super::connection::WriteResponse,
  pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
}

pub(crate) type ReadResponse = Vec<ReadResponseEntry>;
pub(crate) type WriteResponse = Vec<WriteResponseEntry>;

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
    request_timeout: chrono::Duration,
    termination_timeout: chrono::Duration,
    congestion_backoff: chrono::Duration,
    partial_retries: u32,
  ) -> Self {
    let (sender, receiver) = flume::unbounded();
    let task = Task::new(
      request_timeout,
      receiver,
      congestion_backoff,
      partial_retries,
    );
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
  pub(crate) async fn read<
    TSpan: Span,
    TIntoIterator: IntoIterator<Item = TSpan>,
  >(
    &self,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<ReadResponse, SendError> {
    let (sender, receiver) = flume::bounded(1);
    if let Err(error) = self
      .sender
      .send_async(TaskRequest::Read(ReadTaskRequest::new(
        destination,
        spans,
        ReadRequestKind::Read,
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

  pub(crate) async fn write<
    TRecord: Record,
    TIntoIterator: IntoIterator<Item = TRecord>,
  >(
    &self,
    destination: Destination,
    records: TIntoIterator,
  ) -> Result<WriteResponse, SendError> {
    let (sender, receiver) = flume::bounded(1);
    if let Err(error) = self
      .sender
      .send_async(TaskRequest::Write(WriteTaskRequest::new(
        destination,
        records,
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
    impl Stream<Item = Result<ReadResponse, SendError>> + Send + Sync,
    StreamError,
  > {
    let (sender, receiver) = flume::bounded(1024);
    if let Err(error) = self
      .sender
      .send_async(TaskRequest::Read(ReadTaskRequest::new(
        destination,
        spans,
        ReadRequestKind::Stream,
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

#[derive(Clone, Debug)]
enum ReadRequestKind {
  Read,
  Stream,
}

#[derive(Clone, Debug)]
struct ReadTaskRequest {
  destination: Destination,
  spans: Vec<SimpleSpan>,
  kind: ReadRequestKind,
  sender: ReadResponseSender,
}

#[derive(Clone, Debug)]
struct WriteTaskRequest {
  destination: Destination,
  records: Vec<SimpleRecord>,
  sender: WriteResponseSender,
}

#[derive(Clone, Debug)]
enum TaskRequest {
  Read(ReadTaskRequest),
  Write(WriteTaskRequest),
  Terminate,
}

impl ReadTaskRequest {
  fn new<TSpan: Span, TIntoIterator: IntoIterator<Item = TSpan>>(
    destination: Destination,
    spans: TIntoIterator,
    kind: ReadRequestKind,
    sender: ReadResponseSender,
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

impl WriteTaskRequest {
  fn new<TRecord: Record, TIntoIterator: IntoIterator<Item = TRecord>>(
    destination: Destination,
    records: TIntoIterator,
    sender: WriteResponseSender,
  ) -> Self {
    Self {
      destination,
      records: records
        .into_iter()
        .map(|record| SimpleRecord {
          address: record.address(),
          values: record.values().collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>(),
      sender,
    }
  }
}

type RequestSender = flume::Sender<TaskRequest>;
type RequestReceiver = flume::Receiver<TaskRequest>;
type ReadResponseSender = flume::Sender<Result<ReadResponse, SendError>>;
type WriteResponseSender = flume::Sender<Result<WriteResponse, SendError>>;

type Id = uuid::Uuid;

#[derive(Debug, Clone)]
struct ReadPartial {
  spans: Vec<Option<ReadResponseEntry>>,
  retries: u32,
}

#[derive(Debug, Clone)]
struct WritePartial {
  records: Vec<Option<WriteResponseEntry>>,
  retries: u32,
}

#[derive(Debug, Clone)]
struct WriteRequestStorage {
  id: Id,
  sender: WriteResponseSender,
  destination: Destination,
  records: Vec<SimpleRecord>,
  partial: WritePartial,
}

#[derive(Debug, Clone)]
struct ReadRequestStorage {
  id: Id,
  sender: ReadResponseSender,
  destination: Destination,
  spans: Vec<SimpleSpan>,
  partial: ReadPartial,
  generation: u64,
}

#[derive(Debug)]
struct Task {
  connections: HashMap<Device, Connection>,
  receiver: RequestReceiver,
  reads: Vec<ReadRequestStorage>,
  writes: Vec<WriteRequestStorage>,
  streams: Vec<ReadRequestStorage>,
  terminate: bool,
  timeout: chrono::Duration,
  congestion_backoff: tokio::time::Duration,
  partial_retries: u32,
}

impl Task {
  pub(crate) fn new(
    read_timeout: chrono::Duration,
    receiver: RequestReceiver,
    congestion_backoff: chrono::Duration,
    partial_retries: u32,
  ) -> Self {
    Self {
      connections: HashMap::new(),
      receiver,
      writes: Vec::new(),
      reads: Vec::new(),
      streams: Vec::new(),
      terminate: false,
      timeout: read_timeout,
      congestion_backoff: tokio::time::Duration::from_millis(
        congestion_backoff.num_milliseconds() as u64,
      ),
      partial_retries,
    }
  }

  pub(crate) async fn execute(mut self) {
    loop {
      if self.reads.is_empty()
        && self.streams.is_empty()
        && self.writes.is_empty()
      {
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

      self.process_reads(&mut metrics).await;
      self.process_writes(&mut metrics).await;

      if self.terminate {
        if !self.streams.is_empty() {
          self.streams = Vec::new();
        }
      } else if let Some(generation) =
        self.streams.iter().map(|stream| stream.generation).min()
      {
        self.process_streams(&mut metrics, generation).await;
      }

      if !self.terminate {
        tracing::trace!("{:#?}", metrics);
      }
    }
  }

  async fn process_reads(&mut self, metrics: &mut Metrics) {
    let mut reads_to_remove = Vec::new();
    for index in 0..self.reads.len() {
      let read = self.reads.index_mut(index);
      if read.sender.is_disconnected() {
        tracing::trace! {
          "{} read sender got disconnected",
          read.id
        };
        reads_to_remove.push(read.id);
        continue;
      }

      let connection = match Self::attempt_connection(
        &mut self.connections,
        &read.destination,
        Either::Left(&read.sender),
      )
      .await
      {
        ConnectionAttempt::Existing(connection) => connection,
        ConnectionAttempt::New(connection) => self
          .connections
          .entry(read.destination.device.clone())
          .or_insert(connection),
        ConnectionAttempt::Fail => {
          reads_to_remove.push(read.id);
          continue;
        }
      };

      match Self::read(
        read,
        metrics,
        connection,
        self.timeout,
        self.congestion_backoff,
      )
      .await
      {
        Either::Left(partial) => {
          read.partial = partial;
        }
        Either::Right(response) => {
          if let Err(error) = read.sender.try_send(Ok(response)) {
            // NOTE: error -> trace because this should fail when we already cancelled the future from caller
            tracing::trace!(
              "Failed sending read response to {:?} {}",
              read.destination,
              error,
            )
          }

          reads_to_remove.push(read.id);
        }
      };
    }
    self
      .reads
      .retain(|read| !reads_to_remove.iter().any(|id| *id == read.id));

    tracing::trace!(
      "Removed reads {:?} - retained {:?}",
      reads_to_remove,
      self
        .reads
        .iter()
        .map(|read| (read.id, read.destination.slave))
        .collect::<Vec<_>>()
    );
  }

  async fn process_writes(&mut self, metrics: &mut Metrics) {
    let mut writes_to_remove = Vec::new();
    for index in 0..self.writes.len() {
      let write = self.writes.index_mut(index);
      if write.sender.is_disconnected() {
        tracing::trace! {
          "{} write sender got disconnected",
          write.id
        };
        writes_to_remove.push(write.id);
        continue;
      }

      let connection = match Self::attempt_connection(
        &mut self.connections,
        &write.destination,
        Either::Right(&write.sender),
      )
      .await
      {
        ConnectionAttempt::Existing(connection) => connection,
        ConnectionAttempt::New(connection) => self
          .connections
          .entry(write.destination.device.clone())
          .or_insert(connection),
        ConnectionAttempt::Fail => {
          writes_to_remove.push(write.id);
          continue;
        }
      };

      match Self::write(
        write,
        metrics,
        connection,
        self.timeout,
        self.congestion_backoff,
      )
      .await
      {
        Either::Left(partial) => {
          write.partial = partial;
        }
        Either::Right(response) => {
          if let Err(error) = write.sender.try_send(Ok(response)) {
            // NOTE: error -> trace because this should fail when we already cancelled the future from caller
            tracing::trace!(
              "Failed sending write response to {:?} {}",
              write.destination,
              error,
            )
          }

          writes_to_remove.push(write.id);
        }
      };
    }
    self
      .writes
      .retain(|write| !writes_to_remove.iter().any(|id| *id == write.id));

    tracing::trace!(
      "Removed writes {:?} - retained {:?}",
      writes_to_remove,
      self
        .writes
        .iter()
        .map(|write| (write.id, write.destination.slave))
        .collect::<Vec<_>>()
    );
  }

  async fn process_streams(&mut self, metrics: &mut Metrics, generation: u64) {
    let mut streams_to_remove = Vec::new();
    for index in 0..self.streams.len() {
      let stream = self.streams.index_mut(index);
      if stream.sender.is_disconnected() {
        tracing::trace! {
          "{} stream sender got disconnected",
          stream.id
        };
        streams_to_remove.push(stream.id);
        continue;
      }

      if stream.generation != generation {
        continue;
      }

      if stream.partial.retries > self.partial_retries {
        stream.generation = stream.generation.saturating_add(1);
        stream.partial = ReadPartial {
          spans: vec![None; stream.spans.len()],
          retries: 0,
        };
        continue;
      }

      let connection = match Self::attempt_connection(
        &mut self.connections,
        &stream.destination,
        Either::Left(&stream.sender),
      )
      .await
      {
        ConnectionAttempt::Existing(connection) => connection,
        ConnectionAttempt::New(connection) => self
          .connections
          .entry(stream.destination.device.clone())
          .or_insert(connection),
        ConnectionAttempt::Fail => {
          streams_to_remove.push(stream.id);
          continue;
        }
      };

      match Self::read(
        stream,
        metrics,
        connection,
        self.timeout,
        self.congestion_backoff,
      )
      .await
      {
        Either::Left(partial) => {
          stream.partial = partial;
        }
        Either::Right(response) => {
          match stream.sender.try_send(Ok(response)) {
            Ok(()) => {
              stream.partial = ReadPartial {
                spans: vec![None; stream.spans.len()],
                retries: 0,
              };
              stream.generation = stream.generation.saturating_add(1);
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
    self
      .streams
      .retain(|stream| !streams_to_remove.iter().any(|id| *id == stream.id));

    tracing::trace!(
      "Removed streams {:?} - retained {:?}",
      streams_to_remove,
      self
        .streams
        .iter()
        .map(|stream| (stream.id, stream.destination.slave))
        .collect::<Vec<_>>()
    );
  }

  #[tracing::instrument(skip_all)]
  fn try_recv_new_request(&mut self) -> Result<(), flume::TryRecvError> {
    match self.receiver.try_recv()? {
      TaskRequest::Read(request) => {
        if !self.terminate {
          self.add_new_read_request(request);
        }
      }
      TaskRequest::Write(request) => {
        if !self.terminate {
          self.add_new_write_request(request);
        }
      }
      TaskRequest::Terminate => {
        self.terminate = true;
        self.streams = Vec::new();
        tracing::trace!(
          "Terminating {:?}",
          self
            .reads
            .first()
            .map(|read| read.destination.device.clone())
        );
      }
    }

    Ok(())
  }

  #[tracing::instrument(skip_all)]
  async fn recv_async_new_request(&mut self) -> Result<(), flume::RecvError> {
    match self.receiver.recv_async().await? {
      TaskRequest::Read(request) => {
        if !self.terminate {
          self.add_new_read_request(request);
        }
      }
      TaskRequest::Write(request) => {
        if !self.terminate {
          self.add_new_write_request(request);
        }
      }
      TaskRequest::Terminate => {
        self.terminate = true;
        self.streams = Vec::new();
        tracing::trace!(
          "Terminating {:?}",
          self
            .reads
            .first()
            .map(|read| read.destination.device.clone())
        );
      }
    }

    Ok(())
  }

  #[tracing::instrument(skip_all, fields(
    destination = ?request.destination,
    kind = ?request.kind
  ))]
  fn add_new_read_request(&mut self, request: ReadTaskRequest) {
    let ReadTaskRequest {
      destination,
      spans,
      kind,
      sender,
    } = request;
    let spans_len = spans.len();
    let storage = ReadRequestStorage {
      id: Id::new_v4(),
      sender,
      destination,
      spans,
      partial: ReadPartial {
        spans: vec![None; spans_len],
        retries: 0,
      },
      generation: self
        .streams
        .iter()
        .map(|stream| stream.generation)
        .max()
        .unwrap_or(0),
    };

    match kind {
      ReadRequestKind::Read => {
        let index = self
          .reads
          .binary_search_by(|r| {
            r.destination.slave.cmp(&storage.destination.slave)
          })
          .unwrap_or_else(|i| i);
        self.reads.insert(index, storage);
      }
      ReadRequestKind::Stream => {
        let index = self
          .streams
          .binary_search_by(|s| {
            s.destination.slave.cmp(&storage.destination.slave)
          })
          .unwrap_or_else(|i| i);
        self.streams.insert(index, storage);
      }
    }

    tracing::trace!("Added read request");
  }

  #[tracing::instrument(skip_all, fields(
    destination = ?request.destination,
  ))]
  fn add_new_write_request(&mut self, request: WriteTaskRequest) {
    let WriteTaskRequest {
      destination,
      records,
      sender,
    } = request;
    let records_len = records.len();
    let storage = WriteRequestStorage {
      id: Id::new_v4(),
      sender,
      destination,
      records,
      partial: WritePartial {
        records: vec![None; records_len],
        retries: 0,
      },
    };

    let index = self
      .writes
      .binary_search_by(|w| w.destination.slave.cmp(&storage.destination.slave))
      .unwrap_or_else(|i| i);

    self.writes.insert(index, storage);

    tracing::trace!("Added write request");
  }
}

enum ConnectionAttempt<'a> {
  Existing(&'a mut Connection),
  New(Connection),
  Fail,
}

impl Task {
  #[tracing::instrument(skip_all, fields(address = ?destination))]
  async fn attempt_connection<'a>(
    connections: &'a mut HashMap<Device, Connection>,
    destination: &Destination,
    sender: Either<&ReadResponseSender, &WriteResponseSender>,
  ) -> ConnectionAttempt<'a> {
    match connections.get_mut(&destination.device) {
      Some(connection) => {
        tracing::trace!("Connected to existing connection");

        ConnectionAttempt::Existing(connection)
      }
      None => {
        let mut connection = Connection::new(destination.device.clone());
        match connection.ensure_connected(destination.slave).await {
          Ok(()) => {
            tracing::trace!("Connected to new connection");

            ConnectionAttempt::New(connection)
          }
          Err(error) => {
            tracing::trace!("Failed connecting");

            match sender {
              Either::Left(sender) => {
                if let Err(error) = sender.try_send(Err(error.into())) {
                  // NOTE: error -> trace because this should fail when we already cancelled the future from caller
                  tracing::trace!(
                    "Failed sending connection fail from worker task {}",
                    error
                  );
                }
              }
              Either::Right(sender) => {
                if let Err(error) = sender.try_send(Err(error.into())) {
                  // NOTE: error -> trace because this should fail when we already cancelled the future from caller
                  tracing::trace!(
                    "Failed sending connection fail from worker task {}",
                    error
                  );
                }
              }
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
    storage: &ReadRequestStorage,
    metrics: &mut Metrics,
    connection: &mut Connection,
    timeout: chrono::Duration,
    congestion_backoff: tokio::time::Duration,
  ) -> Either<ReadPartial, ReadResponse> {
    let partial = {
      let mut data = Vec::new();
      for (span, partial) in storage
        .spans
        .iter()
        .cloned()
        .zip(storage.partial.spans.iter())
      {
        let read = match partial {
          Some(partial) => Some(partial.clone()),
          None => {
            if let Err(error) =
              connection.ensure_connected(storage.destination.slave).await
            {
              metrics
                .reads
                .entry(storage.destination.clone())
                .or_default()
                .push(ReadMetric {
                  message: format!(
                    "Failed connecting span {:?} {:?}",
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
                    .entry(storage.destination.clone())
                    .or_default()
                    .push(ReadMetric {
                      message: format!("Successfully read span {:?}", span),
                      error: false,
                      span,
                      time: Some(end.signed_duration_since(start)),
                    });

                  Some(ReadResponseEntry {
                    inner: data,
                    timestamp: chrono::Utc::now(),
                  })
                }
                Err(error) => {
                  metrics
                    .reads
                    .entry(storage.destination.clone())
                    .or_default()
                    .push(ReadMetric {
                      message: format!(
                        "Failed reading span {:?} {:?}",
                        span, &error
                      ),
                      error: true,
                      span,
                      time: Some(end.signed_duration_since(start)),
                    });

                  if let ReadError::Read(io_error) = error {
                    if io_error.kind() == std::io::ErrorKind::InvalidData {
                      tokio::time::sleep(congestion_backoff).await;
                    }
                  };

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
      Either::Left(ReadPartial {
        spans: partial,
        retries: storage.partial.retries.saturating_add(1),
      })
    }
  }

  #[tracing::instrument(skip_all, fields(address = ?storage.destination))]
  async fn write(
    storage: &WriteRequestStorage,
    metrics: &mut Metrics,
    connection: &mut Connection,
    timeout: chrono::Duration,
    congestion_backoff: tokio::time::Duration,
  ) -> Either<WritePartial, WriteResponse> {
    let partial = {
      let mut data = Vec::new();
      for (record, partial) in storage
        .records
        .iter()
        .cloned()
        .zip(storage.partial.records.iter())
      {
        let read = match partial {
          Some(partial) => Some(partial.clone()),
          None => {
            if let Err(error) =
              connection.ensure_connected(storage.destination.slave).await
            {
              metrics
                .writes
                .entry(storage.destination.clone())
                .or_default()
                .push(WriteMetric {
                  message: format!(
                    "Failed connecting record {:?} {:?}",
                    record, &error
                  ),
                  error: true,
                  record,
                  time: None,
                });

              None
            } else {
              let start = chrono::Utc::now();
              let data = (*connection)
                .write(storage.destination.slave, record.clone(), timeout)
                .await;
              let end = chrono::Utc::now();

              match data {
                Ok(data) => {
                  metrics
                    .writes
                    .entry(storage.destination.clone())
                    .or_default()
                    .push(WriteMetric {
                      message: format!("Successfully read span {:?}", record),
                      error: false,
                      record,
                      time: Some(end.signed_duration_since(start)),
                    });

                  Some(WriteResponseEntry {
                    inner: data,
                    timestamp: chrono::Utc::now(),
                  })
                }
                Err(error) => {
                  metrics
                    .writes
                    .entry(storage.destination.clone())
                    .or_default()
                    .push(WriteMetric {
                      message: format!(
                        "Failed reading span {:?} {:?}",
                        record, &error
                      ),
                      error: true,
                      record,
                      time: Some(end.signed_duration_since(start)),
                    });

                  if let WriteError::Read(io_error) = error {
                    if io_error.kind() == std::io::ErrorKind::InvalidData {
                      tokio::time::sleep(congestion_backoff).await;
                    }
                  };

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
      Either::Left(WritePartial {
        records: partial,
        retries: storage.partial.retries.saturating_add(1),
      })
    }
  }
}

#[derive(Debug, Clone)]
#[allow(unused)]
struct WriteMetric {
  message: String,
  error: bool,
  record: SimpleRecord,
  time: Option<chrono::Duration>,
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
  writes: HashMap<Destination, Vec<WriteMetric>>,
}

impl Metrics {
  fn new() -> Self {
    Self {
      reads: HashMap::new(),
      writes: HashMap::new(),
    }
  }
}
