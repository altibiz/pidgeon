use tokio_modbus::{Address, Quantity};

pub trait Span {
  fn address(&self) -> Address;

  fn quantity(&self) -> Quantity;
}

pub trait SpanParser<TParsed: Span>: Span {
  fn parse<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
  ) -> Option<TParsed>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>;
}
