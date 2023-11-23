use std::fmt::Debug;

use either::Either;
use tokio_modbus::{Address, Quantity};

pub(crate) trait Span {
  fn address(&self) -> Address;

  fn quantity(&self) -> Quantity;
}

pub(crate) trait SpanParser<TParsed: Span> {
  fn parse<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
  ) -> anyhow::Result<TParsed>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>;

  fn parse_with_timestamp<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
    timestamp: chrono::DateTime<chrono::Utc>,
  ) -> anyhow::Result<TParsed>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>;
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub(crate) struct SimpleSpan {
  pub(crate) address: u16,
  pub(crate) quantity: u16,
}

impl Span for SimpleSpan {
  fn address(&self) -> Address {
    self.address
  }

  fn quantity(&self) -> Quantity {
    self.quantity
  }
}

impl<TLeftSpan: Span, TRightSpan: Span> Span for Either<TLeftSpan, TRightSpan> {
  fn address(&self) -> Address {
    match self {
      Either::Left(span) => span.address(),
      Either::Right(span) => span.address(),
    }
  }

  fn quantity(&self) -> Quantity {
    match self {
      Either::Left(span) => span.quantity(),
      Either::Right(span) => span.quantity(),
    }
  }
}

impl<
    TLeftSpan: Span,
    TLeftSpanParser: SpanParser<TLeftSpan>,
    TRightSpan: Span,
    TRightSpanParser: SpanParser<TRightSpan>,
  > SpanParser<Either<TLeftSpan, TRightSpan>>
  for Either<TLeftSpanParser, TRightSpanParser>
{
  fn parse<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
  ) -> anyhow::Result<Either<TLeftSpan, TRightSpan>>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
  {
    match self {
      Either::Left(parser) => Ok(Either::Left(parser.parse(data)?)),
      Either::Right(parser) => Ok(Either::Right(parser.parse(data)?)),
    }
  }

  fn parse_with_timestamp<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
    timestamp: chrono::DateTime<chrono::Utc>,
  ) -> anyhow::Result<Either<TLeftSpan, TRightSpan>>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
  {
    match self {
      Either::Left(parser) => {
        Ok(Either::Left(parser.parse_with_timestamp(data, timestamp)?))
      }
      Either::Right(parser) => {
        Ok(Either::Right(parser.parse_with_timestamp(data, timestamp)?))
      }
    }
  }
}
