mod discovery;
mod measurement;
mod ping;
mod push;
mod update;

pub trait Process {}

#[async_trait::async_trait]
pub trait Recurring: Process {
  async fn execute(&self) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait Background: Process {
  async fn execute(&self);
}
