use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::conn::*;

// TODO: bounded channels?
// TODO: trace server info
// TODO: investigate if arc mutex is correct here
// TODO: make a tunable connection and store read params in it

#[derive(Debug, Clone)]
pub struct Request {
  pub socket: SocketAddr,
  pub slave: Option<tokio_modbus::Slave>,
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
  sender: flume::Sender<(Request, flume::Sender<Result<Response, Error>>)>,
  handle: Arc<Mutex<tokio::task::JoinHandle<()>>>,
}

impl Worker {
  pub fn new() -> Self {
    let (sender, receiver) = flume::unbounded();
    let task = Task {
      connections: HashMap::new(),
      receiver,
    };
    let handle = tokio::spawn(task.execute());
    Self {
      sender,
      handle: Arc::new(Mutex::new(handle)),
    }
  }

  pub async fn send(&self, request: Request) -> Result<Response, Error> {
    let (sender, receiver) = flume::bounded(1);
    self.sender.send_async((request, sender)).await;
    let response = receiver.recv_async().await?;
    response
  }
}

#[derive(Debug, Clone)]
struct Task {
  connections: HashMap<(SocketAddr, Option<u8>), Arc<Mutex<Connection>>>,
  receiver: flume::Receiver<(Request, flume::Sender<Result<Response, Error>>)>,
  pending: Vec<(Request, flume::Sender<Result<Response, Error>>)>,
}

impl Task {
  pub async fn execute(mut self) {
    let read_params = ConnectionReadParams::new(
      chrono::Duration::milliseconds(1000),
      chrono::Duration::milliseconds(50),
      3,
    )
    .unwrap();

    loop {
      let (request, sender) = match self.receiver.recv_async().await {
        Ok(received) => received,
        Err(error) => {
          tracing::debug! {
            %error,
            "Failed receiving request from worker task",
          };
          continue;
        }
      };

      let connection = match self
        .connections
        .get(&(request.socket, request.slave.map(|slave| slave.0)))
      {
        Some(connection) => connection.clone(),
        None => {
          match Connection::connect(request.socket, request.slave).await {
            Ok(connection) => {
              let connection = Arc::new(Mutex::new(connection));
              self.connections.insert(
                (request.socket, request.slave.map(|slave| slave.0)),
                connection.clone(),
              );
              connection
            }
            Err(_) => {
              if let Err(error) =
                sender.send_async(Err(Error::FailedToConnect)).await
              {
                tracing::debug! {
                  %error,
                  "Failed sending response from worker task"
                }
              }
              continue;
            }
          }
        }
      };

      let spans = {
        let mut connection = connection.clone().lock_owned().await;
        let mut data = Vec::new();
        for span in request.spans {
          let read = match (*connection)
            .read(span.0, span.1, read_params.clone())
            .await
          {
            Ok(read) => read,
            Err(_) => {
              continue;
            }
          };
          data.push(read);
        }
        data
      };

      let response = Response { spans };

      if let Err(error) = sender.send_async(Ok(response)).await {
        tracing::debug! {
          %error,
          "Failed sending response from worker task"
        }
      }
    }
  }
}
