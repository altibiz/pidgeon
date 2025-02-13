{ writeShellApplication, nodePackages, ... }:

writeShellApplication {
  name = "prettier";
  runtimeInputs = [ nodePackages.prettier ];
  text = ''
    prettier --write "$(git rev-parse --show-toplevel)"
  '';
}
