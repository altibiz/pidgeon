use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::connection::*;

// TODO: save responses/errors across completions

// TODO: tuning

// TODO: initial read params from config

// TODO: optimize
// 1. remove cloning as much as possible
// 2. try removing arc mutex on connection
// 3. try spinning

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Request {
  pub destination: Destination,
  pub spans: Vec<Span>,
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
    let receiver = receiver.recv_async().await?;
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
    let receiver = receiver.recv_async().await?;
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
  sender: flume::Sender<ResponseReceiver>,
}

type ResponseSender = flume::Sender<Result<Response, Error>>;
type ResponseReceiver = flume::Receiver<Result<Response, Error>>;
type RequestSender = flume::Sender<Carrier>;
type RequestReceiver = flume::Receiver<Carrier>;

#[derive(Debug, Clone)]
struct Storage {
  sender: ResponseSender,
  receiver: ResponseReceiver,
  request: Request,
  kind: RequestKind,
  partial: Vec<Option<super::connection::Response>>,
}

#[derive(Debug, Clone)]
struct Task {
  connections: HashMap<Destination, Arc<Mutex<Connection>>>,
  receiver: RequestReceiver,
  oneshots: HashMap<Request, Storage>,
  streams: HashMap<Request, Storage>,
  params: Params,
}

impl Task {
  pub fn new(params: Params, receiver: RequestReceiver) -> Self {
    Self {
      connections: HashMap::new(),
      receiver,
      oneshots: HashMap::new(),
      streams: HashMap::new(),
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

      self.filter_requests();

      for storage in self.make_current() {
        let connection = {
          match self.connect(storage).await {
            Some(connection) => connection,
            None => continue,
          }
        };

        if let Some(response) = self.read(storage, connection.clone()).await {
          self.send(storage, response).await;
        }
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
      RequestKind::Oneshot => match self.oneshots.get(&request) {
        Some(Storage { receiver, .. }) => receiver.clone(),
        None => {
          let (sender, receiver) = flume::bounded(1);
          self.oneshots.insert(
            request.clone(),
            Storage {
              sender,
              receiver: receiver.clone(),
              request,
              kind: RequestKind::Oneshot,
              partial: (0..request.spans.len())
                .into_iter()
                .map(|_| None)
                .collect::<Vec<_>>(),
            },
          );
          receiver
        }
      },
      RequestKind::Stream => match self.streams.get_mut(&request) {
        Some(Storage { receiver, .. }) => receiver.clone(),
        None => {
          let (sender, receiver) = flume::unbounded();
          self.streams.insert(
            request.clone(),
            Storage {
              sender,
              receiver: receiver.clone(),
              request,
              kind: RequestKind::Stream,
              partial: (0..request.spans.len())
                .into_iter()
                .map(|_| None)
                .collect::<Vec<_>>(),
            },
          );
          receiver
        }
      },
    };

    if let Err(error) = sender.try_send(receiver) {
      match kind {
        RequestKind::Oneshot => self.oneshots.remove(&request),
        RequestKind::Stream => self.streams.remove(&request),
      };
      tracing::debug! {
        %error,
        "Failed sending back receiver from worker for {:?}",
        request
      }
    }

    Ok(())
  }

  fn filter_requests(&mut self) {
    self.oneshots = self
      .oneshots
      .into_iter()
      .filter(|(_, Storage { receiver, .. })| receiver.receiver_count() <= 1)
      .collect::<HashMap<_, _>>();

    self.streams = self
      .streams
      .into_iter()
      .filter(|(_, Storage { receiver, .. })| receiver.receiver_count() <= 1)
      .collect::<HashMap<_, _>>();
  }

  fn make_current(&self) -> Vec<&Storage> {
    let current = {
      let mut current = Vec::new();
      current.extend(self.oneshots.values());
      current.extend(self.streams.values());
      current
    };

    current
  }

  async fn connect(
    &mut self,
    storage: &Storage,
  ) -> Option<Arc<Mutex<Connection>>> {
    let connection = match self.connections.get(&storage.request.destination) {
      Some(connection) => Some(connection.clone()),
      None => match Connection::connect(storage.request.destination).await {
        Ok(connection) => {
          let connection = Arc::new(Mutex::new(connection));
          self
            .connections
            .insert(storage.request.destination, connection.clone());
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

          match storage.kind {
            RequestKind::Oneshot => self.oneshots.remove(&storage.request),
            RequestKind::Stream => self.streams.remove(&storage.request),
          };

          None
        }
      },
    };

    connection
  }

  async fn read(
    &mut self,
    storage: &Storage,
    connection: Arc<Mutex<Connection>>,
  ) -> Option<Response> {
    let response = {
      let mut connection = connection.clone().lock_owned().await;
      let mut data = Vec::new();
      let mut completed = true;
      for span in storage.request.spans.iter() {
        let read = match (*connection).read(*span, self.params.clone()).await {
          Ok(read) => read,
          Err(_) => {
            completed = false;
            continue;
          }
        };
        data.push(read);
      }

      if completed {
        if let RequestKind::Oneshot = storage.kind {
          self.oneshots.remove(&storage.request);
        }
      }

      data
    };

    Some(response)
  }

  async fn send(&mut self, storage: &Storage, response: Response) {
    if let Err(error) = storage.sender.try_send(Ok(response)) {
      match storage.kind {
        RequestKind::Oneshot => self.oneshots.remove(&storage.request),
        RequestKind::Stream => self.streams.remove(&storage.request),
      };

      tracing::debug! {
        %error,
        "Failed sending response from worker task for {:?}",
        storage.request
      }
    }
  }

  fn tune(&mut self) {}
}
