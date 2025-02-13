{ writeShellApplication, markdownlint-cli, ... }:

writeShellApplication {
  name = "prettier";
  runtimeInputs = [ markdownlint-cli ];
  text = ''
    markdownlint "$(git rev-parse --show-toplevel)"
  '';
}
