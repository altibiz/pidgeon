use std::collections::HashMap;
use std::ops::{Index, IndexMut};
use std::sync::Arc;

use either::Either;
use futures::Stream;

use super::connection::*;
use super::span::SimpleSpan;

// TODO: tuning

// TODO: initial read params from config

// TODO: optimize
// 1. fix notes
// 4. use Arc slices instead of Vecs
// 6. try spinning

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Request {
  pub destination: Destination,
  pub spans: Vec<SimpleSpan>,
}

pub type Response = Vec<super::connection::Response>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Failed to connect")]
  FailedToConnect(#[from] ConnectError),

  #[error("Channel receive failure")]
  FailedChannel(#[from] flume::RecvError),
}

#[derive(Debug, Clone)]
pub struct Worker {
  sender: RequestSender,
  handle: Arc<tokio::task::JoinHandle<()>>,
}

impl Worker {
  pub fn new() -> Self {
    let (sender, receiver) = flume::unbounded();
    let params = Params::new(
      chrono::Duration::milliseconds(1000),
      chrono::Duration::milliseconds(50),
      3,
    );
    let task = Task::new(params, receiver);
    let handle = tokio::spawn(task.execute());
    Self {
      sender,
      handle: Arc::new(handle),
    }
  }
}

impl Worker {
  pub async fn send(&self, request: Request) -> Result<Response, Error> {
    let (sender, receiver) = flume::bounded(1);
    self
      .sender
      .send_async(Carrier {
        request,
        kind: RequestKind::Oneshot,
        sender,
      })
      .await;
    let response = receiver.recv_async().await?;
    response
  }

  pub async fn stream(
    &self,
    request: Request,
  ) -> impl Stream<Item = Result<Response, Error>> {
    // NOTE: check 1024 is okay
    let (sender, receiver) = flume::bounded(1024);
    self
      .sender
      .send_async(Carrier {
        request,
        kind: RequestKind::Stream,
        sender,
      })
      .await;
    receiver.into_stream()
  }
}

#[derive(Clone, Debug)]
enum RequestKind {
  Oneshot,
  Stream,
}

#[derive(Clone, Debug)]
struct Carrier {
  request: Request,
  kind: RequestKind,
  sender: ResponseSender,
}

type ResponseSender = flume::Sender<Result<Response, Error>>;
type ResponseReceiver = flume::Receiver<Result<Response, Error>>;
type RequestSender = flume::Sender<Carrier>;
type RequestReceiver = flume::Receiver<Carrier>;

type Partial = Vec<Option<super::connection::Response>>;
type Id = uuid::Uuid;

#[derive(Debug, Clone)]
struct Storage {
  id: Id,
  sender: ResponseSender,
  request: Request,
  partial: Partial,
}

#[derive(Debug)]
struct Task {
  connections: HashMap<Destination, Connection>,
  receiver: RequestReceiver,
  oneshots: Vec<Storage>,
  streams: Vec<Storage>,
  params: Params,
}

impl Task {
  pub fn new(params: Params, receiver: RequestReceiver) -> Self {
    Self {
      connections: HashMap::new(),
      receiver,
      oneshots: Vec::new(),
      streams: Vec::new(),
      params,
    }
  }

  pub async fn execute(mut self) {
    loop {
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
        let oneshot = self.oneshots.index(index);
        let connection = match Task::attempt_connection(
          &mut self.connections,
          oneshot,
        )
        .await
        {
          ConnectionAttempt::Existing(connection) => connection,
          ConnectionAttempt::New(connection) => self
            .connections
            .entry(oneshot.request.destination)
            .or_insert(connection),
          ConnectionAttempt::Fail => {
            oneshots_to_remove.push(oneshot.id);
            continue;
          }
        };

        match Task::read(oneshot, self.params, &mut metrics, connection).await {
          Either::Left(partial) => {
            self.oneshots.index_mut(index).partial = partial
          }
          Either::Right(response) => {
            if let Err(error) = oneshot.sender.try_send(Ok(response)) {
              tracing::debug! {
                %error,
                "Failed sending oneshot response {:?}",
                oneshot.request
              }
            }

            oneshots_to_remove.push(oneshot.id);
          }
        };
      }
      self.oneshots.retain(|oneshot| {
        !oneshots_to_remove.iter().any(|id| *id == oneshot.id)
      });

      let mut streams_to_remove = Vec::new();
      for index in 0..self.streams.len() {
        let stream = self.streams.index(index);
        let connection =
          match Task::attempt_connection(&mut self.connections, stream).await {
            ConnectionAttempt::Existing(connection) => connection,
            ConnectionAttempt::New(connection) => self
              .connections
              .entry(stream.request.destination)
              .or_insert(connection),
            ConnectionAttempt::Fail => {
              oneshots_to_remove.push(stream.id);
              continue;
            }
          };

        match Task::read(stream, self.params, &mut metrics, connection).await {
          Either::Left(partial) => {
            self.streams.index_mut(index).partial = partial;
          }
          Either::Right(response) => {
            match stream.sender.try_send(Ok(response)) {
              Ok(()) => {
                self.streams.index_mut(index).partial =
                  vec![None; stream.request.spans.len()];
              }
              Err(_) => {
                streams_to_remove.push(stream.id);
              }
            }
          }
        };
      }
      self
        .streams
        .retain(|stream| !streams_to_remove.iter().any(|id| *id == stream.id));

      self.tune(metrics);
    }
  }

  fn try_recv_new_request(&mut self) -> Result<(), flume::TryRecvError> {
    let Carrier {
      request,
      kind,
      sender,
    } = self.receiver.try_recv()?;

    match kind {
      RequestKind::Oneshot => self.oneshots.push(Storage {
        id: Id::new_v4(),
        sender,
        request,
        partial: vec![None; request.spans.len()],
      }),
      RequestKind::Stream => self.oneshots.push(Storage {
        id: Id::new_v4(),
        sender,
        request,
        partial: vec![None; request.spans.len()],
      }),
    };

    Ok(())
  }
}

enum ConnectionAttempt<'a> {
  Existing(&'a mut Connection),
  New(Connection),
  Fail,
}
impl Task {
  async fn attempt_connection<'a>(
    connections: &'a mut HashMap<Destination, Connection>,
    storage: &Storage,
  ) -> ConnectionAttempt<'a> {
    match connections.get_mut(&storage.request.destination) {
      Some(connection) => ConnectionAttempt::Existing(connection),
      None => match Connection::connect(storage.request.destination).await {
        Ok(connection) => ConnectionAttempt::New(connection),
        Err(error) => {
          if let Err(error) = storage.sender.try_send(Err(error.into())) {
            tracing::debug! {
              %error,
              "Failed sending connection fail from worker task for {:?}",
              storage.request
            }
          }

          ConnectionAttempt::Fail
        }
      },
    }
  }
}

impl Task {
  // NOTE: remove the copying here
  async fn read(
    storage: &Storage,
    params: Params,
    metrics: &mut Metrics,
    connection: &mut Connection,
  ) -> Either<Partial, Response> {
    let partial = {
      let mut data = Vec::new();
      for (span, partial) in storage
        .request
        .spans
        .iter()
        .cloned()
        .zip(storage.partial.iter())
      {
        let read = match partial {
          Some(partial) => Some(partial.clone()),
          None => match (*connection).parameterized_read(span, params).await {
            Ok(read) => Some(read),
            Err(mut errors) => {
              metrics
                .errors
                .entry(storage.request.destination)
                .or_insert_with(|| Vec::new())
                .append(&mut errors);
              None
            }
          },
        };

        data.push(read);
      }

      data
    };

    if partial.iter().all(|x| x.is_some()) {
      Either::Right(
        partial
          .iter()
          .cloned()
          .filter_map(std::convert::identity)
          .collect::<Vec<_>>(),
      )
    } else {
      Either::Left(partial)
    }
  }
}

#[derive(Debug)]
struct Metrics {
  errors: HashMap<Destination, Vec<ReadError>>,
}

impl Metrics {
  fn new() -> Self {
    Self {
      errors: HashMap::new(),
    }
  }
}

impl Task {
  fn tune(&mut self, metrics: Metrics) {}
}
