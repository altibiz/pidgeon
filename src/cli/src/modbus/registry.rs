use std::{
  collections::HashMap,
  net::SocketAddr,
  sync::{Arc, Mutex},
};

use super::{batch::batch_spans, register::*, span::SpanParser};
use super::{span::Span, worker::*};

#[derive(Clone, Debug)]
pub struct Registry {
  slaves: HashMap<String, (SocketAddr, tokio_modbus::Slave)>,
  workers: HashMap<SocketAddr, Arc<Worker>>,
}

impl Registry {
  pub fn new() -> Self {
    Self {
      slaves: HashMap::new(),
      workers: HashMap::new(),
    }
  }

  pub async fn r#match(
    &mut self,
    socket: SocketAddr,
    slave: Option<tokio_modbus::Slave>,
    detect: Vec<DetectRegister<RegisterKind>>,
    id: Vec<IdRegister<RegisterKind>>,
  ) -> Option<String> {
    let worker = match self.workers.get(&socket) {
      Some(worker) => worker.clone(),
      None => Arc::new(Worker::new()),
    };

    let detect_batches = batch_spans(detect, 3);
    let detect_response = worker
      .send(Request {
        socket,
        slave,
        spans: detect_batches
          .iter()
          .map(|batch| (batch.address(), batch.quantity()))
          .collect::<Vec<_>>(),
      })
      .await
      .ok()?;

    let matches = detect_batches.iter().zip(detect_response.spans.iter()).all(
      |(batch, span)| {
        let parsed = match batch.parse(span.iter().cloned()) {
          Ok(parsed) => parsed,
          Err(_) => return false,
        };

        parsed.spans.iter().all(|span| span.matches())
      },
    );

    if !matches {
      return None;
    }

    let id_batches = batch_spans(id, 3);
    let id_response = worker
      .send(Request {
        socket,
        slave,
        spans: id_batches
          .iter()
          .map(|batch| (batch.address(), batch.quantity()))
          .collect::<Vec<_>>(),
      })
      .await
      .ok()?;

    

    id_batches.iter().zip(id_response.spans.iter())
      .map(|(batch, span)| { 
         batch.parse(span)

      })
      .reduce(|id, next|)      
      None
  }
}
