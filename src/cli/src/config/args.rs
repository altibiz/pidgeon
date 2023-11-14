#[derive(Debug, Clone, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Values {
  /// Run in development mode
  #[arg(short, long)]
  pub(crate) dev: bool,

  /// Alternative configuration location
  #[arg(short, long)]
  pub(crate) config: Option<String>,
}

pub(crate) fn parse() -> Values {
  clap::Parser::parse()
}
