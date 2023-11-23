use std::pin::Pin;
use std::sync::Arc;

use either::Either;
use futures::stream::StreamExt;
use futures::{future::join_all, FutureExt};
use futures_core::Stream;
use itertools::Itertools;
use tokio::sync::Mutex;

use crate::{service::*, *};

pub(crate) struct Process {
  #[allow(unused)]
  config: config::Manager,

  #[allow(unused)]
  services: service::Container,

  streams: Arc<Mutex<Vec<DeviceStream>>>,
}

impl Process {
  pub(crate) fn new(
    config: config::Manager,
    services: service::Container,
  ) -> Self {
    Self {
      config,
      services,
      streams: Arc::new(Mutex::new(Vec::new())),
    }
  }
}

impl super::Process for Process {}

#[async_trait::async_trait]
impl process::Recurring for Process {
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let config = self.config.reload().await;
    let measurements = self.get_unprocessed_measurements().await;
    self.consolidate(measurements).await;

    let devices_from_db = self.get_devices_from_db(config).await?;
    {
      let mut streams = self.streams.clone().lock_owned().await;
      self.merge_devices(&mut streams, devices_from_db).await;
    }

    Ok(())
  }
}

type MeasurementStreamRegisters = Vec<
  Either<
    modbus::IdRegister<modbus::RegisterValueStorage>,
    modbus::MeasurementRegister<modbus::RegisterValueStorage>,
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
  id_registers: Vec<modbus::IdRegister<modbus::RegisterKindStorage>>,
  measurement_registers:
    Vec<modbus::MeasurementRegister<modbus::RegisterKindStorage>>,
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
  #[tracing::instrument(skip(self))]
  async fn get_unprocessed_measurements(&self) -> Vec<DeviceRegisters> {
    let mut streams = self.streams.clone().lock_owned().await;
    let streams_len_before = streams.len();

    let mut measurements = Vec::new();
    streams.retain_mut(|DeviceStream { device, stream }| loop {
      match stream.next().now_or_never() {
        None => {
          return true;
        }
        Some(None) => {
          tracing::debug!("Stream for {:?} ended", device.id);
          return false;
        }
        Some(Some(Err(modbus::ServerReadError::FailedToConnect(error)))) => {
          tracing::warn!(
            "Failed to connect to device {:?} {}",
            device.id,
            error
          );
          return false;
        }
        Some(Some(Err(modbus::ServerReadError::ServerFailed(error)))) => {
          tracing::warn!("Device server failed {:?} {}", device.id, error);
          return false;
        }
        Some(Some(Err(modbus::ServerReadError::ParsingFailed(error)))) => {
          tracing::warn!("Parsing failed {:?} {}", device.id, error);
        }
        Some(Some(Ok(registers))) => {
          measurements.push(DeviceRegisters {
            device: device.clone(),
            registers,
          });
        }
      }
    });
    let measurements_len = measurements.len();
    let streams_len_after = streams.len();

    tracing::info!(
      "Got {:?} new measurements from {:?} streams of which {:?} were removed",
      measurements_len,
      streams_len_before,
      streams_len_after
    );

    measurements
  }

  #[tracing::instrument(skip_all)]
  async fn get_devices_from_db(
    &self,
    config: config::Values,
  ) -> anyhow::Result<Vec<Device>> {
    let db_devices = match self.services.db().get_devices().await {
      Ok(db_devices) => db_devices,
      Err(error) => {
        tracing::error!("Failed fetching devices from db {}", error);
        return Err(error.into());
      }
    };
    let db_devices_len = db_devices.len();

    let merged_devices = db_devices
      .into_iter()
      .filter_map(|device| {
        config
          .modbus
          .devices
          .values()
          .find(|device_config| device_config.kind == device.kind)
          .map(|config| Device {
            id: device.id,
            kind: device.kind,
            id_registers: config.id.clone(),
            measurement_registers: config.measurement.clone(),
          })
      })
      .collect::<Vec<_>>();
    let merged_devices_len = merged_devices.len();

    tracing::debug!(
      "Fetched {:?} from db of which {:?} had configs",
      db_devices_len,
      merged_devices_len
    );

    Ok(merged_devices)
  }

  #[tracing::instrument(skip_all)]
  async fn merge_devices(
    &self,
    devices: &mut Vec<DeviceStream>,
    new_devices: Vec<Device>,
  ) {
    let devices_before_len = devices.len();
    let new_devices_len = new_devices.len();

    let merged_devices = devices
      .drain(0..)
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
      .collect::<Vec<_>>();
    let merged_devices_len = merged_devices.len();

    *devices = join_all(merged_devices.into_iter().map(|x| async move {
      Ok(match x {
        Either::Left(stream_device) => stream_device,
        Either::Right(device) => DeviceStream {
          stream: match self.make_stream(device.clone()).await {
            Ok(stream) => stream,
            Err(error) => return Err((device, error)),
          },
          device,
        },
      })
    }))
    .await
    .into_iter()
    .filter_map(|stream| match stream {
      Ok(stream) => Some(stream),
      Err((device, error)) => {
        tracing::warn!("Failed creating stream for {:?} {}", device.id, error);
        None
      }
    })
    .collect::<Vec<_>>();
    let devices_after_len = devices.len();

    tracing::info!(
      "Merged {:?} old devices and {:?} new devices into {:?} devices of which {:?} could be streamed",
      devices_before_len,
      new_devices_len,
      merged_devices_len,
      devices_after_len
    );
  }

  #[tracing::instrument(skip_all)]
  async fn consolidate(&self, measurements: Vec<DeviceRegisters>) {
    let measurements_len = measurements.len();

    let verified_measurements = measurements
      .into_iter()
      .filter_map(|measurement| {
        let source = modbus::make_id(
          measurement.device.kind,
          measurement
            .registers
            .iter()
            .cloned()
            .filter_map(Either::left),
        );

        if source != measurement.device.id {
          tracing::warn! {
            "Failed verifying measurement of {:?}: got id {:?}",
            measurement.device.id,
            source
          }

          return None;
        }

        let timestamp =
          match measurement.registers.iter().cloned().find_map(Either::left) {
            None => {
              tracing::warn! {
                "No id register found for measurement of {:?}",
                measurement.device.id
              };
              return None;
            }
            Some(register) => register,
          }
          .storage
          .timestamp();

        let data = modbus::serialize_registers(
          measurement.registers.into_iter().filter_map(Either::right),
        );

        Some(db::Measurement {
          id: 0,
          source,
          timestamp,
          data,
        })
      })
      .collect::<Vec<_>>();
    let verified_measurements_len = verified_measurements.len();

    if let Err(error) = self
      .services
      .db()
      .insert_measurements(verified_measurements)
      .await
    {
      tracing::error!(
        "Failed sending {:?} measurements to the db {}",
        verified_measurements_len,
        error
      );
    };

    tracing::info!(
      "Of {:?} unverified measurements {:?} were verified and sent to the db",
      measurements_len,
      verified_measurements_len
    );
  }

  async fn make_stream(
    &self,
    device: Device,
  ) -> anyhow::Result<BoxedMeasurementStream> {
    Ok(Box::pin(
      self
        .services
        .modbus()
        .stream_from_id(
          &device.id,
          device
            .measurement_registers
            .into_iter()
            .map(Either::Right)
            .chain(device.id_registers.into_iter().map(Either::Left))
            .collect::<Vec<_>>(),
        )
        .await?,
    ))
  }
}
