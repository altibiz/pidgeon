use itertools::Itertools;

#[cfg(target_endian = "little")]
pub(crate) fn decode_numeric_bytes<TIterator, TIntoIterator>(
  data: TIntoIterator,
) -> Vec<u8>
where
  TIterator: DoubleEndedIterator<Item = u16>,
  TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
{
  let data = data.into_iter().collect::<Vec<_>>();

  if data.iter().all(|register| *register == u16::MAX) {
    return data.iter().flat_map(|_| [u8::MIN, u8::MIN]).collect();
  }

  data
    .iter()
    .rev()
    .flat_map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .collect()
}

#[cfg(target_endian = "big")]
pub(crate) fn decode_numeric_bytes<TIntoIterator>(
  data: TIntoIterator,
) -> Vec<u8>
where
  TIntoIterator: IntoIterator<Item = u16>,
{
  let data = data.into_iter().collect::<Vec<_>>();

  if data.iter().all(|register| *register == u16::MAX) {
    return data.iter().flat_map(|_| [u8::MIN, u8::MIN]).collect();
  }

  data
    .iter()
    .flat_map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .collect()
}

#[cfg(target_endian = "little")]
pub(crate) fn decode_string_bytes<TIntoIterator>(data: TIntoIterator) -> Vec<u8>
where
  TIntoIterator: IntoIterator<Item = u16>,
{
  data
    .into_iter()
    .flat_map(|value| [(value >> 8) as u8, (value & 0xFF) as u8])
    .collect()
}

#[cfg(target_endian = "big")]
pub(crate) fn decode_string_bytes<TIntoIterator>(data: TIntoIterator) -> Vec<u8>
where
  TIntoIterator: IntoIterator<Item = u16>,
{
  data
    .into_iter()
    .flat_map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .collect()
}

#[cfg(target_endian = "little")]
pub(crate) fn encode_numeric_bytes<TIterator, TIntoIterator>(
  data: TIntoIterator,
) -> Vec<u16>
where
  TIterator: DoubleEndedIterator<Item = u8>,
  TIntoIterator: IntoIterator<Item = u8, IntoIter = TIterator>,
{
  data
    .into_iter()
    .rev()
    .chunks(2)
    .into_iter()
    .map(|mut chunk| {
      let first = chunk.next().unwrap_or(0u8);
      let second = chunk.next().unwrap_or(0u8);
      u16::from_le_bytes([first, second])
    })
    .collect()
}

#[cfg(target_endian = "big")]
pub(crate) fn encode_numeric_bytes<TIntoIterator>(
  data: TIntoIterator,
) -> Vec<u16>
where
  TIntoIterator: IntoIterator<Item = u8>,
{
  data
    .into_iter()
    .chunks(2)
    .into_iter()
    .map(|mut chunk| {
      let first = chunk.next().unwrap_or(0u8);
      let second = chunk.next().unwrap_or(0u8);
      u16::from_le_bytes([first, second])
    })
    .collect()
}

#[cfg(target_endian = "little")]
pub(crate) fn encode_string_bytes<TIntoIterator>(
  data: TIntoIterator,
) -> Vec<u16>
where
  TIntoIterator: IntoIterator<Item = u8>,
{
  data
    .into_iter()
    .chunks(2)
    .into_iter()
    .map(|mut chunk| {
      let first = chunk.next().unwrap_or(0u8);
      let second = chunk.next().unwrap_or(0u8);
      u16::from_le_bytes([second, first])
    })
    .collect()
}

#[cfg(target_endian = "big")]
pub(crate) fn encode_string_bytes<TIntoIterator>(
  data: TIntoIterator,
) -> Vec<u16>
where
  TIntoIterator: IntoIterator<Item = u8>,
{
  data
    .into_iter()
    .chunks(2)
    .into_iter()
    .map(|mut chunk| {
      let first = chunk.next().unwrap_or(0u8);
      let second = chunk.next().unwrap_or(0u8);
      u16::from_le_bytes([first, second])
    })
    .collect()
}
