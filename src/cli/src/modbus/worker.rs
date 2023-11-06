use std::collections::HashMap;
use std::sync::Arc;

use sqlx::types::chrono;
use tokio::sync::Mutex;

use tokio::task::JoinHandle;

// TODO: bounded channels?
// TODO: trace server info
// TODO: investigate if arc mutex is correct here
// TODO: make a tunable connection and store read params in it

#[derive(Debug, Clone)]
pub struct Request {
  pub id: String,
  pub spans: Vec<Box<dyn super::span::Span>>,
}

#[derive(Debug, Clone)]
pub struct Response {
  pub id: String,
  pub spans: Vec<Vec<u16>>,
}

pub enum Error {
  InvalidId,
}

#[derive(Debug, Clone)]
pub struct Worker {
  sender: flume::Sender<(Request, flume::Sender<Result<Response, Error>>)>,
  handle: Arc<JoinHandle<()>>,
}

impl Worker {
  pub fn new(connections: HashMap<String, super::conn::Connection>) -> Self {
    let (sender, receiver) = flume::unbounded();
    let connections = connections
      .iter_mut()
      .map(|(id, connection)| (*id, Arc::new(Mutex::new(*connection))))
      .collect::<HashMap<_, _>>();
    let handle = tokio::spawn(async move {
      let task = WorkerTask {
        connections,
        receiver,
      };
      task.execute().await
    });
    Self { sender, handle }
  }

  pub async fn send(
    &self,
    request: Request,
  ) -> Result<Response, flume::RecvError> {
    let (sender, receiver) = flume::bounded(1);
    self.sender.send_async((request, sender)).await;
    let response = receiver.recv_async().await?;
    Ok(response)
  }
}

#[derive(Debug, Clone)]
struct WorkerTask {
  connections: HashMap<String, Arc<Mutex<super::conn::Connection>>>,
  receiver: flume::Receiver<(Request, flume::Sender<Result<Response, Error>>)>,
}

impl WorkerTask {
  pub async fn execute(&self) -> () {
    let mut read_params = super::conn::ConnectionReadParams::new(
      chrono::Duration::milliseconds(1000),
      chrono::Duration::milliseconds(50),
      3,
    );

    loop {
      match self.receiver.recv_async().await {
        Ok((request, sender)) => {
          let connection = match self.connections.get(&request.id) {
            Some(connection) => connection.clone(),
            None => {
              if let Err(error) = sender.send_async(Err(Error::InvalidId)).await
              {
                tracing::debug! {
                  %error,
                  "Failed sending response from worker task"
                }
              }
            }
          };

          let data = {
            let connection = connection.clone().lock_owned().await;
            let data = Vec::new();
            for span in request.spans {
              let read = match (*connection)
                .read(span.as_ref(), read_params.clone())
                .await
              {
                Ok(read) => read,
                Err(error) => {
                  tracing::debug! {
                    %error,
                    "failed reading from connection"
                  };
                }
              };
              data.push(read);
            }
          };

          let response = Response {
            id: request.id,
            spans: data,
          };

          if let Err(error) = sender.send_async(Ok(response)).await {
            tracing::debug! {
              %error,
              "Failed sending response from worker task"
            }
          }
        }
        Err(error) => {
          tracing::debug! {
            %error,
            "Failed receiving request from worker task",
          };
        }
      }
    }
  }
}
