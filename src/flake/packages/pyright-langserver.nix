{ pkgs, ... }:

pkgs.writeShellApplication {
  name = "pyright-langserver";
  runtimeInputs = [ pkgs.poetry pkgs.nodejs ];
  text = ''
    # shellcheck disable=SC1091
    source "$(poetry env info --path)/bin/activate"
    pyright-langserver "$@"
  '';
}

