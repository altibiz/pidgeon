use std::fmt::Display;

use either::Either;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio_modbus::{Address, Quantity};

use super::span::*;

pub trait RegisterStorage {
  fn quantity(&self) -> Quantity;
}

pub trait Register: Span {
  fn storage<'a>(&'a self) -> &'a dyn RegisterStorage;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
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

#[derive(Debug, Clone)]
pub struct MeasurementRegister<T: RegisterStorage> {
  pub address: Address,
  pub storage: T,
  pub name: String,
}

#[derive(Debug, Clone)]
pub struct DetectRegister<T: RegisterStorage> {
  pub address: Address,
  pub storage: T,
  pub r#match: Either<String, Regex>,
}

#[derive(Debug, Clone)]
pub struct IdRegister<T: RegisterStorage> {
  pub address: Address,
  pub storage: T,
}

impl RegisterStorage for RegisterKind {
  fn quantity(&self) -> Quantity {
    match self {
      RegisterKind::U16(_) => 1,
      RegisterKind::U32(_) => 2,
      RegisterKind::U64(_) => 4,
      RegisterKind::S16(_) => 1,
      RegisterKind::S32(_) => 2,
      RegisterKind::S64(_) => 4,
      RegisterKind::F32(_) => 2,
      RegisterKind::F64(_) => 4,
      RegisterKind::String(StringRegisterKind { length }) => *length,
    }
  }
}

impl RegisterStorage for RegisterValue {
  fn quantity(&self) -> Quantity {
    match self {
      RegisterValue::U16(_) => 1,
      RegisterValue::U32(_) => 2,
      RegisterValue::U64(_) => 4,
      RegisterValue::S16(_) => 1,
      RegisterValue::S32(_) => 2,
      RegisterValue::S64(_) => 4,
      RegisterValue::F32(_) => 2,
      RegisterValue::F64(_) => 4,
      RegisterValue::String(value) => value.len() as Quantity,
    }
  }
}

impl Display for RegisterValue {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> Result<(), std::fmt::Error> {
    match self {
      RegisterValue::U16(value) => value.fmt(f),
      RegisterValue::U32(value) => value.fmt(f),
      RegisterValue::U64(value) => value.fmt(f),
      RegisterValue::S16(value) => value.fmt(f),
      RegisterValue::S32(value) => value.fmt(f),
      RegisterValue::S64(value) => value.fmt(f),
      RegisterValue::F32(value) => value.fmt(f),
      RegisterValue::F64(value) => value.fmt(f),
      RegisterValue::String(value) => value.fmt(f),
    }
  }
}

impl DetectRegister<RegisterValue> {
  pub fn matches(&self) -> bool {
    let storage = self.storage.to_string();
    match &self.r#match {
      Either::Left(string) => string.eq(storage.as_str()),
      Either::Right(regex) => regex.is_match(storage.as_str()),
    }
  }
}

pub fn serialize_registers<TIntoIterator>(
  registers: TIntoIterator,
) -> serde_json::Value
where
  for<'a> &'a TIntoIterator:
    IntoIterator<Item = &'a MeasurementRegister<RegisterValue>>,
{
  serde_json::Value::Object(
    registers
      .into_iter()
      .map(
        |MeasurementRegister::<RegisterValue> { name, storage, .. }| {
          (name.clone(), serde_json::json!(storage))
        },
      )
      .collect::<serde_json::Map<String, serde_json::Value>>(),
  )
}

macro_rules! impl_register {
  ($type: ident) => {
    impl<T: RegisterStorage> Span for $type<T> {
      fn address(&self) -> Address {
        self.address
      }

      fn quantity(&self) -> Quantity {
        self.storage.quantity()
      }
    }

    impl<T: RegisterStorage> Register for $type<T> {
      fn storage<'a>(&'a self) -> &'a dyn RegisterStorage {
        &self.storage
      }
    }

    impl Display for $type<RegisterValue> {
      fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
      ) -> Result<(), std::fmt::Error> {
        self.storage.fmt(f)
      }
    }
  };
}

impl_register!(MeasurementRegister);
impl_register!(DetectRegister);
impl_register!(IdRegister);

macro_rules! parse_integer_register {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident) => {{
    let bytes = parse_numeric_bytes($data);
    let slice = bytes.as_slice().try_into()?;
    let value = <$type>::from_ne_bytes(slice);
    RegisterValue::$variant(match $multiplier {
      Some($multiplier) => ((value as f64) * $multiplier).round() as $type,
      None => value,
    })
  }};
}

macro_rules! parse_floating_register {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident) => {{
    let bytes = parse_numeric_bytes($data);
    let slice = bytes.as_slice().try_into()?;
    let value = <$type>::from_ne_bytes(slice);
    RegisterValue::$variant(match $multiplier {
      Some($multiplier) => ((value as f64) * $multiplier) as $type,
      None => value,
    })
  }};
}

macro_rules! parse_register {
  ($self: ident, $data: ident, $result: expr) => {{
    let value = match $self.storage {
      RegisterKind::U16(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U16, u16, $data, multiplier)
      }
      RegisterKind::U32(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U32, u32, $data, multiplier)
      }
      RegisterKind::U64(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U64, u64, $data, multiplier)
      }
      RegisterKind::S16(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S16, i16, $data, multiplier)
      }
      RegisterKind::S32(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S32, i32, $data, multiplier)
      }
      RegisterKind::S64(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S64, i64, $data, multiplier)
      }
      RegisterKind::F32(NumericRegisterKind { multiplier }) => {
        parse_floating_register!(F32, f32, $data, multiplier)
      }
      RegisterKind::F64(NumericRegisterKind { multiplier }) => {
        parse_floating_register!(F64, f64, $data, multiplier)
      }
      RegisterKind::String(_) => {
        let bytes = parse_string_bytes($data);
        RegisterValue::String(String::from_utf8(bytes)?)
      }
    };

    Ok($result($self, value))
  }};
}

macro_rules! impl_parse_register {
  ($type: ident, $result: expr) => {
    #[cfg(target_endian = "little")]
    impl SpanParser<$type<RegisterValue>> for $type<RegisterKind> {
      fn parse<TIterator, TIntoIterator>(
        &self,
        data: TIntoIterator,
      ) -> anyhow::Result<$type<RegisterValue>>
      where
        TIterator: DoubleEndedIterator<Item = u16>,
        TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
      {
        parse_register!(self, data, $result)
      }
    }
  };
}

impl_parse_register!(
  MeasurementRegister,
  |register: &MeasurementRegister::<RegisterKind>, storage| {
    MeasurementRegister::<RegisterValue> {
      address: register.address,
      storage,
      name: register.name.clone(),
    }
  }
);
impl_parse_register!(
  DetectRegister,
  |register: &DetectRegister::<RegisterKind>, storage| {
    DetectRegister::<RegisterValue> {
      address: register.address,
      storage,
      r#match: register.r#match.clone(),
    }
  }
);
impl_parse_register!(IdRegister, |register: &IdRegister::<RegisterKind>,
                                  storage| {
  IdRegister::<RegisterValue> {
    address: register.address,
    storage,
  }
});

#[cfg(target_endian = "little")]
fn parse_numeric_bytes<TIterator, TIntoIterator>(data: TIntoIterator) -> Vec<u8>
where
  TIterator: DoubleEndedIterator<Item = u16>,
  TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
{
  data
    .into_iter()
    .rev()
    .flat_map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .collect()
}

#[cfg(target_endian = "big")]
fn parse_numeric_bytes<TIntoIterator>(data: TIntoIterator) -> Vec<u8>
where
  TIntoIterator: IntoIterator<Item = u16>,
{
  data
    .into_iter()
    .flat_map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .collect()
}

#[cfg(target_endian = "little")]
fn parse_string_bytes<TIntoIterator>(data: TIntoIterator) -> Vec<u8>
where
  TIntoIterator: IntoIterator<Item = u16>,
{
  data
    .into_iter()
    .flat_map(|value| [(value >> 8) as u8, (value & 0xFF) as u8])
    .collect()
}

#[cfg(target_endian = "big")]
fn parse_string_bytes<TIntoIterator>(data: TIntoIterator) -> Vec<u8>
where
  TIntoIterator: IntoIterator<Item = u16>,
{
  data
    .into_iter()
    .flat_map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .collect()
}
