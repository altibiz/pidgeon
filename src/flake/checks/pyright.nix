{ writeShellApplication, pyright }:

writeShellApplication {
  name = "pyright";
  runtimeInputs = [ pyright ];
  text = ''
    cd "$(git rev-parse --show-toplevel)"/src/probe
    pyright .
  '';
}
