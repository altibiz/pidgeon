use futures_time::{future::FutureExt, time::Duration};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_modbus::{
    client::Context,
    prelude::{tcp::connect_slave, Reader},
    Address, Quantity, Slave, SlaveId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Register {
    pub name: String,
    pub address: Address,
    pub kind: RegisterKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegisterKind {
    U16,
    U32,
    S16,
    S32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterData {
    pub register: Register,
    pub value: RegisterValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegisterValue {
    U16(u16),
    U32(u32),
    S16(i16),
    S32(i32),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ConnectionId {
    pub socket: SocketAddr,
    pub slave_id: SlaveId,
}

#[derive(Debug)]
struct Connection {
    pub ctx: Context,
}

// TODO: detect whether the device is on and clean up connection if not
pub struct ModbusClient {
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
    pub fn new() -> Result<Self, ModbusClientError> {
        Ok(Self {
            pool: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn read(
        &self,
        ip: IpAddr,
        slave: SlaveId,
        registers: Vec<Register>,
    ) -> Result<Vec<RegisterData>, ModbusClientError> {
        let mutex = self.conn(ip, slave).await?;

        let mut data: Vec<RegisterData> = Vec::new();
        for register in registers {
            let register_size: Quantity = match register.kind {
                RegisterKind::U16 => 1,
                RegisterKind::U32 => 2,
                RegisterKind::S16 => 1,
                RegisterKind::S32 => 2,
            };

            let response = {
                let mut conn = mutex.lock().await;
                conn.ctx
                    .read_holding_registers(register.address, register_size)
                    .timeout(Duration::from_secs(10))
                    .await??
            };

            let value = match register.kind {
                RegisterKind::U16 => match Self::parse_u16_register(response) {
                    Some(value) => RegisterValue::U16(value),
                    None => return Err(ModbusClientError::Parse),
                },
                RegisterKind::U32 => match Self::parse_u32_register(response) {
                    Some(value) => RegisterValue::U32(value),
                    None => return Err(ModbusClientError::Parse),
                },
                RegisterKind::S16 => match Self::parse_s16_register(response) {
                    Some(value) => RegisterValue::S16(value),
                    None => return Err(ModbusClientError::Parse),
                },
                RegisterKind::S32 => match Self::parse_s32_register(response) {
                    Some(value) => RegisterValue::S32(value),
                    None => return Err(ModbusClientError::Parse),
                },
            };

            data.push(RegisterData { register, value })
        }

        Ok(data)
    }

    async fn conn(
        &self,
        ip: IpAddr,
        slave: SlaveId,
    ) -> Result<Arc<Mutex<Connection>>, ModbusClientError> {
        let id = ConnectionId {
            socket: match ip {
                IpAddr::V4(ipv4) => SocketAddr::V4(SocketAddrV4::new(ipv4, 502)),
                IpAddr::V6(_) => return Err(ModbusClientError::Ipv6),
            },
            slave_id: slave,
        };

        let mut map = self.pool.lock().await;
        let mutex = match map.get(&id) {
            Some(mutex) => mutex.clone(),
            None => {
                let ctx = connect_slave(id.socket.clone(), Slave(id.slave_id.clone()))
                    .timeout(Duration::from_secs(10))
                    .await??;
                let conn = Connection { ctx };
                let mutex = Arc::new(Mutex::new(conn));
                map.entry(id).or_insert(mutex).clone()
            }
        };

        Ok(mutex)
    }

    fn parse_u16_register(data: Vec<u16>) -> Option<u16> {
        let first = data.get(0)?.clone();

        Some(first)
    }

    fn parse_u32_register(data: Vec<u16>) -> Option<u32> {
        let first = data.get(0)?.clone();
        let second = data.get(1)?.clone();

        Some(u32::from(first) << 16 | u32::from(second))
    }

    fn parse_s16_register(data: Vec<u16>) -> Option<i16> {
        let first = data.get(0)?.clone();

        Some(first as i16)
    }

    fn parse_s32_register(data: Vec<u16>) -> Option<i32> {
        let first = data.get(0)?.clone();
        let second = data.get(1)?.clone();

        Some((u32::from(first) << 16 | u32::from(second)) as i32)
    }
}

pub fn registers_to_json(registers: Vec<RegisterData>) -> serde_json::Value {
    serde_json::Value::Object(
        registers
            .iter()
            .map(
                |RegisterData {
                     register: Register { name, .. },
                     value,
                 }| {
                    (
                        name.clone(),
                        match value {
                            RegisterValue::U16(value) => serde_json::json!(value),
                            RegisterValue::U32(value) => serde_json::json!(value),
                            RegisterValue::S16(value) => serde_json::json!(value),
                            RegisterValue::S32(value) => serde_json::json!(value),
                        },
                    )
                },
            )
            .collect::<serde_json::Map<String, serde_json::Value>>(),
    )
}
