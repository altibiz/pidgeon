use std::collections::HashMap;
use std::sync::Arc;

use either::Either;
use tokio::sync::Mutex;

use super::connection::*;
use super::span::SimpleSpan;

// TODO: fix broadcasting

// TODO: save responses/errors across completions

// TODO: tuning

// TODO: initial read params from config

// TODO: optimize
// 1. use bounded channels for streams
// 2. remove cloning as much as possible
// 3. use Arc slices instead of vecs
// 4. try removing arc mutex on connection
// 5. try spinning

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
  handle: Arc<Mutex<tokio::task::JoinHandle<()>>>,
}

impl Worker {
  pub fn new() -> Self {
    let (sender, receiver) = flume::unbounded();
    let params = Params::new(
      chrono::Duration::milliseconds(1000),
      chrono::Duration::milliseconds(50),
      3,
    )
    .unwrap();
    let task = Task::new(params, receiver);
    let handle = tokio::spawn(task.execute());
    Self {
      sender,
      handle: Arc::new(Mutex::new(handle)),
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
  ) -> Result<flume::Receiver<Result<Response, Error>>, flume::RecvError> {
    let (sender, receiver) = flume::bounded(1);
    self
      .sender
      .send_async(Carrier {
        request,
        kind: RequestKind::Stream,
        sender,
      })
      .await;
    Ok(receiver)
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
  kind: RequestKind,
  partial: Partial,
}

#[derive(Debug, Clone)]
struct Task {
  connections: HashMap<Destination, Arc<Mutex<Connection>>>,
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
        if let Err(error) = self.try_recv() {
          match error {
            flume::TryRecvError::Empty => break,
            flume::TryRecvError::Disconnected => return,
          }
        }
      }

      for storage in self.make_current() {
        let connection = {
          match self.connect(&storage).await {
            Some(connection) => {
              self
                .connections
                .insert(storage.request.destination, connection.clone());
              connection
            }
            None => {
              match storage.kind {
                RequestKind::Oneshot => {
                  self.oneshots.retain(|x| x.id != storage.id);
                }
                RequestKind::Stream => {
                  self.streams.retain(|x| x.id != storage.id);
                }
              };
              continue;
            }
          }
        };

        let response = { self.read(&storage, connection.clone()).await };

        match response {
          Either::Left(partial) => match storage.kind {
            RequestKind::Oneshot => {
              if let Some(storage) = self
                .oneshots
                .iter_mut()
                .filter(|x| x.id == storage.id)
                .next()
              {
                storage.partial = partial;
              }
            }
            RequestKind::Stream => {
              if let Some(storage) = self
                .streams
                .iter_mut()
                .filter(|x| x.id == storage.id)
                .next()
              {
                storage.partial = partial;
              }
            }
          },
          Either::Right(response) => match storage.kind {
            RequestKind::Oneshot => {
              if let Err(error) = storage.sender.try_send(Ok(response)) {
                tracing::debug! {
                  %error,
                  "Failed sending oneshot response {:?}",
                  storage.request
                }
              }

              self.oneshots.retain(|x| x.id != storage.id);
            }
            RequestKind::Stream => {
              match storage.sender.try_send(Ok(response)) {
                Ok(()) => {
                  if let Some(storage) = self
                    .streams
                    .iter_mut()
                    .filter(|x| x.id == storage.id)
                    .next()
                  {
                    storage.partial = (0..storage.request.spans.len())
                      .map(|_| None)
                      .collect::<Partial>();
                  }
                }
                Err(_) => {
                  self.streams.retain(|x| x.id != storage.id);
                }
              }
            }
          },
        };
      }

      self.tune();
    }
  }

  fn try_recv(&mut self) -> Result<(), flume::TryRecvError> {
    let Carrier {
      request,
      kind,
      sender,
    } = self.receiver.try_recv()?;

    let receiver = match &kind {
      RequestKind::Oneshot => self.oneshots.push(Storage {
        id: Id::new_v4(),
        sender,
        request,
        kind,
        partial: (0..request.spans.len()).map(|_| None).collect::<Vec<_>>(),
      }),
      RequestKind::Stream => self.oneshots.push(Storage {
        id: Id::new_v4(),
        sender,
        request,
        kind,
        partial: (0..request.spans.len()).map(|_| None).collect::<Vec<_>>(),
      }),
    };

    Ok(())
  }

  fn make_current(&self) -> Vec<Storage> {
    // NOTE: this is a hell of a lot of copying
    let current = {
      let mut current = Vec::new();
      current.extend(self.oneshots.iter().cloned());
      current.extend(self.streams.iter().cloned());
      current
    };

    current
  }

  async fn connect(&self, storage: &Storage) -> Option<Arc<Mutex<Connection>>> {
    let connection = match self.connections.get(&storage.request.destination) {
      Some(connection) => Some(connection.clone()),
      None => match Connection::connect(storage.request.destination).await {
        Ok(connection) => {
          let connection = Arc::new(Mutex::new(connection));
          Some(connection)
        }
        Err(error) => {
          if let Err(error) = storage.sender.try_send(Err(error.into())) {
            tracing::debug! {
              %error,
              "Failed sending connection fail from worker task for {:?}",
              storage.request
            }
          }

          None
        }
      },
    };

    connection
  }

  async fn read(
    &self,
    storage: &Storage,
    connection: Arc<Mutex<Connection>>,
  ) -> Either<Partial, Response> {
    let partial = {
      let mut connection = connection.clone().lock_owned().await;
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
          None => match (*connection).read(span, self.params).await {
            Ok(read) => Some(read),
            Err(_) => None,
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

  fn tune(&mut self) {}
}
