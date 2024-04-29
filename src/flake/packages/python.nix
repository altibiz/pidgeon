{ pkgs, ... }:

pkgs.writeShellApplication {
  name = "python";
  runtimeInputs = [ pkgs.poetry ];
  text = ''
    # shellcheck disable=SC1091
    source "$(poetry env info --path)/bin/activate"
    python "$@"
  '';
}
