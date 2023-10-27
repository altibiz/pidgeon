mod register;

use futures_time::future::FutureExt;
use register::*;
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
  Slave, SlaveId,
};

#[derive(Debug, Clone)]
pub struct DeviceConfig {
  pub kind: String,
  pub detect: Vec<DetectRegister<RegisterKind>>,
  pub id: Vec<IdRegister<RegisterKind>>,
  pub measurement: Vec<MeasurementRegister<RegisterKind>>,
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
    registers: Vec<IdRegister<RegisterKind>>,
  ) -> Option<String> {
    let mut id = format!("{kind}-");

    for register in registers {
      let value =
        Self::read_register(mutex.clone(), timeout, retries, register)
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
    register: DetectRegister<RegisterKind>,
  ) -> bool {
    let value = match Self::read_register(
      mutex.clone(),
      timeout,
      retries,
      register,
    )
    .await
    {
      Ok(value) => value,
      Err(_) => {
        return false;
      }
    };

    value.matches()
  }

  async fn read_register<
    TParsed: Register,
    TRegister: Register + UnparsedRegister<TRegister>,
  >(
    mutex: Arc<Mutex<Connection>>,
    timeout: futures_time::time::Duration,
    retries: u64,
    register: TRegister,
  ) -> Result<TParsed, ModbusClientError> {
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

    register.parse(&response)
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
}
