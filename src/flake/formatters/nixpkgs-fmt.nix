{ writeShellApplication, nixpkgs-fmt, ... }:

writeShellApplication {
  name = "nixpkgs-fmt";
  runtimeInputs = [ nixpkgs-fmt ];
  text = ''
    nixpkgs-fmt "$(git rev-parse --show-toplevel)"
  '';
}
