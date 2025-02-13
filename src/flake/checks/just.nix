{ writeShellApplication, just, ... }:

writeShellApplication {
  name = "just";
  runtimeInputs = [ just ];
  text = ''
    cd "$(git rev-parse --show-toplevel)"
    just --unstable --fmt --check
  '';
}
