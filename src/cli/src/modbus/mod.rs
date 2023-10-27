use either::Either;
use futures_time::future::FutureExt;
use regex::Regex;
use std::{
  collections::HashMap,
  net::{IpAddr, SocketAddr, SocketAddrV4},
  sync::Arc,
};
use thiserror::Error;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_modbus::{
  client::Context,
  prelude::{rtu, tcp, Reader},
  Address, Quantity, Slave, SlaveId,
};

macro_rules! parse_integer_register_kind {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident) => {{
    let bytes = Self::parse_numeric_bytes($data)?;
    let slice = bytes.as_slice().try_into().ok()?;
    let value = <$type>::from_ne_bytes(slice);
    RegisterValue::$variant(match $multiplier {
      Some($multiplier) => ((value as f64) * $multiplier).round() as $type,
      None => value,
    })
  }};
}

macro_rules! parse_floating_register_kind {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident) => {{
    let bytes = Self::parse_numeric_bytes($data)?;
    let slice = bytes.as_slice().try_into().ok()?;
    let value = <$type>::from_ne_bytes(slice);
    RegisterValue::$variant(match $multiplier {
      Some($multiplier) => ((value as f64) * $multiplier) as $type,
      None => value,
    })
  }};
}

#[derive(Debug, Clone, Copy)]
pub struct StringRegisterKind {
  pub length: Quantity,
}

#[derive(Debug, Clone, Copy)]
pub struct NumericRegisterKind {
  pub multiplier: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum RegisterKind {
  U16(NumericRegisterKind),
  U32(NumericRegisterKind),
  U64(NumericRegisterKind),
  S16(NumericRegisterKind),
  S32(NumericRegisterKind),
  S64(NumericRegisterKind),
  F32(NumericRegisterKind),
  F64(NumericRegisterKind),
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
struct Device {
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
  U64(u64),
  S16(i16),
  S32(i32),
  S64(i64),
  F32(f32),
  F64(f64),
  String(String),
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
enum Framer {
  Tcp,
  Rtu,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
struct ConnectionId {
  socket: SocketAddr,
  slave_id: Option<SlaveId>,
  framer: Framer,
}

#[derive(Debug, Clone)]
struct BatchRequestDefinition {
  address: Address,
  size: Quantity,
  registers: Vec<RegisterConfig>,
}

#[derive(Debug)]
struct Connection {
  ctx: Context,
}

#[derive(Debug, Clone)]
pub struct ModbusClient {
  timeout: futures_time::time::Duration,
  retries: u64,
  batching_threshold: usize,
  devices: Vec<DeviceConfig>,
  network_devices: Arc<Mutex<Vec<Device>>>,
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
    retries: u64,
    batching_threshold: usize,
    devices: Vec<DeviceConfig>,
  ) -> Result<Self, ModbusClientError> {
    Ok(Self {
      timeout: futures_time::time::Duration::from_millis(timeout),
      retries,
      devices,
      batching_threshold,
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

      for slave in Slave::min_device().0..Slave::max_device().0 {
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
      let mutex = match self.connect(device.connection_id).await {
        Ok(mutex) => mutex,
        Err(error) => {
          tracing::trace! {
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
          self.retries,
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

  async fn match_device(
    &self,
    ip: IpAddr,
    slave: Option<SlaveId>,
  ) -> Option<Device> {
    let tcp_connection_id =
      match Self::make_connection_id(ip, slave, Framer::Tcp) {
        Ok(connection_id) => connection_id,
        _ => return None,
      };

    if let Some(device) = self.match_framed_device(tcp_connection_id).await {
      return Some(device);
    }

    let rtu_connection_id =
      match Self::make_connection_id(ip, slave, Framer::Rtu) {
        Ok(connection_id) => connection_id,
        _ => return None,
      };

    self.match_framed_device(rtu_connection_id).await
  }

  async fn match_framed_device(
    &self,
    connection_id: ConnectionId,
  ) -> Option<Device> {
    for config in self.devices.iter() {
      let mutex = match self.connect(connection_id).await {
        Ok(mutex) => mutex,
        Err(err) => {
          tracing::trace! {
            %err,
            "Failed connecting to device on {:?}",
            connection_id
          };
          continue;
        }
      };

      let mut detected = true;
      for detect in config.detect.iter() {
        if !Self::detect_register(
          mutex.clone(),
          self.timeout,
          self.retries,
          detect.clone(),
        )
        .await
        {
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
        self.retries,
        config.kind.clone(),
        config.id.clone(),
      )
      .await;
      match id {
        None => continue,
        Some(id) => {
          return Some(Device {
            config: config.clone(),
            connection_id,
            id,
          })
        }
      }
    }

    None
  }

  async fn get_id(
    mutex: Arc<Mutex<Connection>>,
    timeout: futures_time::time::Duration,
    retries: u64,
    kind: String,
    registers: Vec<IdRegister>,
  ) -> Option<String> {
    let mut id = format!("{kind}-");

    for register in registers {
      let value = Self::read_register(
        mutex.clone(),
        timeout,
        retries,
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
          id += Self::register_to_string(value).as_str();
        }
      }
    }

    Some(id)
  }

  async fn detect_register(
    mutex: Arc<Mutex<Connection>>,
    timeout: futures_time::time::Duration,
    retries: u64,
    detect: DetectRegister,
  ) -> bool {
    let value = match Self::read_register(
      mutex.clone(),
      timeout,
      retries,
      RegisterConfig {
        name: "detect".to_string(),
        address: detect.address,
        kind: detect.kind,
      },
    )
    .await
    {
      Ok(value) => value,
      Err(_) => {
        return false;
      }
    };

    Self::match_register(value, detect.r#match.clone())
  }

  async fn read_register(
    mutex: Arc<Mutex<Connection>>,
    timeout: futures_time::time::Duration,
    retries: u64,
    register: RegisterConfig,
  ) -> Result<RegisterValue, ModbusClientError> {
    let register_size = Self::register_size(register.kind);

    let response = {
      let mut conn = mutex.lock().await;
      let mut response = conn
        .ctx
        .read_holding_registers(register.address, register_size)
        .timeout(timeout)
        .await;
      let mut remaining = retries;
      while remaining > 0 {
        response = conn
          .ctx
          .read_holding_registers(register.address, register_size)
          .timeout(timeout)
          .await;
        match response {
          Ok(Ok(_)) => {
            remaining = 0;
          }
          _ => {
            remaining -= 1;
          }
        };
      }

      response
    }??;

    match Self::parse_register(response, register.kind) {
      Some(register) => Ok(register),
      None => Err(ModbusClientError::Parse),
    }
  }

  async fn connect(
    &self,
    id: ConnectionId,
  ) -> Result<Arc<Mutex<Connection>>, ModbusClientError> {
    match id.framer {
      Framer::Tcp => self.connect_tcp(id).await,
      Framer::Rtu => self.connect_rtu(id).await,
    }
  }

  async fn connect_rtu(
    &self,
    id: ConnectionId,
  ) -> Result<Arc<Mutex<Connection>>, ModbusClientError> {
    let mut map = self.pool.lock().await;
    let mutex = match map.get(&id) {
      Some(mutex) => mutex.clone(),
      None => {
        let ctx = match id.slave_id {
          Some(slave_id) => {
            let transport = TcpStream::connect(id.socket)
              .timeout(self.timeout)
              .await??;
            rtu::attach_slave(transport, Slave(slave_id))
          }
          None => {
            let transport = TcpStream::connect(id.socket)
              .timeout(self.timeout)
              .await??;
            rtu::attach(transport)
          }
        };
        let conn = Connection { ctx };
        let mutex = Arc::new(Mutex::new(conn));
        map.entry(id).or_insert(mutex).clone()
      }
    };

    Ok(mutex)
  }

  async fn connect_tcp(
    &self,
    id: ConnectionId,
  ) -> Result<Arc<Mutex<Connection>>, ModbusClientError> {
    let mut map = self.pool.lock().await;
    let mutex = match map.get(&id) {
      Some(mutex) => mutex.clone(),
      None => {
        let ctx = match id.slave_id {
          Some(slave_id) => {
            tcp::connect_slave(id.socket, Slave(slave_id))
              .timeout(self.timeout)
              .await??
          }
          None => tcp::connect(id.socket).timeout(self.timeout).await??,
        };
        let conn = Connection { ctx };
        let mutex = Arc::new(Mutex::new(conn));
        map.entry(id).or_insert(mutex).clone()
      }
    };

    Ok(mutex)
  }

  fn make_batch_request_definitions(
    registers: Vec<RegisterConfig>,
    threshold: usize,
  ) -> Vec<BatchRequestDefinition> {
    registers.as_mut().sort_by_key(|register| register.address);
    let result = Vec::new();
    let mut current_batch = BatchRequestDefinition {
      address: registers[0].address,
      size: Self::register_size(registers[0].kind),
      registers: vec![registers[0]],
    };

    for register in registers.drain(1..) {}

    result
  }

  fn make_connection_id(
    ip: IpAddr,
    slave: Option<SlaveId>,
    framer: Framer,
  ) -> Result<ConnectionId, ModbusClientError> {
    let id = ConnectionId {
      socket: match ip {
        IpAddr::V4(ipv4) => SocketAddr::V4(SocketAddrV4::new(ipv4, 502)),
        IpAddr::V6(_) => return Err(ModbusClientError::Ipv6),
      },
      slave_id: slave,
      framer,
    };

    Ok(id)
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
      RegisterValue::U64(value) => value.to_string(),
      RegisterValue::S16(value) => value.to_string(),
      RegisterValue::S32(value) => value.to_string(),
      RegisterValue::S64(value) => value.to_string(),
      RegisterValue::F32(value) => value.to_string(),
      RegisterValue::F64(value) => value.to_string(),
      RegisterValue::String(value) => value,
    }
  }

  fn register_size(kind: RegisterKind) -> Quantity {
    match kind {
      RegisterKind::U16(_) => 1,
      RegisterKind::U32(_) => 2,
      RegisterKind::U64(_) => 4,
      RegisterKind::S16(_) => 1,
      RegisterKind::S32(_) => 2,
      RegisterKind::S64(_) => 4,
      RegisterKind::F32(_) => 2,
      RegisterKind::F64(_) => 4,
      RegisterKind::String(StringRegisterKind { length }) => length,
    }
  }

  fn parse_register(
    data: Vec<u16>,
    kind: RegisterKind,
  ) -> Option<RegisterValue> {
    let value = match kind {
      RegisterKind::U16(NumericRegisterKind { multiplier }) => {
        parse_integer_register_kind!(U16, u16, data, multiplier)
      }
      RegisterKind::U32(NumericRegisterKind { multiplier }) => {
        parse_integer_register_kind!(U32, u32, data, multiplier)
      }
      RegisterKind::U64(NumericRegisterKind { multiplier }) => {
        parse_integer_register_kind!(U64, u64, data, multiplier)
      }
      RegisterKind::S16(NumericRegisterKind { multiplier }) => {
        parse_integer_register_kind!(S16, i16, data, multiplier)
      }
      RegisterKind::S32(NumericRegisterKind { multiplier }) => {
        parse_integer_register_kind!(S32, i32, data, multiplier)
      }
      RegisterKind::S64(NumericRegisterKind { multiplier }) => {
        parse_integer_register_kind!(S64, i64, data, multiplier)
      }
      RegisterKind::F32(NumericRegisterKind { multiplier }) => {
        parse_floating_register_kind!(F32, f32, data, multiplier)
      }
      RegisterKind::F64(NumericRegisterKind { multiplier }) => {
        parse_floating_register_kind!(F64, f64, data, multiplier)
      }
      RegisterKind::String(_) => {
        let bytes = Self::parse_string_bytes(data)?;
        RegisterValue::String(String::from_utf8(bytes).ok()?)
      }
    };

    Some(value)
  }

  fn parse_numeric_bytes(data: Vec<u16>) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(data.len() * 2);

    #[cfg(target_endian = "little")]
    {
      for value in data.into_iter().rev() {
        bytes.push((value & 0xFF) as u8);
        bytes.push((value >> 8) as u8);
      }
    }
    #[cfg(target_endian = "big")]
    {
      for value in data.into_iter() {
        bytes.push((value & 0xFF) as u8);
        bytes.push((value >> 8) as u8);
      }
    }

    Some(bytes)
  }

  fn parse_string_bytes(data: Vec<u16>) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(data.len() * 2);

    #[cfg(target_endian = "little")]
    {
      for value in data.into_iter() {
        bytes.push((value >> 8) as u8);
        bytes.push((value & 0xFF) as u8);
      }
    }
    #[cfg(target_endian = "big")]
    {
      for value in data.into_iter() {
        bytes.push((value & 0xFF) as u8);
        bytes.push((value >> 8) as u8);
      }
    }

    Some(bytes)
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
              RegisterValue::U64(value) => serde_json::json!(value),
              RegisterValue::S16(value) => serde_json::json!(value),
              RegisterValue::S32(value) => serde_json::json!(value),
              RegisterValue::S64(value) => serde_json::json!(value),
              RegisterValue::F32(value) => serde_json::json!(value),
              RegisterValue::F64(value) => serde_json::json!(value),
              RegisterValue::String(value) => serde_json::json!(value),
            },
          )
        },
      )
      .collect::<serde_json::Map<String, serde_json::Value>>(),
  )
}
