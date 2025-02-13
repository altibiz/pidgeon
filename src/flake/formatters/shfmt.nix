{ writeShellApplication, shfmt }:

writeShellApplication {
  name = "shellcheck";
  runtimeInputs = [ shfmt ];
  text = ''
    shfmt --write "$(git rev-parse --show-toplevel)"
  '';
}
