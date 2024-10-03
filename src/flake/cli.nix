{ pkgs, rustPkgs, ... }:

let
  unwrapped = (rustPkgs.workspace.pidgeon-cli { });
in
pkgs.writeShellApplication
{
  name = "pidgeon";
  runtimeInputs = [ unwrapped ];
  text = ''
    ${unwrapped}/bin/pidgeon-cli "$@"
  '';
}
