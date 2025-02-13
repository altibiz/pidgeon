{ writeShellApplication, nodePackages, ... }:

writeShellApplication {
  name = "prettier";
  runtimeInputs = [ nodePackages.prettier ];
  text = ''
    prettier --check "$(git rev-parse --show-toplevel)"
  '';
}
