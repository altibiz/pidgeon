{ pkgs, mkShell, self, ... }:

mkShell {
  packages = with pkgs; [
    # Python - first because DVC python gets first in path
    poetry
    (self.lib.poetry.mkEnvWrapper pkgs "pyright")
    (self.lib.poetry.mkEnvWrapper pkgs "yapf")
    (self.lib.poetry.mkEnv pkgs)

    # scripts
    nushell
    just

    # spelling
    nodePackages.cspell

    # misc
    nodePackages.prettier

    # shell
    shfmt
    shellcheck

    # Nix
    nixpkgs-fmt

    # Rust
    rustc
    cargo
    clippy
    rustfmt

    # build inputs
    pkg-config
    openssl
    systemd

    # tools
    zip
    unzip
  ];
}
