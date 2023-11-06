use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::conn::*;

// TODO: trace server info
// TODO: maybe spinning is better for all this?
// TODO: try removing arc mutex on connection
// TODO: parameter tuning
// TODO: optimize

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Request {
  pub socket: SocketAddr,
  pub slave: Option<u8>,
  pub spans: Vec<(tokio_modbus::Address, tokio_modbus::Quantity)>,
}

#[derive(Debug, Clone)]
pub struct Response {
  pub spans: Vec<Vec<u16>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Failed to connect")]
  FailedToConnect,

  #[error("Channel receive failure")]
  FailedChannel(#[from] flume::RecvError),
}

#[derive(Debug, Clone)]
pub struct Worker {
  sender: flume::Sender<(
    RequestKind,
    flume::Sender<flume::Receiver<Result<Response, Error>>>,
  )>,
  handle: Arc<Mutex<tokio::task::JoinHandle<()>>>,
}

impl Worker {
  pub fn new() -> Self {
    let params = ConnectionReadParams::new(
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
  connections: HashMap<(SocketAddr, Option<u8>), Arc<Mutex<Connection>>>,
  receiver: flume::Receiver<(
    RequestKind,
    flume::Sender<flume::Receiver<Result<Response, Error>>>,
  )>,
  oneshots: HashMap<
    Request,
    (
      flume::Sender<Result<Response, Error>>,
      flume::Receiver<Result<Response, Error>>,
    ),
  >,
  streams: HashMap<
    Request,
    (
      flume::Sender<Result<Response, Error>>,
      flume::Receiver<Result<Response, Error>>,
    ),
  >,
  params: ConnectionReadParams,
}

impl Task {
  pub async fn execute(mut self) {
    loop {
      loop {
        let (request, sender) = match self.receiver.try_recv() {
          Ok(received) => received,
          Err(error) => match error {
            flume::TryRecvError::Empty => break,
            flume::TryRecvError::Disconnected => return,
          },
        };

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
            "Failed sending back receiver from worker"
          }
        }
      }

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

      for (request_kind, sender) in current {
        let request = match &request_kind {
          RequestKind::Oneshot(request) => request,
          RequestKind::Stream(request) => request,
        };

        let connection = match self
          .connections
          .get(&(request.socket, request.slave))
        {
          Some(connection) => connection.clone(),
          None => {
            match Connection::connect(
              request.socket,
              request.slave.map(|slave| tokio_modbus::Slave(slave)),
            )
            .await
            {
              Ok(connection) => {
                let connection = Arc::new(Mutex::new(connection));
                self
                  .connections
                  .insert((request.socket, request.slave), connection.clone());
                connection
              }
              Err(_) => {
                // NOTE: this logically shouldn't happen
                if let Err(error) = sender.try_send(Err(Error::FailedToConnect))
                {
                  tracing::debug! {
                    %error,
                    "Failed sending response from worker task"
                  }
                }

                match &request_kind {
                  RequestKind::Oneshot(request) => {
                    self.oneshots.remove(request)
                  }
                  RequestKind::Stream(request) => self.streams.remove(request),
                };

                continue;
              }
            }
          }
        };

        let spans = {
          let mut connection = connection.clone().lock_owned().await;
          let mut data = Vec::new();
          for span in request.spans.iter() {
            let read = match (*connection)
              .read(span.0, span.1, self.params.clone())
              .await
            {
              Ok(read) => {
                if let RequestKind::Oneshot(request) = &request_kind {
                  self.oneshots.remove(request);
                }
                read
              }
              Err(_) => {
                continue;
              }
            };
            data.push(read);
          }
          data
        };
        let response = Response { spans };

        // NOTE: this logically shouldn't happen
        if let Err(error) = sender.try_send(Ok(response.clone())) {
          match &request_kind {
            RequestKind::Oneshot(request) => self.oneshots.remove(request),
            RequestKind::Stream(request) => self.streams.remove(request),
          };

          tracing::debug! {
            %error,
            "Failed sending response from worker task"
          }
        }
      }
    }
  }
}
