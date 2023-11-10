use std::pin::Pin;
use std::sync::Arc;

use either::Either;
use futures::stream::StreamExt;
use futures::{future::join_all, FutureExt};
use futures_core::Stream;
use itertools::Itertools;
use tokio::sync::Mutex;

use crate::{config, service::*};

pub struct Process {
  config: config::Manager,
  services: super::Services,
  streams: Arc<Mutex<Vec<DeviceStream>>>,
}

impl super::Process for Process {
  fn new(config: config::Manager, services: super::Services) -> Self {
    Self {
      config,
      services,
      streams: Arc::new(Mutex::new(Vec::new())),
    }
  }
}

#[async_trait::async_trait]
impl super::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let measurements = self.get_unprocessed_measurements().await;
    if let Err(error) = self.consolidate(measurements).await {
      tracing::debug! {
        %error,
        "Failed to send measurements to the db"
      }
    }
    let config = self.config.reload_async().await?;
    let devices_from_db = self.get_devices_from_db(config).await?;
    let streams = self.streams.clone().lock_owned().await;
    *streams = self.merge_devices(*streams, devices_from_db).await;
    Ok(())
  }
}

type MeasurementStreamRegisters = Vec<
  Either<
    modbus::IdRegister<modbus::RegisterValue>,
    modbus::MeasurementRegister<modbus::RegisterValue>,
  >,
>;

type BoxedMeasurementStream = Pin<
  Box<
    dyn Stream<Item = Result<MeasurementStreamRegisters, modbus::ServerReadError>>
      + Send
      + Sync,
  >,
>;

#[derive(Clone, Debug)]
struct Device {
  id: String,
  kind: String,
  destination: modbus::Destination,
  id_registers: Vec<modbus::IdRegister<modbus::RegisterKind>>,
  measurement_registers: Vec<modbus::MeasurementRegister<modbus::RegisterKind>>,
}

struct DeviceStream {
  device: Device,
  stream: BoxedMeasurementStream,
}

#[derive(Clone, Debug)]
struct DeviceRegisters {
  device: Device,
  registers: MeasurementStreamRegisters,
}

impl Process {
  async fn get_unprocessed_measurements(&self) -> Vec<DeviceRegisters> {
    let mut streams = self.streams.clone().lock_owned().await;
    let mut measurements = Vec::new();
    for DeviceStream { device, stream } in streams.iter_mut() {
      loop {
        match stream
          .next()
          .now_or_never()
          .flatten()
          .map(|x| x.ok())
          .flatten()
        {
          Some(registers) => measurements.push(DeviceRegisters {
            device: device.clone(),
            registers,
          }),
          None => break,
        }
      }
    }
    measurements
  }

  async fn get_devices_from_db(
    &self,
    config: config::Parsed,
  ) -> anyhow::Result<Vec<Device>> {
    Ok(
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
            .map(|config| Device {
              id: device.id,
              kind: device.kind,
              destination: modbus::Destination {
                address: network::to_socket(db::to_ip(device.address)),
                slave: db::to_modbus_slave(device.slave),
              },
              id_registers: config.id.clone(),
              measurement_registers: config.measurement.clone(),
            })
        })
        .collect::<Vec<_>>(),
    )
  }

  async fn merge_devices(
    &self,
    old_devices: Vec<DeviceStream>,
    new_devices: Vec<Device>,
  ) -> Vec<DeviceStream> {
    join_all(
      old_devices
        .into_iter()
        .merge_join_by(new_devices.into_iter(), |x, y| {
          Ord::cmp(x.device.id.as_str(), y.id.as_str())
        })
        .filter_map(|x| match x {
          itertools::EitherOrBoth::Both(old_device, new_device) => {
            Some(Either::Left(DeviceStream {
              device: new_device,
              stream: old_device.stream,
            }))
          }
          itertools::EitherOrBoth::Left(_old_device) => None,
          itertools::EitherOrBoth::Right(new_device) => {
            Some(Either::Right(new_device))
          }
        })
        .map(|x| async move {
          Ok(match x {
            Either::Left(stream_device) => stream_device,
            Either::Right(device) => DeviceStream {
              stream: match self.make_stream(device.clone()).await {
                Ok(stream) => stream,
                Err(error) => {
                  return Err(std::convert::Into::<anyhow::Error>::into(error))
                }
              },
              device,
            },
          })
        }),
    )
    .await
    .into_iter()
    .filter_map(|x| x.ok())
    .collect::<Vec<_>>()
  }

  async fn make_stream(
    &self,
    device: Device,
  ) -> anyhow::Result<BoxedMeasurementStream> {
    Ok(Box::pin(
      self
        .services
        .modbus
        .stream_from_id(
          &device.id,
          device
            .id_registers
            .into_iter()
            .map(|register| Either::Left(register))
            .chain(
              device
                .measurement_registers
                .into_iter()
                .map(|register| Either::Right(register)),
            )
            .collect::<Vec<_>>(),
        )
        .await?,
    ))
  }

  async fn consolidate(
    &self,
    measurements: Vec<DeviceRegisters>,
  ) -> anyhow::Result<()> {
    self
      .services
      .db
      .insert_measurements(
        measurements
          .into_iter()
          .filter_map(|measurement| {
            let id = modbus::make_id(
              measurement.device.kind,
              measurement
                .registers
                .iter()
                .cloned()
                .filter_map(Either::left),
            );

            let expected_id = measurement.device.id;
            if id != expected_id {
              tracing::debug! {
                "Failed verifying measurement of {:?}: got id {:?}",
                expected_id,
                id
              }

              return None;
            }

            Some(db::Measurement {
              id: 0,
              source: id,
              timestamp: chrono::Utc::now(),
              data: modbus::serialize_registers(
                measurement.registers.into_iter().filter_map(Either::right),
              ),
            })
          })
          .collect::<Vec<_>>(),
      )
      .await?;

    Ok(())
  }
}
