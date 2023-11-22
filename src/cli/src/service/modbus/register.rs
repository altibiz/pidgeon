use std::fmt::Debug;
use std::fmt::Display;

use either::Either;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio_modbus::{Address, Quantity};

use super::span::*;

pub(crate) trait RegisterStorage {
  fn quantity(&self) -> Quantity;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StringRegisterKind {
  pub(crate) length: Quantity,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NumericRegisterKind {
  pub(crate) multiplier: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RegisterKindStorage {
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
pub(crate) struct RegisterValue<T> {
  pub(crate) value: T,
  pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum RegisterValueStorage {
  U16(RegisterValue<u16>),
  U32(RegisterValue<u32>),
  U64(RegisterValue<u64>),
  S16(RegisterValue<i16>),
  S32(RegisterValue<i32>),
  S64(RegisterValue<i64>),
  F32(RegisterValue<f32>),
  F64(RegisterValue<f64>),
  String(RegisterValue<String>),
}

impl RegisterValueStorage {
  pub(crate) fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
    match self {
      RegisterValueStorage::U16(storage) => storage.timestamp,
      RegisterValueStorage::U32(storage) => storage.timestamp,
      RegisterValueStorage::U64(storage) => storage.timestamp,
      RegisterValueStorage::S16(storage) => storage.timestamp,
      RegisterValueStorage::S32(storage) => storage.timestamp,
      RegisterValueStorage::S64(storage) => storage.timestamp,
      RegisterValueStorage::F32(storage) => storage.timestamp,
      RegisterValueStorage::F64(storage) => storage.timestamp,
      RegisterValueStorage::String(storage) => storage.timestamp,
    }
  }
}

#[derive(Debug, Clone)]
pub(crate) struct MeasurementRegister<T: RegisterStorage> {
  pub(crate) address: Address,
  pub(crate) storage: T,
  pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DetectRegister<T: RegisterStorage> {
  pub(crate) address: Address,
  pub(crate) storage: T,
  pub(crate) r#match: Either<String, Regex>,
}

#[derive(Debug, Clone)]
pub(crate) struct IdRegister<T: RegisterStorage> {
  pub(crate) address: Address,
  pub(crate) storage: T,
}

pub(crate) fn make_id<
  TIntoIterator: IntoIterator<Item = IdRegister<RegisterValueStorage>>,
>(
  kind: String,
  id_registers: TIntoIterator,
) -> String {
  id_registers
    .into_iter()
    .map(|id| id.to_string())
    .fold(format!("{kind}-"), |acc, next| acc + next.as_str())
}

impl RegisterStorage for RegisterKindStorage {
  fn quantity(&self) -> Quantity {
    match self {
      RegisterKindStorage::U16(_) => 1,
      RegisterKindStorage::U32(_) => 2,
      RegisterKindStorage::U64(_) => 4,
      RegisterKindStorage::S16(_) => 1,
      RegisterKindStorage::S32(_) => 2,
      RegisterKindStorage::S64(_) => 4,
      RegisterKindStorage::F32(_) => 2,
      RegisterKindStorage::F64(_) => 4,
      RegisterKindStorage::String(StringRegisterKind { length }) => *length,
    }
  }
}

impl RegisterStorage for RegisterValueStorage {
  fn quantity(&self) -> Quantity {
    match self {
      RegisterValueStorage::U16(_) => 1,
      RegisterValueStorage::U32(_) => 2,
      RegisterValueStorage::U64(_) => 4,
      RegisterValueStorage::S16(_) => 1,
      RegisterValueStorage::S32(_) => 2,
      RegisterValueStorage::S64(_) => 4,
      RegisterValueStorage::F32(_) => 2,
      RegisterValueStorage::F64(_) => 4,
      RegisterValueStorage::String(storage) => storage.value.len() as Quantity,
    }
  }
}

impl Display for RegisterValueStorage {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> Result<(), std::fmt::Error> {
    match self {
      RegisterValueStorage::U16(storage) => {
        std::fmt::Display::fmt(&storage.value, f)
      }
      RegisterValueStorage::U32(storage) => {
        std::fmt::Display::fmt(&storage.value, f)
      }
      RegisterValueStorage::U64(storage) => {
        std::fmt::Display::fmt(&storage.value, f)
      }
      RegisterValueStorage::S16(storage) => {
        std::fmt::Display::fmt(&storage.value, f)
      }
      RegisterValueStorage::S32(storage) => {
        std::fmt::Display::fmt(&storage.value, f)
      }
      RegisterValueStorage::S64(storage) => {
        std::fmt::Display::fmt(&storage.value, f)
      }
      RegisterValueStorage::F32(storage) => {
        std::fmt::Display::fmt(&storage.value, f)
      }
      RegisterValueStorage::F64(storage) => {
        std::fmt::Display::fmt(&storage.value, f)
      }
      RegisterValueStorage::String(storage) => {
        std::fmt::Debug::fmt(&storage.value, f)
      }
    }
  }
}

impl DetectRegister<RegisterValueStorage> {
  pub(crate) fn matches(&self) -> bool {
    let storage = self.storage.to_string();
    match &self.r#match {
      Either::Left(string) => string.eq(storage.as_str()),
      Either::Right(regex) => regex.is_match(storage.as_str()),
    }
  }
}

pub(crate) fn serialize_registers<
  TIntoIterator: IntoIterator<Item = MeasurementRegister<RegisterValueStorage>>,
>(
  registers: TIntoIterator,
) -> serde_json::Value {
  serde_json::Value::Object(
    registers
      .into_iter()
      .map(
        |MeasurementRegister::<RegisterValueStorage> {
           name, storage, ..
         }| { (name.clone(), serde_json::json!(storage)) },
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

    impl<T: RegisterStorage> Span for &$type<T> {
      fn address(&self) -> Address {
        self.address
      }

      fn quantity(&self) -> Quantity {
        self.storage.quantity()
      }
    }

    impl Display for $type<RegisterValueStorage> {
      fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
      ) -> Result<(), std::fmt::Error> {
        std::fmt::Display::fmt(&self.storage, f)
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
    RegisterValueStorage::$variant(RegisterValue::<$type> {
      value: match $multiplier {
        Some($multiplier) => ((value as f64) * $multiplier).round() as $type,
        None => value,
      },
      timestamp: chrono::Utc::now(),
    })
  }};
}

macro_rules! parse_floating_register {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident) => {{
    let bytes = parse_numeric_bytes($data);
    let slice = bytes.as_slice().try_into()?;
    let value = <$type>::from_ne_bytes(slice);
    RegisterValueStorage::$variant(RegisterValue::<$type> {
      value: match $multiplier {
        Some($multiplier) => ((value as f64) * $multiplier) as $type,
        None => value,
      },
      timestamp: chrono::Utc::now(),
    })
  }};
}

macro_rules! parse_register {
  ($self: ident, $data: ident, $result: expr) => {{
    let value = match $self.storage {
      RegisterKindStorage::U16(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U16, u16, $data, multiplier)
      }
      RegisterKindStorage::U32(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U32, u32, $data, multiplier)
      }
      RegisterKindStorage::U64(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U64, u64, $data, multiplier)
      }
      RegisterKindStorage::S16(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S16, i16, $data, multiplier)
      }
      RegisterKindStorage::S32(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S32, i32, $data, multiplier)
      }
      RegisterKindStorage::S64(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S64, i64, $data, multiplier)
      }
      RegisterKindStorage::F32(NumericRegisterKind { multiplier }) => {
        parse_floating_register!(F32, f32, $data, multiplier)
      }
      RegisterKindStorage::F64(NumericRegisterKind { multiplier }) => {
        parse_floating_register!(F64, f64, $data, multiplier)
      }
      RegisterKindStorage::String(_) => {
        let bytes = parse_string_bytes($data);
        RegisterValueStorage::String(RegisterValue::<String> {
          value: String::from_utf8(bytes)?,
          timestamp: chrono::Utc::now(),
        })
      }
    };

    #[allow(clippy::redundant_closure_call)]
    Ok($result($self, value))
  }};
}

macro_rules! impl_parse_register {
  ($type: ident, $result: expr) => {
    impl SpanParser<$type<RegisterValueStorage>>
      for $type<RegisterKindStorage>
    {
      fn parse<TIterator, TIntoIterator>(
        &self,
        data: TIntoIterator,
      ) -> anyhow::Result<$type<RegisterValueStorage>>
      where
        TIterator: DoubleEndedIterator<Item = u16>,
        TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
      {
        parse_register!(self, data, $result)
      }
    }

    impl SpanParser<$type<RegisterValueStorage>>
      for &$type<RegisterKindStorage>
    {
      fn parse<TIterator, TIntoIterator>(
        &self,
        data: TIntoIterator,
      ) -> anyhow::Result<$type<RegisterValueStorage>>
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
  |register: &MeasurementRegister::<RegisterKindStorage>, storage| {
    MeasurementRegister::<RegisterValueStorage> {
      address: register.address,
      storage,
      name: register.name.clone(),
    }
  }
);
impl_parse_register!(
  DetectRegister,
  |register: &DetectRegister::<RegisterKindStorage>, storage| {
    DetectRegister::<RegisterValueStorage> {
      address: register.address,
      storage,
      r#match: register.r#match.clone(),
    }
  }
);
impl_parse_register!(IdRegister, |register: &IdRegister::<
  RegisterKindStorage,
>,
                                  storage| {
  IdRegister::<RegisterValueStorage> {
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
