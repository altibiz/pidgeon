{ self, ... }:

{
  flake.lib.lint.mkDevShell = pkgs: pkgs.mkShell {
    inputsFrom = [
      (self.lib.python.mkDevShell pkgs)
      (self.lib.rust.mkDevShell pkgs)
    ];
    packages = with pkgs; [
      nodePackages.prettier
      nodePackages.cspell
      just
      nixpkgs-fmt
      shellcheck
      markdownlint-cli
      nodePackages.markdown-link-check
      fd
    ];
  };
}
