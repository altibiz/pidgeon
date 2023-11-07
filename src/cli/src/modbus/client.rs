use std::net::SocketAddr;

use super::connection::*;
use super::worker::*;

#[async_trait::async_trait]
pub trait Client {
  fn address(&self) -> SocketAddr;

  async fn add(&mut self, id: String, connection: Connection) -> ();

  async fn remove(&mut self, id: String) -> Option<Connection>;

  async fn send(&self, request: Request) -> Result<Response, Error>;
}
