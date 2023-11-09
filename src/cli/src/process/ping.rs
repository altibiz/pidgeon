use futures::future::join_all;

use crate::{config, service::*};

pub struct Process {
  config: config::Manager,
  services: super::Services,
}

impl super::Process for Process {
  fn new(config: config::Manager, services: super::Services) -> Self {
    Self { config, services }
  }
}

#[async_trait::async_trait]
impl super::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let devices = self.services.db.get_devices().await?;

    join_all()

    Ok(())
  }
}


impl Process {
  async fn ping_device(device: Device) {
    
  }
}
