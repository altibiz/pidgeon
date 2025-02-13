{ writeShellApplication, cargo, clippy }:

writeShellApplication {
  name = "clippy";
  runtimeInputs = [ cargo clippy ];
  text = ''
    cd "$(git rev-parse --show-toplevel)"
    cargo clippy -- -D warnings
  '';
}
