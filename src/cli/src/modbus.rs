use futures_time::{future::FutureExt, time::Duration};
use std::net::SocketAddr;
use thiserror::Error;
use tokio_modbus::{
    prelude::{tcp::connect_slave, Reader},
    Quantity, Slave,
};

use crate::config::{Config, Register, RegisterKind};

#[derive(Debug)]
pub struct RegisterData {
    pub register: Register,
    pub value: RegisterValue,
}

#[derive(Debug)]
pub enum RegisterValue {
    U16(u16),
    U32(u32),
    S16(i16),
    S32(i32),
}

#[derive(Debug, Error)]
pub enum ModbusError {
    #[error("Failed connecting to slave")]
    SlaveConnection(#[from] std::io::Error),

    #[error("Failed to parse register values")]
    Parse,
}

pub async fn read(
    socket: SocketAddr,
    slave: Slave,
    config: Config,
) -> Result<Vec<RegisterData>, ModbusError> {
    let mut ctx = connect_slave(socket, slave).await?;
    dbg!("connected");

    let mut data: Vec<RegisterData> = Vec::new();

    for register in config.registers {
        let register_size: Quantity = match register.kind {
            RegisterKind::U16 => 1,
            RegisterKind::U32 => 2,
            RegisterKind::S16 => 1,
            RegisterKind::S32 => 2,
        };

        let response = ctx
            .read_holding_registers(register.address, register_size)
            .timeout(Duration::from_millis(10000))
            .await??;

        let value = match register.kind {
            RegisterKind::U16 => match parse_u16_register(response) {
                Some(value) => RegisterValue::U16(value),
                None => return Err(ModbusError::Parse),
            },
            RegisterKind::U32 => match parse_u32_register(response) {
                Some(value) => RegisterValue::U32(value),
                None => return Err(ModbusError::Parse),
            },
            RegisterKind::S16 => match parse_s16_register(response) {
                Some(value) => RegisterValue::S16(value),
                None => return Err(ModbusError::Parse),
            },
            RegisterKind::S32 => match parse_s32_register(response) {
                Some(value) => RegisterValue::S32(value),
                None => return Err(ModbusError::Parse),
            },
        };

        data.push(RegisterData { register, value })
    }

    Ok(data)
}

pub fn parse_u16_register(data: Vec<u16>) -> Option<u16> {
    let first = data.get(0)?.clone();

    Some(first)
}

pub fn parse_u32_register(data: Vec<u16>) -> Option<u32> {
    let first = data.get(0)?.clone();
    let second = data.get(1)?.clone();

    Some(u32::from(first) << 16 | u32::from(second))
}

pub fn parse_s16_register(data: Vec<u16>) -> Option<i16> {
    let first = data.get(0)?.clone();

    Some(first as i16)
}

pub fn parse_s32_register(data: Vec<u16>) -> Option<i32> {
    let first = data.get(0)?.clone();
    let second = data.get(1)?.clone();

    Some((u32::from(first) << 16 | u32::from(second)) as i32)
}
