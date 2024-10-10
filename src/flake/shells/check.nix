{ pkgs, poetry2nix, pidgeonLib, cargo2nix, ... }:

let
  env = poetry2nix.mkPoetryEnv pidgeonLib.poetry.common;
in
pkgs.mkShell {
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  packages = with pkgs; [
    # Python - first because DVC python gets first in path
    poetry
    (pidgeonLib.poetry.mkEnvWrapper env "pyright")
    (pidgeonLib.poetry.mkEnvWrapper env "pyright-langserver")
    env

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
    cargo2nix.packages.${system}.default

    # build inputs
    pkg-config
    openssl
    systemd

    # tools
    zip
    unzip
  ];
}
