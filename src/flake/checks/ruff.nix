{ writeShellApplication, ruff }:

writeShellApplication {
  name = "ruff";
  runtimeInputs = [ ruff ];
  text = ''
    ruff check "$(git rev-parse --show-toplevel)"/src/probe
  '';
}
