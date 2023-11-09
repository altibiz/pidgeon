mod discovery;
mod measurement;
mod ping;
mod push;
mod update;

pub trait Process {
  fn execute(&self);
}
