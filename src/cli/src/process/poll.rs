#[allow(unused_imports)]
use crate::{service::*, *};

pub(crate) struct Process {
  #[allow(unused)]
  config: config::Manager,

  #[allow(unused)]
  services: service::Container,
}

impl Process {
  pub(crate) fn new(
    config: config::Manager,
    services: service::Container,
  ) -> Self {
    Self { config, services }
  }
}

impl super::Process for Process {}

#[async_trait::async_trait]
impl super::Recurring for Process {
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let response = self.services.cloud().poll().await?;

    let _ = self.config.reload_json(&response.text).await;

    Ok(())
  }
}
