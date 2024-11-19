use tokio_modbus::{Address, Quantity};

use super::span::Span;

pub(crate) trait Record: Span {
  fn values(&self) -> impl Iterator<Item = u16>;
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct SimpleRecord {
  pub(crate) address: u16,
  pub(crate) values: Vec<u16>,
}

impl Span for SimpleRecord {
  fn address(&self) -> Address {
    self.address
  }

  fn quantity(&self) -> Quantity {
    self.values.len() as Quantity
  }
}

impl Span for &SimpleRecord {
  fn address(&self) -> Address {
    self.address
  }

  fn quantity(&self) -> Quantity {
    self.values.len() as Quantity
  }
}

impl Span for Box<SimpleRecord> {
  fn address(&self) -> Address {
    self.address
  }

  fn quantity(&self) -> Quantity {
    self.values.len() as Quantity
  }
}

impl Record for SimpleRecord {
  fn values(&self) -> impl Iterator<Item = u16> {
    self.values.clone().into_iter()
  }
}

impl Record for &SimpleRecord {
  fn values(&self) -> impl Iterator<Item = u16> {
    self.values.clone().into_iter()
  }
}

impl Record for Box<SimpleRecord> {
  fn values(&self) -> impl Iterator<Item = u16> {
    self.values.clone().into_iter()
  }
}
