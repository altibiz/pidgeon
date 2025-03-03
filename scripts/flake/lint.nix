{
  flake.lib.lint.mkDevShell = pkgs: pkgs.mkShell {
    packages = with pkgs; [
      nodePackages.prettier
      nodePackages.cspell
      just
      nixpkgs-fmt
      cargo
      clippy
      shellcheck
      ruff
      pyright
    ];
  };
}
