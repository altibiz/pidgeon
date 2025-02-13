{ writeShellApplication, nodePackages, ... }:

writeShellApplication {
  name = "cspell";
  runtimeInputs = [ nodePackages.cspell ];
  text = ''
    cspell lint "$(git rev-parse --show-toplevel)" --no-progress
  '';
}
