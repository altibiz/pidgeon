{ writeShellApplication, nodePackages, fd, ... }:

writeShellApplication {
  name = "prettier";
  runtimeInputs = [
    nodePackages.markdown-link-check
    fd
  ];
  text = ''
    cd "$(git rev-parse --show-toplevel)"
    fd '.*.md' -x \
      markdown-link-check \
        --config .markdown-link-check.json \
        --quiet
  '';
}
