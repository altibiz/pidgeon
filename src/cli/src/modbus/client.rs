use std::collections::HashMap;
use std::net::SocketAddr;

#[async_trait::async_trait]
pub trait Client {
  fn address(&self) -> SocketAddr;

  async fn reset(
    &mut self,
    slaves: HashMap<String, super::conn::Connection>,
  ) -> ();

  async fn send(
    &self,
    request: super::worker::Request,
  ) -> Result<super::worker::Response, super::worker::Error>;
}
