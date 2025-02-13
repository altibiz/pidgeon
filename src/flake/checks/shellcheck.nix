{ writeShellApplication, nodePackages }:

writeShellApplication {
  name = "shellcheck";
  runtimeInputs = [ nodePackages.cspell ];
  text = ''
    for script in "$(git rev-parse --show-toplevel)"/scripts/*.sh; do
      shellcheck "$script"
    done
  '';
}
