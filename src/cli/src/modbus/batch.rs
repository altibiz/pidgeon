use tokio_modbus::{Address, Quantity};

use super::register::*;

pub trait RegisterBatch {
  fn address(&self) -> Address;

  fn quantity(&self) -> Quantity;
}

pub trait UnparsedRegisterBatch<TParsed: RegisterBatch>: RegisterBatch {
  fn parse<TIterator: IntoIterator<Item = u16>>(
    &self,
    data: &TIterator,
  ) -> Option<TParsed>;
}

pub struct SortedRegisterBatch<TRegister: Register> {
  pub address: Address,
  pub quantity: Quantity,
  pub registers: Vec<TRegister>,
}

impl<TRegister: Register> RegisterBatch for SortedRegisterBatch<TRegister> {
  fn address(&self) -> Address {
    self.address
  }

  fn quantity(&self) -> Quantity {
    self.quantity
  }
}

impl<
    TParsedRegister: Register,
    TUnparsedRegister: UnparsedRegister<TParsedRegister>,
  > UnparsedRegisterBatch<SortedRegisterBatch<TParsedRegister>>
  for SortedRegisterBatch<TUnparsedRegister>
{
  fn parse<TIterator: IntoIterator<Item = u16>>(
    &self,
    data: &TIterator,
  ) -> Option<SortedRegisterBatch<TParsedRegister>> {
    let mut registers = Vec::with_capacity(self.registers.len());
    let data = data.into_iter().collect::<Vec<_>>();

    for register in self.registers {
      let start = (register.address() - self.address) as usize;
      let end = start + register.quantity() as usize;
      let slice = &data[start..end];
      let parsed = register.parse(&slice.iter().cloned())?;
      registers.push(parsed);
    }

    Some(SortedRegisterBatch::<TParsedRegister> {
      address: self.address,
      quantity: self.quantity,
      registers,
    })
  }
}

pub fn batch_registers<
  TRegister: Register + Clone,
  TIntoIterator: IntoIterator<Item = TRegister>,
>(
  registers: TIntoIterator,
  threshold: usize,
) -> Vec<SortedRegisterBatch<TRegister>> {
  let mut registers = registers.into_iter().collect::<Vec<_>>();
  registers.sort_by_key(|register| register.address());

  let mut batches = Vec::new();
  let mut current = SortedRegisterBatch::<TRegister> {
    address: registers[0].address(),
    quantity: registers[0].storage().quantity(),
    registers: vec![registers[0].clone()],
  };

  for register in registers.drain(1..) {
    let end = current.address + current.quantity;
    let gap = register.address() - end;
    if (gap as usize) < threshold {
      current.quantity += gap + register.storage().quantity();
      current.registers.push(register);
    } else {
      batches.push(current);
      current = SortedRegisterBatch::<TRegister> {
        address: register.address(),
        quantity: register.storage().quantity(),
        registers: vec![register],
      }
    }
  }
  batches.push(current);

  batches
}
