use either::Either;
use futures::{future::try_join_all, TryFutureExt};
use futures_core::Stream;

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
impl super::Background for Process {
  async fn execute(&self) {
    let config = self.config.reload_async().await.unwrap();

    let devices = self.init_devices(config).await;

    loop {}
  }
}

type BoxedMeasurementStream = Box<
  dyn Stream<
      Item = Result<
        Vec<
          Either<
            modbus::IdRegister<modbus::RegisterValue>,
            modbus::MeasurementRegister<modbus::RegisterValue>,
          >,
        >,
        modbus::ServerReadError,
      >,
    > + Send
    + Sync,
>;

struct Device {
  id: String,
  kind: String,
  destination: modbus::Destination,
  id_registers: Vec<modbus::IdRegister<modbus::RegisterKind>>,
  measurement_registers: Vec<modbus::MeasurementRegister<modbus::RegisterKind>>,
  stream: BoxedMeasurementStream,
}

impl Process {
  async fn init_devices(
    &self,
    config: config::Parsed,
  ) -> anyhow::Result<Vec<Device>> {
    try_join_all(
      self
        .services
        .db
        .get_devices()
        .await?
        .into_iter()
        .filter_map(|device| {
          config
            .modbus
            .devices
            .values()
            .filter(|device_config| device_config.kind == device.kind)
            .next()
            .map(|config| (device, config.clone()))
        })
        .map(|(device, config)| {
          self.make_stream(device.clone(), config.clone()).map_ok(
            move |stream| Device {
              id: device.id,
              kind: device.kind,
              destination: modbus::Destination {
                address: network::to_socket(db::to_ip(device.address)),
                slave: db::to_modbus_slave(device.slave),
              },
              id_registers: config.id,
              measurement_registers: config.measurement,
              stream,
            },
          )
        }),
    )
    .await
  }

  async fn make_stream(
    &self,
    device: db::Device,
    config: config::ParsedDevice,
  ) -> anyhow::Result<BoxedMeasurementStream> {
    Ok(Box::new(
      self
        .services
        .modbus
        .stream_from_id(
          &device.id,
          config
            .id
            .into_iter()
            .map(|register| Either::Left(register))
            .chain(
              config
                .measurement
                .into_iter()
                .map(|register| Either::Right(register)),
            )
            .collect::<Vec<_>>(),
        )
        .await?,
    ))
  }
}
