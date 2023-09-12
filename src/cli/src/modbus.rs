use either::Either;
use futures_time::future::FutureExt;
use regex::Regex;
use std::{
  collections::HashMap,
  net::{IpAddr, SocketAddr, SocketAddrV4},
  sync::Arc,
};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_modbus::{
  client::Context,
  prelude::{
    tcp::{connect, connect_slave},
    Reader,
  },
  Address, Quantity, Slave, SlaveId,
};

#[derive(Debug, Clone, Copy)]
pub struct StringRegisterKind {
  pub length: Quantity,
}

#[derive(Debug, Clone, Copy)]
pub enum RegisterKind {
  U16,
  U32,
  S16,
  S32,
  String(StringRegisterKind),
}

#[derive(Debug, Clone)]
pub struct RegisterConfig {
  pub name: String,
  pub address: Address,
  pub kind: RegisterKind,
}

#[derive(Debug, Clone)]
pub struct DetectRegister {
  pub address: u16,
  pub kind: RegisterKind,
  pub r#match: Either<String, Regex>,
}

#[derive(Debug, Clone)]
pub struct IdRegister {
  pub address: u16,
  pub kind: RegisterKind,
}

#[derive(Debug, Clone)]
pub struct DeviceConfig {
  pub kind: String,
  pub detect: Vec<DetectRegister>,
  pub id: Vec<IdRegister>,
  pub registers: Vec<RegisterConfig>,
}

#[derive(Debug, Clone)]
struct NetworkDevice {
  connection_id: ConnectionId,
  id: String,
  config: DeviceConfig,
}

#[derive(Debug, Clone)]
pub struct DeviceData {
  pub device: DeviceConfig,
  pub id: String,
  pub registers: Vec<RegisterData>,
}

#[derive(Debug, Clone)]
pub struct RegisterData {
  pub register: RegisterConfig,
  pub value: RegisterValue,
}

#[derive(Debug, Clone)]
pub enum RegisterValue {
  U16(u16),
  U32(u32),
  S16(i16),
  S32(i32),
  String(String),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ConnectionId {
  socket: SocketAddr,
  slave_id: Option<SlaveId>,
}

#[derive(Debug)]
struct Connection {
  ctx: Context,
}

#[derive(Debug, Clone)]
pub struct ModbusClient {
  timeout: futures_time::time::Duration,
  devices: Vec<DeviceConfig>,
  network_devices: Arc<Mutex<Vec<NetworkDevice>>>,
  pool: Arc<Mutex<HashMap<ConnectionId, Arc<Mutex<Connection>>>>>,
}

#[derive(Debug, Error)]
pub enum ModbusClientError {
  #[error("Failed connecting to device")]
  Connection(#[from] std::io::Error),

  #[error("Ipv6 addresses are not supported")]
  Ipv6,

  #[error("Failed to parse response")]
  Parse,
}

impl ModbusClient {
  pub fn new(
    timeout: u64,
    devices: Vec<DeviceConfig>,
  ) -> Result<Self, ModbusClientError> {
    Ok(Self {
      timeout: futures_time::time::Duration::from_millis(timeout),
      devices,
      network_devices: Arc::new(Mutex::new(Vec::new())),
      pool: Arc::new(Mutex::new(HashMap::new())),
    })
  }

  #[tracing::instrument(skip_all, fields(ip_count = ips.len(), count))]
  pub async fn detect(
    &self,
    ips: Vec<IpAddr>,
  ) -> Result<(), ModbusClientError> {
    let mut network_devices = Vec::new();
    for ip in ips {
      if let Some(device) = self.match_device(ip, None).await {
        network_devices.push(device);
        continue;
      }

      for slave in 0..255 {
        if let Some(device) = self.match_device(ip, Some(slave)).await {
          network_devices.push(device);
        }
      }
    }
    tracing::Span::current().record("count", network_devices.len());

    tracing::debug! {
      "Found {:?} devices",
      network_devices.len()
    };

    {
      *self.network_devices.lock().await = network_devices;
    };

    Ok(())
  }

  #[tracing::instrument(fields(count))]
  pub async fn clean(&self) {
    let mut map = self.pool.lock().await;
    let network_devices = self.network_devices.lock().await;

    let count_before = map.len();
    map.retain(|id, _| {
      for network_device in network_devices.iter() {
        if network_device.connection_id.eq(id) {
          return true;
        }
      }
      false
    });
    let count_after = map.len();
    tracing::Span::current().record("count", count_before - count_after);
  }

  #[tracing::instrument(skip_all, fields(to_read, read))]
  pub async fn read(&self) -> Result<Vec<DeviceData>, ModbusClientError> {
    let network_devices = { self.network_devices.lock().await.clone() };

    let to_read = network_devices.len();
    let mut read = to_read;
    tracing::Span::current().record("to_read", to_read);
    let mut data: Vec<DeviceData> = Vec::new();
    for device in network_devices {
      let mutex = match self.connect(device.connection_id.clone()).await {
        Ok(mutex) => mutex,
        Err(error) => {
          tracing::warn! {
            %error,
            "Failed connecting to device {:?}",
            device.connection_id.clone()
          };
          read -= 1;
          continue;
        }
      };
      let mut device_data: DeviceData = DeviceData {
        device: device.config.clone(),
        id: device.id.clone(),
        registers: Vec::new(),
      };

      for register in device.config.registers.iter() {
        let value = match Self::read_register(
          mutex.clone(),
          self.timeout,
          register.clone(),
        )
        .await
        {
          Ok(value) => value,
          Err(error) => {
            tracing::warn! {
              %error,
              "Failed reading device {:?} register {:?}",
              device.connection_id.clone(),
              register.clone()
            };
            read -= 1;
            break;
          }
        };

        device_data.registers.push(RegisterData {
          register: register.clone(),
          value,
        })
      }

      data.push(device_data);
    }
    tracing::Span::current().record("read", read);

    tracing::debug! {
      "Read {:?} devices",
      data.len()
    };

    Ok(data)
  }

  fn make_connection_id(
    ip: IpAddr,
    slave: Option<SlaveId>,
  ) -> Result<ConnectionId, ModbusClientError> {
    let id = ConnectionId {
      socket: match ip {
        IpAddr::V4(ipv4) => SocketAddr::V4(SocketAddrV4::new(ipv4, 502)),
        IpAddr::V6(_) => return Err(ModbusClientError::Ipv6),
      },
      slave_id: slave,
    };

    Ok(id)
  }

  async fn connect(
    &self,
    id: ConnectionId,
  ) -> Result<Arc<Mutex<Connection>>, ModbusClientError> {
    let mut map = self.pool.lock().await;
    let mutex = match map.get(&id) {
      Some(mutex) => mutex.clone(),
      None => {
        let ctx = match id.slave_id {
          Some(slave_id) => {
            connect_slave(id.socket, Slave(slave_id))
              .timeout(self.timeout)
              .await??
          }
          None => connect(id.socket).timeout(self.timeout).await??,
        };
        let conn = Connection { ctx };
        let mutex = Arc::new(Mutex::new(conn));
        map.entry(id).or_insert(mutex).clone()
      }
    };

    Ok(mutex)
  }

  async fn match_device(
    &self,
    ip: IpAddr,
    slave: Option<SlaveId>,
  ) -> Option<NetworkDevice> {
    let connection_id = match Self::make_connection_id(ip, slave) {
      Ok(connection_id) => connection_id,
      _ => return None,
    };

    for config in self.devices.iter() {
      let mutex = match self.connect(connection_id.clone()).await {
        Ok(mutex) => mutex,
        _ => continue,
      };

      let tasks = config.detect.iter().map(|detect| {
        let task_mutext = mutex.clone();
        let task_timeout = self.timeout;
        let task_detect = detect.clone();

        tokio::spawn(async move {
          Self::detect_register(task_mutext, task_timeout, task_detect).await
        })
      });

      let mut detected = true;
      for task in tasks {
        if !task.await.unwrap_or(false) {
          detected = false;
          break;
        }
      }
      if !detected {
        continue;
      }

      let id = Self::get_id(
        mutex.clone(),
        self.timeout,
        config.kind.clone(),
        config.id.clone(),
      )
      .await;
      match id {
        None => continue,
        Some(id) => {
          return Some(NetworkDevice {
            config: config.clone(),
            connection_id,
            id,
          })
        }
      }
    }

    None
  }

  async fn detect_register(
    mutex: Arc<Mutex<Connection>>,
    timeout: futures_time::time::Duration,
    detect: DetectRegister,
  ) -> bool {
    let value = match Self::read_register(
      mutex.clone(),
      timeout,
      RegisterConfig {
        name: "detect".to_string(),
        address: detect.address,
        kind: detect.kind,
      },
    )
    .await
    {
      Ok(value) => value,
      _ => return false,
    };

    Self::match_register(value, detect.r#match.clone())
  }

  async fn get_id(
    mutex: Arc<Mutex<Connection>>,
    timeout: futures_time::time::Duration,
    kind: String,
    registers: Vec<IdRegister>,
  ) -> Option<String> {
    let mut id = format!("{kind}-");

    for register in registers {
      let value = Self::read_register(
        mutex.clone(),
        timeout.clone(),
        RegisterConfig {
          name: "id".to_string(),
          address: register.address,
          kind: register.kind,
        },
      )
      .await
      .ok();

      match value {
        None => return None,
        Some(value) => {
          id = id + Self::register_to_string(value).as_str();
        }
      }
    }

    Some(id)
  }

  fn match_register(
    value: RegisterValue,
    r#match: Either<String, Regex>,
  ) -> bool {
    let matching_value = Self::register_to_string(value);

    match &r#match {
      Either::Left(value) => matching_value.eq(value),
      Either::Right(regex) => regex.is_match(matching_value.as_str()),
    }
  }

  fn register_to_string(value: RegisterValue) -> String {
    match value {
      RegisterValue::U16(value) => value.to_string(),
      RegisterValue::U32(value) => value.to_string(),
      RegisterValue::S16(value) => value.to_string(),
      RegisterValue::S32(value) => value.to_string(),
      RegisterValue::String(value) => value,
    }
  }

  async fn read_register(
    mutex: Arc<Mutex<Connection>>,
    timeout: futures_time::time::Duration,
    register: RegisterConfig,
  ) -> Result<RegisterValue, ModbusClientError> {
    let register_size: Quantity = match register.kind {
      RegisterKind::U16 => 1,
      RegisterKind::U32 => 2,
      RegisterKind::S16 => 1,
      RegisterKind::S32 => 2,
      RegisterKind::String(StringRegisterKind { length }) => length,
    };

    let response = {
      let mut conn = mutex.lock().await;
      conn
        .ctx
        .read_holding_registers(register.address, register_size)
        .timeout(timeout)
        .await??
    };

    let value = Self::parse_register(response, register.kind)?;

    Ok(value)
  }

  fn parse_register(
    data: Vec<u16>,
    kind: RegisterKind,
  ) -> Result<RegisterValue, ModbusClientError> {
    let value = match kind {
      RegisterKind::U16 => match Self::parse_u16_register(data) {
        Some(value) => RegisterValue::U16(value),
        None => return Err(ModbusClientError::Parse),
      },
      RegisterKind::U32 => match Self::parse_u32_register(data) {
        Some(value) => RegisterValue::U32(value),
        None => return Err(ModbusClientError::Parse),
      },
      RegisterKind::S16 => match Self::parse_s16_register(data) {
        Some(value) => RegisterValue::S16(value),
        None => return Err(ModbusClientError::Parse),
      },
      RegisterKind::S32 => match Self::parse_s32_register(data) {
        Some(value) => RegisterValue::S32(value),
        None => return Err(ModbusClientError::Parse),
      },
      RegisterKind::String(_) => match Self::parse_string_register(data) {
        Some(value) => RegisterValue::String(value),
        None => return Err(ModbusClientError::Parse),
      },
    };

    Ok(value)
  }

  fn parse_u16_register(data: Vec<u16>) -> Option<u16> {
    let first = *data.first()?;

    Some(first)
  }

  fn parse_u32_register(data: Vec<u16>) -> Option<u32> {
    let first = *data.first()?;
    let second = *data.get(1)?;

    Some(u32::from(first) << 16 | u32::from(second))
  }

  fn parse_s16_register(data: Vec<u16>) -> Option<i16> {
    let first = *data.first()?;

    Some(first as i16)
  }

  fn parse_s32_register(data: Vec<u16>) -> Option<i32> {
    let first = *data.first()?;
    let second = *data.get(1)?;

    Some((u32::from(first) << 16 | u32::from(second)) as i32)
  }

  fn parse_bytes(data: Vec<u16>) -> Option<Vec<u8>> {
    let mut result = Vec::new();

    for &value in &data {
      let _upper_char = (value >> 8) as u8;
      let _lower_char = (value & 0xFF) as u8;

      result.push((value >> 8) as u8);
      result.push((value & 0xFF) as u8);
    }

    Some(result)
  }

  fn parse_string_register(data: Vec<u16>) -> Option<String> {
    let bytes = Self::parse_bytes(data)?;
    let string = String::from_utf8(bytes).ok()?;

    Some(string)
  }
}

pub fn registers_to_json(registers: Vec<RegisterData>) -> serde_json::Value {
  serde_json::Value::Object(
    registers
      .iter()
      .map(
        |RegisterData {
           register: RegisterConfig { name, .. },
           value,
         }| {
          (
            name.clone(),
            match value {
              RegisterValue::U16(value) => serde_json::json!(value),
              RegisterValue::U32(value) => serde_json::json!(value),
              RegisterValue::S16(value) => serde_json::json!(value),
              RegisterValue::S32(value) => serde_json::json!(value),
              RegisterValue::String(value) => serde_json::json!(value),
            },
          )
        },
      )
      .collect::<serde_json::Map<String, serde_json::Value>>(),
  )
}
