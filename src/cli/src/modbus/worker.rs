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

type ResponseSender = flume::Sender<Result<Response, Error>>;
type ResponseReceiver = flume::Receiver<Result<Response, Error>>;
type RequestSender =
  flume::Sender<(RequestKind, flume::Sender<ResponseReceiver>)>;
type RequestReceiver =
  flume::Receiver<(RequestKind, flume::Sender<ResponseReceiver>)>;

#[derive(Debug, Clone)]
pub struct Worker {
  sender: RequestSender,
  handle: Arc<Mutex<tokio::task::JoinHandle<()>>>,
}

impl Worker {
  pub fn new() -> Self {
    let params = Params::new(
      chrono::Duration::milliseconds(1000),
      chrono::Duration::milliseconds(50),
      3,
    )
    .unwrap();
    let (sender, receiver) = flume::unbounded();
    let task = Task {
      connections: HashMap::new(),
      receiver,
      streams: HashMap::new(),
      oneshots: HashMap::new(),
      params,
    };
    let handle = tokio::spawn(task.execute());
    Self {
      sender,
      handle: Arc::new(Mutex::new(handle)),
    }
  }
}

#[derive(Clone, Debug)]
enum RequestKind {
  Oneshot(Request),
  Stream(Request),
}

impl Worker {
  pub async fn send(&self, request: Request) -> Result<Response, Error> {
    let (sender, receiver) = flume::bounded(1);
    self
      .sender
      .send_async((RequestKind::Oneshot(request), sender))
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
      .send_async((RequestKind::Stream(request), sender))
      .await;
    let receiver = receiver.recv_async().await?;
    Ok(receiver)
  }
}

#[derive(Debug, Clone)]
struct Task {
  connections: HashMap<Destination, Arc<Mutex<Connection>>>,
  receiver: RequestReceiver,
  oneshots: HashMap<Request, (ResponseSender, ResponseReceiver)>,
  streams: HashMap<Request, (ResponseSender, ResponseReceiver)>,
  params: Params,
}

impl Task {
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

      for (request_kind, sender) in self.make_current() {
        let connection = {
          match self.connect(request_kind.clone(), sender.clone()).await {
            Some(connection) => connection,
            None => continue,
          }
        };

        let response =
          self.read(request_kind.clone(), connection.clone()).await;

        self
          .send(request_kind.clone(), sender.clone(), response.clone())
          .await;
      }

      self.tune();
    }
  }

  fn try_recv(&mut self) -> Result<(), flume::TryRecvError> {
    let (request, sender) = self.receiver.try_recv()?;

    let (_, receiver) = match &request {
      RequestKind::Oneshot(request) => match self.oneshots.get(request) {
        Some(pair) => pair.clone(),
        None => {
          let pair = flume::bounded(1);
          self.oneshots.insert(request.clone(), pair.clone());
          pair
        }
      },
      RequestKind::Stream(request) => match self.streams.get_mut(request) {
        Some(pair) => pair.clone(),
        None => {
          let pair = flume::unbounded();
          self.streams.insert(request.clone(), pair.clone());
          pair
        }
      },
    };

    if let Err(error) = sender.try_send(receiver) {
      match &request {
        RequestKind::Oneshot(request) => self.oneshots.remove(request),
        RequestKind::Stream(request) => self.streams.remove(request),
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
      .filter(|(_, (_, receiver))| receiver.receiver_count() > 1)
      .collect::<HashMap<_, _>>();

    self.streams = self
      .streams
      .into_iter()
      .filter(|(_, (_, receiver))| receiver.receiver_count() > 1)
      .collect::<HashMap<_, _>>();
  }

  fn make_current(&self) -> Vec<(RequestKind, ResponseSender)> {
    let current = {
      let mut current = Vec::new();
      current.extend(self.oneshots.iter().map(|(request, (sender, _))| {
        (RequestKind::Oneshot(request.clone()), sender.clone())
      }));
      current.extend(self.streams.iter().map(|(request, (sender, _))| {
        (RequestKind::Stream(request.clone()), sender.clone())
      }));
      current
    };

    current
  }

  async fn connect(
    &mut self,
    request_kind: RequestKind,
    sender: flume::Sender<Result<Response, Error>>,
  ) -> Option<Arc<Mutex<Connection>>> {
    let request = match &request_kind {
      RequestKind::Oneshot(request) => request,
      RequestKind::Stream(request) => request,
    };

    let connection = match self.connections.get(&request.destination) {
      Some(connection) => Some(connection.clone()),
      None => match Connection::connect(request.destination).await {
        Ok(connection) => {
          let connection = Arc::new(Mutex::new(connection));
          self
            .connections
            .insert(request.destination, connection.clone());
          Some(connection)
        }
        Err(error) => {
          if let Err(error) = sender.try_send(Err(error.into())) {
            tracing::debug! {
              %error,
              "Failed sending connection fail from worker task for {:?}",
              request
            }
          }

          match &request_kind {
            RequestKind::Oneshot(request) => self.oneshots.remove(request),
            RequestKind::Stream(request) => self.streams.remove(request),
          };

          None
        }
      },
    };

    connection
  }

  async fn read(
    &mut self,
    request_kind: RequestKind,
    connection: Arc<Mutex<Connection>>,
  ) -> Response {
    let request = match &request_kind {
      RequestKind::Oneshot(request) => request,
      RequestKind::Stream(request) => request,
    };

    let reponse = {
      let mut connection = connection.clone().lock_owned().await;
      let mut data = Vec::new();
      let mut completed = true;
      for span in request.spans.iter() {
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
        if let RequestKind::Oneshot(request) = &request_kind {
          self.oneshots.remove(request);
        }
      }

      data
    };

    reponse
  }

  async fn send(
    &mut self,
    request_kind: RequestKind,
    sender: flume::Sender<Result<Response, Error>>,
    response: Response,
  ) {
    if let Err(error) = sender.try_send(Ok(response.clone())) {
      let request = match request_kind {
        RequestKind::Oneshot(request) => {
          self.oneshots.remove(&request);
          request
        }
        RequestKind::Stream(request) => {
          self.streams.remove(&request);
          request
        }
      };

      tracing::debug! {
        %error,
        "Failed sending response from worker task for {:?}",
        request
      }
    }
  }

  fn tune(&mut self) {}
}
