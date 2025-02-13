{ writeShellApplication, cargo }:

writeShellApplication {
  name = "clippy";
  runtimeInputs = [ cargo ];
  text = ''
    cd "$(git rev-parse --show-toplevel)"
    cargo fmt --all
  '';
}
