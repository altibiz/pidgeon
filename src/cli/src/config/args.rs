#[derive(Debug, Clone, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Values {
  /// Alternative configuration location
  #[arg(short, long)]
  pub(crate) config: Option<String>,
}

pub(crate) fn parse() -> Values {
  clap::Parser::parse()
}
