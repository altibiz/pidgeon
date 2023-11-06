use std::{
  collections::HashMap,
  net::SocketAddr,
  sync::{Arc, Mutex},
};

use super::register::*;

#[derive(Clone, Debug)]
pub struct Registry {
  clients: HashMap<String, Arc<Mutex<Box<dyn super::client::Client>>>>,
  transient: HashMap<
    (SocketAddr, Option<tokio_modbus::Slave>),
    Arc<Mutex<Box<dyn super::client::Client>>>,
  >,
}

impl Registry {
  pub fn new() -> Self {
    Self {
      clients: HashMap::new(),
      transient: HashMap::new(),
    }
  }

  pub fn r#match(
    &mut self,
    detect: Vec<DetectRegister<RegisterKind>>,
    id: Vec<IdRegister<RegisterKind>>,
  ) -> Option<String> {
    None
  }
}
