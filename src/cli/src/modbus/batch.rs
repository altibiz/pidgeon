use tokio_modbus::{Address, Quantity};

use super::span::*;

pub struct Batch<TSpan: Span> {
  pub address: Address,
  pub quantity: Quantity,
  pub spans: Vec<TSpan>,
}

impl<TSpan: Span> Span for Batch<TSpan> {
  fn address(&self) -> Address {
    self.address
  }

  fn quantity(&self) -> Quantity {
    self.quantity
  }
}

impl<TParsedSpan: Span, TUnparsedSpan: UnparsedSpan<TParsedSpan>>
  UnparsedSpan<Batch<TParsedSpan>> for Batch<TUnparsedSpan>
{
  fn parse<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
  ) -> Option<Batch<TParsedSpan>>
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

    Some(Batch::<TParsedSpan> {
      address: self.address,
      quantity: self.quantity,
      spans: registers,
    })
  }
}

pub fn batch_spans<TSpan: Span + Clone, TIntoIterator>(
  spans: TIntoIterator,
  threshold: usize,
) -> Vec<Batch<TSpan>>
where
  for<'a> &'a TIntoIterator: IntoIterator<Item = &'a TSpan>,
{
  let mut spans = spans.into_iter().cloned().collect::<Vec<_>>();
  spans.sort_by_key(|span| span.address());

  let mut batches = Vec::new();
  let mut current = Batch::<TSpan> {
    address: spans[0].address(),
    quantity: spans[0].quantity(),
    spans: vec![spans[0].clone()],
  };

  for span in spans.drain(1..) {
    let end = current.address + current.quantity;
    let gap = span.address() - end;
    if (gap as usize) < threshold {
      current.quantity += gap + span.quantity();
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
