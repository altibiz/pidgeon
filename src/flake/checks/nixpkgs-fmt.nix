{ writeShellApplication, nixpkgs-fmt, ... }:

writeShellApplication {
  name = "nixpkgs-fmt";
  runtimeInputs = [ nixpkgs-fmt ];
  text = ''
    nixpkgs-fmt --check "$(git rev-parse --show-toplevel)"
  '';
}
