use chrono::{Datelike, Timelike};

use super::record::*;

#[derive(Debug, Clone, Copy)]
pub(crate) enum TimeImplementation {
  SchneideriEM3xxx,
}

pub(crate) trait Time {
  fn create(&self) -> SimpleRecord;
}

pub(crate) fn implementation_for(
  implementation: TimeImplementation,
) -> impl Time {
  match implementation {
    TimeImplementation::SchneideriEM3xxx => SchneideriEM3xxxTime::new(),
  }
}

pub(crate) struct SchneideriEM3xxxTime {}

impl SchneideriEM3xxxTime {
  fn new() -> Self {
    Self {}
  }
}

impl Time for SchneideriEM3xxxTime {
  fn create(&self) -> SimpleRecord {
    let now = chrono::Utc::now();

    let values = vec![
      1003,
      0,
      now.year() as u16,
      now.month() as u16,
      now.day() as u16,
      now.hour() as u16,
      now.minute() as u16,
      now.second() as u16,
      0,
    ];

    SimpleRecord {
      address: 5250,
      values,
    }
  }
}
