use tokio_modbus::{Address, Quantity};

use super::span::*;

// TODO: check what causes overflows on add/subtract

#[derive(Clone, Debug)]
pub(crate) struct Batch<TSpan: Span> {
  pub(crate) address: Address,
  pub(crate) quantity: Quantity,
  pub(crate) spans: Vec<TSpan>,
}

impl<TSpan: Span> Span for Batch<TSpan> {
  fn address(&self) -> Address {
    self.address
  }

  fn quantity(&self) -> Quantity {
    self.quantity
  }
}

impl<TSpan: Span> Span for &Batch<TSpan> {
  fn address(&self) -> Address {
    self.address
  }

  fn quantity(&self) -> Quantity {
    self.quantity
  }
}

impl<TSpan: Span, TSpanParser: Span + SpanParser<TSpan>>
  SpanParser<Batch<TSpan>> for Batch<TSpanParser>
{
  fn parse<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
  ) -> anyhow::Result<Batch<TSpan>>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
  {
    let mut registers = Vec::with_capacity(self.spans.len());
    let data = data.into_iter().collect::<Vec<_>>();

    for register in &self.spans {
      let start = (register.address() - self.address) as usize;
      let end = start + register.quantity() as usize;
      let slice = &data[start..end];
      let parsed = register.parse(slice.iter().cloned())?;
      registers.push(parsed);
    }

    Ok(Batch::<TSpan> {
      address: self.address,
      quantity: self.quantity,
      spans: registers,
    })
  }

  fn parse_with_timestamp<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
    timestamp: chrono::DateTime<chrono::Utc>,
  ) -> anyhow::Result<Batch<TSpan>>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
  {
    let mut registers = Vec::with_capacity(self.spans.len());
    let data = data.into_iter().collect::<Vec<_>>();

    for register in &self.spans {
      let start = (register.address() - self.address) as usize;
      let end = start + register.quantity() as usize;
      let slice = &data[start..end];
      let parsed =
        register.parse_with_timestamp(slice.iter().cloned(), timestamp)?;
      registers.push(parsed);
    }

    Ok(Batch::<TSpan> {
      address: self.address,
      quantity: self.quantity,
      spans: registers,
    })
  }
}

impl<TSpan: Span, TSpanParser: Span + SpanParser<TSpan>>
  SpanParser<Batch<TSpan>> for &Batch<TSpanParser>
{
  fn parse<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
  ) -> anyhow::Result<Batch<TSpan>>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
  {
    let mut registers = Vec::with_capacity(self.spans.len());
    let data = data.into_iter().collect::<Vec<_>>();

    for register in &self.spans {
      let start = (register.address() - self.address) as usize;
      let end = start + register.quantity() as usize;
      let slice = &data[start..end];
      let parsed = register.parse(slice.iter().cloned())?;
      registers.push(parsed);
    }

    Ok(Batch::<TSpan> {
      address: self.address,
      quantity: self.quantity,
      spans: registers,
    })
  }

  fn parse_with_timestamp<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
    timestamp: chrono::DateTime<chrono::Utc>,
  ) -> anyhow::Result<Batch<TSpan>>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
  {
    let mut registers = Vec::with_capacity(self.spans.len());
    let data = data.into_iter().collect::<Vec<_>>();

    for register in &self.spans {
      let start = (register.address() - self.address) as usize;
      let end = start + register.quantity() as usize;
      let slice = &data[start..end];
      let parsed =
        register.parse_with_timestamp(slice.iter().cloned(), timestamp)?;
      registers.push(parsed);
    }

    Ok(Batch::<TSpan> {
      address: self.address,
      quantity: self.quantity,
      spans: registers,
    })
  }
}

pub(crate) fn batch_spans<
  TSpan: Span,
  TIntoIterator: IntoIterator<Item = TSpan>,
>(
  spans: TIntoIterator,
  threshold: u16,
) -> Vec<Batch<TSpan>> {
  let mut spans = spans.into_iter().collect::<Vec<_>>();
  spans.sort_by_key(|span| span.address());

  let mut iter = spans.into_iter();
  let first = match iter.by_ref().next() {
    Some(first) => first,
    None => return Vec::new(),
  };
  let mut batches = Vec::new();
  let mut current = Batch::<TSpan> {
    address: first.address(),
    quantity: first.quantity(),
    spans: vec![first],
  };

  for span in iter {
    let gap = span
      .address()
      .saturating_sub(current.address.saturating_add(current.quantity));
    if gap < threshold {
      let quantity = span
        .address()
        .saturating_add(span.quantity())
        .saturating_sub(current.address);
      current.quantity = quantity;
      current.spans.push(span);
    } else {
      batches.push(current);
      current = Batch::<TSpan> {
        address: span.address(),
        quantity: span.quantity(),
        spans: vec![span],
      }
    }
  }
  batches.push(current);

  batches
}
