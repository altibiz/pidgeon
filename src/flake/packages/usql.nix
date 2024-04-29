{ pkgs, ... }:

pkgs.writeShellApplication {
  name = "usql";
  runtimeInputs = [ pkgs.usql ];
  text = ''
    usql pg://pidgeon:pidgeon@localhost:5433/pidgeon?sslmode=disable "$@"
  '';
}

