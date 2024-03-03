use std::fmt::Debug;
use std::fmt::Display;
use std::iter::IntoIterator;

use either::Either;
use regex::Regex;
use rust_decimal::Decimal;
use tokio_modbus::{Address, Quantity};

use super::encoding::*;
use super::record::*;
use super::span::*;

// TODO: make the MAX checking a config option

pub(crate) trait RegisterStorage {
  fn quantity(&self) -> Quantity;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StringRegisterKind {
  pub(crate) length: Quantity,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NumericRegisterKind {
  pub(crate) multiplier: Option<Decimal>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RawRegisterKind {
  pub(crate) length: Quantity,
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
  Raw(RawRegisterKind),
}

#[derive(Debug, Clone)]
pub(crate) struct RegisterValue<T> {
  pub(crate) value: T,
  pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub(crate) enum RegisterValueStorage {
  U16(RegisterValue<Decimal>),
  U32(RegisterValue<Decimal>),
  U64(RegisterValue<Decimal>),
  S16(RegisterValue<Decimal>),
  S32(RegisterValue<Decimal>),
  S64(RegisterValue<Decimal>),
  F32(RegisterValue<Decimal>),
  F64(RegisterValue<Decimal>),
  String(RegisterValue<String>),
  Raw(RegisterValue<Vec<u16>>),
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
      RegisterValueStorage::Raw(storage) => storage.timestamp,
    }
  }

  pub(crate) fn serialize(&self) -> serde_json::Value {
    match self {
      RegisterValueStorage::U16(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::U32(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::U64(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::S16(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::S32(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::S64(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::F32(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::F64(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::String(storage) => serde_json::json!(storage.value),
      RegisterValueStorage::Raw(storage) => {
        serde_json::json!(storage
          .value
          .iter()
          .map(|&num| format!("0x{:04X}", num))
          .collect::<Vec<_>>())
      }
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

#[derive(Debug, Clone)]
pub(crate) struct ValueRegister<T: RegisterStorage> {
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
      RegisterKindStorage::Raw(RawRegisterKind { length }) => *length,
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
      RegisterValueStorage::Raw(storage) => storage.value.len() as Quantity,
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
      RegisterValueStorage::Raw(storage) => {
        std::fmt::Debug::fmt(&storage.value.iter().map(|&num| Hex(num)), f)
      }
    }
  }
}

struct Hex(u16);

impl std::fmt::Debug for Hex {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "0x{:04X}", self.0)
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
         }| { (name.clone(), storage.serialize()) },
      )
      .collect::<serde_json::Map<String, serde_json::Value>>(),
  )
}

macro_rules! impl_display {
  ($type: ident) => {
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

impl_display!(MeasurementRegister);
impl_display!(DetectRegister);
impl_display!(IdRegister);
impl_display!(ValueRegister);

macro_rules! impl_span {
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
  };
}

impl_span!(MeasurementRegister);
impl_span!(DetectRegister);
impl_span!(IdRegister);
impl_span!(ValueRegister);

macro_rules! parse_integer_register {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident, $timestamp: expr) => {{
    let bytes = decode_numeric_bytes($data);
    let slice = bytes.as_slice().try_into()?;
    let mut typed = <$type>::from_ne_bytes(slice);
    if (typed == <$type>::MAX) {
      typed = 0;
    }

    let value = Decimal::from(typed);
    RegisterValueStorage::$variant(RegisterValue::<Decimal> {
      value: match $multiplier {
        Some($multiplier) => value
          .checked_mul($multiplier)
          .ok_or_else(|| anyhow::anyhow!("Failed multiplyting register"))?,
        None => value,
      },
      timestamp: $timestamp,
    })
  }};
}

macro_rules! parse_floating_register {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident, $timestamp: expr) => {{
    let bytes = decode_numeric_bytes($data);
    let slice = bytes.as_slice().try_into()?;
    let value = Decimal::try_from(<$type>::from_ne_bytes(slice))?;
    RegisterValueStorage::$variant(RegisterValue::<Decimal> {
      value: match $multiplier {
        Some($multiplier) => value
          .checked_mul($multiplier)
          .ok_or_else(|| anyhow::anyhow!("Failed multiplying register"))?,
        None => value,
      },
      timestamp: $timestamp,
    })
  }};
}

macro_rules! parse_register {
  ($self: ident, $data: ident, $result: expr, $timestamp: expr) => {{
    let value = match $self.storage {
      RegisterKindStorage::U16(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U16, u16, $data, multiplier, $timestamp)
      }
      RegisterKindStorage::U32(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U32, u32, $data, multiplier, $timestamp)
      }
      RegisterKindStorage::U64(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(U64, u64, $data, multiplier, $timestamp)
      }
      RegisterKindStorage::S16(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S16, i16, $data, multiplier, $timestamp)
      }
      RegisterKindStorage::S32(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S32, i32, $data, multiplier, $timestamp)
      }
      RegisterKindStorage::S64(NumericRegisterKind { multiplier }) => {
        parse_integer_register!(S64, i64, $data, multiplier, $timestamp)
      }
      RegisterKindStorage::F32(NumericRegisterKind { multiplier }) => {
        parse_floating_register!(F32, f32, $data, multiplier, $timestamp)
      }
      RegisterKindStorage::F64(NumericRegisterKind { multiplier }) => {
        parse_floating_register!(F64, f64, $data, multiplier, $timestamp)
      }
      RegisterKindStorage::String(_) => {
        let bytes = decode_string_bytes($data);
        RegisterValueStorage::String(RegisterValue::<String> {
          value: String::from_utf8(bytes)?,
          timestamp: $timestamp,
        })
      }
      RegisterKindStorage::Raw(_) => {
        RegisterValueStorage::Raw(RegisterValue::<Vec<u16>> {
          value: $data.into_iter().collect::<Vec<_>>(),
          timestamp: $timestamp,
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
        parse_register!(self, data, $result, chrono::Utc::now())
      }

      fn parse_with_timestamp<TIterator, TIntoIterator>(
        &self,
        data: TIntoIterator,
        timestamp: chrono::DateTime<chrono::Utc>,
      ) -> anyhow::Result<$type<RegisterValueStorage>>
      where
        TIterator: DoubleEndedIterator<Item = u16>,
        TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
      {
        parse_register!(self, data, $result, timestamp)
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
        parse_register!(self, data, $result, chrono::Utc::now())
      }

      fn parse_with_timestamp<TIterator, TIntoIterator>(
        &self,
        data: TIntoIterator,
        timestamp: chrono::DateTime<chrono::Utc>,
      ) -> anyhow::Result<$type<RegisterValueStorage>>
      where
        TIterator: DoubleEndedIterator<Item = u16>,
        TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
      {
        parse_register!(self, data, $result, timestamp)
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

impl_parse_register!(
  ValueRegister,
  |register: &ValueRegister::<RegisterKindStorage>, storage| {
    ValueRegister::<RegisterValueStorage> {
      address: register.address,
      storage,
    }
  }
);

macro_rules! serialize_numeric_register {
  ($type: ty, $value: ident, $default: expr) => {{
    let value = TryInto::<u16>::try_into(*$value).unwrap_or(0u16);
    let bytes = value.to_ne_bytes();
    encode_numeric_bytes(bytes).into_iter()
  }};
}

macro_rules! serialize_register {
  ($self: ident) => {{
    match &$self.storage {
      RegisterValueStorage::U16(RegisterValue::<Decimal> { value, .. }) => {
        serialize_numeric_register!(u16, value, 0u16)
      }
      RegisterValueStorage::U32(RegisterValue::<Decimal> { value, .. }) => {
        serialize_numeric_register!(u32, value, 0u32)
      }
      RegisterValueStorage::U64(RegisterValue::<Decimal> { value, .. }) => {
        serialize_numeric_register!(u64, value, 0u64)
      }
      RegisterValueStorage::S16(RegisterValue::<Decimal> { value, .. }) => {
        serialize_numeric_register!(i16, value, 0i16)
      }
      RegisterValueStorage::S32(RegisterValue::<Decimal> { value, .. }) => {
        serialize_numeric_register!(i32, value, 0i32)
      }
      RegisterValueStorage::S64(RegisterValue::<Decimal> { value, .. }) => {
        serialize_numeric_register!(i64, value, 0i64)
      }
      RegisterValueStorage::F32(RegisterValue::<Decimal> { value, .. }) => {
        serialize_numeric_register!(f32, value, 0f32)
      }
      RegisterValueStorage::F64(RegisterValue::<Decimal> { value, .. }) => {
        serialize_numeric_register!(f64, value, 0f64)
      }
      RegisterValueStorage::String(RegisterValue::<String> {
        value, ..
      }) => encode_string_bytes(value.as_str().bytes()).into_iter(),
      RegisterValueStorage::Raw(RegisterValue::<Vec<u16>> {
        value, ..
      }) => value.clone().into_iter(),
    }
  }};
}

macro_rules! impl_record {
  ($type: ident) => {
    impl Record for $type<RegisterValueStorage> {
      fn values(&self) -> impl Iterator<Item = u16> {
        serialize_register!(self)
      }
    }

    impl Record for &$type<RegisterValueStorage> {
      fn values(&self) -> impl Iterator<Item = u16> {
        serialize_register!(self)
      }
    }
  };
}

impl_record!(MeasurementRegister);
impl_record!(DetectRegister);
impl_record!(IdRegister);
impl_record!(ValueRegister);
