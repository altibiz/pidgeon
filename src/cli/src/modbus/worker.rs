use tokio::sync::Mutex;

use tokio::task::JoinHandle;

use super::*;

// TODO: bounded channels

#[derive(Debug, Clone)]
pub struct Request {
  pub id: String,
  pub spans: Vec<Box<dyn Span>>,
}

#[derive(Debug, Clone)]
pub struct Response {
  pub id: String,
  pub spans: Vec<Vec<u16>>,
}

#[derive(Debug, Clone)]
pub struct Worker {
  sender: flume::Sender<(Request, flume::Sender<Response>)>,
  handle: Arc<JoinHandle<()>>,
}

impl Worker {
  pub fn new(connections: HashMap<String, Connection>) -> Self {
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
  // TODO: investigate if arc mutex is correct here
  connections: HashMap<String, Arc<Mutex<Connection>>>,
  receiver: flume::Receiver<(Request, flume::Sender<Response>)>,
}

impl WorkerTask {
  #[tracing::instrument(skip(self))]
  pub async fn execute(&self) -> () {
    loop {
      match self.receiver.recv_async().await {
        Ok((request, sender)) => {
          let connection = match self.connections.get(&request.id) {
            Some(connection) => connection.clone(),
            None => continue,
          };

          let data = {
            let connection = connection.clone().lock_owned().await;
            let data = Vec::new();
            for span in request.spans {
              let read = match connection.read(span).await {
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

          if let Err(error) = sender.send_async(response).await {
            tracing::debug! {
              %error,
              "Failed sending response from worker task"
            }
          }
        }
        Err(error) => {
          // TODO: trace server info
          tracing::debug! {
            %error,
            "Failed receiving request from worker task",
          };
        }
      }
    }
  }

  fn tune() {}
}
