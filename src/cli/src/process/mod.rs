pub mod discover;
pub mod measure;
pub mod ping;
pub mod push;
pub mod update;

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{config, service};

pub trait Process {
  fn new(config: config::Manager, services: service::Container) -> Self;
}

#[async_trait::async_trait]
pub trait Recurring: Process {
  async fn execute(&self) -> anyhow::Result<()>;
}

struct Handle {
  token: tokio_util::sync::CancellationToken,
  handle: tokio::task::JoinHandle<()>,
}

struct Values {
  discover: Handle,
  ping: Handle,
  measure: Handle,
  push: Handle,
  update: Handle,
}

pub struct Processes {
  values: Arc<Mutex<Values>>,
}

impl Processes {
  pub fn new(config: config::Manager) -> Self {
    Self {
      values: Arc::new(Mutex::new()),
    }
  }
}
