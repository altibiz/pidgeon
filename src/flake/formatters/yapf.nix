{ writeShellApplication, yapf }:

writeShellApplication {
  name = "pyright";
  runtimeInputs = [ yapf ];
  text = ''
    yapf --recursive --in-place --parallel "$(git rev-parse --show-toplevel)"/src/probe
  '';
}
