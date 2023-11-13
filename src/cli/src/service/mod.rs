pub mod cloud;
pub mod db;
pub mod hardware;
pub mod modbus;
pub mod network;

use crate::*;

pub trait Service {
  fn new(config: config::Values) -> Self;
}
