use tokio_modbus::{Address, Quantity};

use super::span::*;

// NITPICK: better error handling for span batching
// NITPICK: macros to implement batch parsing

#[derive(Clone, Debug)]
pub(crate) struct Batch<TSpan: Span> {
  pub(crate) address: Address,
  pub(crate) quantity: Quantity,
  pub(crate) inner: Vec<TSpan>,
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

macro_rules! parse_batch {
  ($self: ident, $data: ident, $timestamp: expr) => {{
    let mut inner = Vec::with_capacity($self.inner.len());
    let data = $data.into_iter().collect::<Vec<_>>();

    for span in &$self.inner {
      let start = Into::<usize>::into(
        span.address().checked_sub($self.address).ok_or_else(|| {
          anyhow::anyhow!(
            "Failed getting starting address for span {:?} {:?}",
            span.address(),
            span.quantity()
          )
        })?,
      );
      let end = start
        .checked_add(Into::<usize>::into(span.quantity()))
        .ok_or_else(|| {
          anyhow::anyhow!(
            "Failed getting ending address for span {:?} {:?}",
            span.address(),
            span.quantity()
          )
        })?;
      let slice = &data[start..end];
      let parsed =
        span.parse_with_timestamp(slice.iter().cloned(), $timestamp)?;
      inner.push(parsed);
    }

    Ok(Batch::<TSpan> {
      address: $self.address,
      quantity: $self.quantity,
      inner,
    })
  }};
}

macro_rules! impl_batch_span_parser {
  ($type: ty) => {
    impl<TSpan: Span, TSpanParser: Span + SpanParser<TSpan>>
      SpanParser<Batch<TSpan>> for $type
    {
      fn parse<TIterator, TIntoIterator>(
        &self,
        data: TIntoIterator,
      ) -> anyhow::Result<Batch<TSpan>>
      where
        TIterator: DoubleEndedIterator<Item = u16>,
        TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
      {
        parse_batch!(self, data, chrono::Utc::now())
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
        parse_batch!(self, data, timestamp)
      }
    }
  };
}

impl_batch_span_parser!(Batch<TSpanParser>);
impl_batch_span_parser!(&Batch<TSpanParser>);

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
    inner: vec![first],
  };

  for span in iter {
    #[allow(clippy::unwrap_used, reason = "it should panic")]
    let gap = span
      .address()
      .checked_sub(current.address.checked_add(current.quantity).unwrap())
      .unwrap();
    if gap < threshold {
      #[allow(clippy::unwrap_used, reason = "it should panic")]
      let quantity = span
        .address()
        .checked_add(span.quantity())
        .unwrap()
        .checked_sub(current.address)
        .unwrap();
      current.quantity = quantity;
      current.inner.push(span);
    } else {
      batches.push(current);
      current = Batch::<TSpan> {
        address: span.address(),
        quantity: span.quantity(),
        inner: vec![span],
      }
    }
  }
  batches.push(current);

  batches
}
