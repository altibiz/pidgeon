{ pkgs, poetry2nix, pidgeonLib, cargo2nix, ... }:

let
  env = poetry2nix.mkPoetryEnv pidgeonLib.poetry.common;
in
pkgs.mkShell {
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  RUST_BACKTRACE = "full";

  DATABASE_URL = "postgres://pidgeon:pidgeon@localhost:5433/pidgeon?sslmode=disable";

  # PIDGEON_CLOUD_DOMAIN = "localhost:5000";
  # PIDGEON_CLOUD_API_KEY = "messenger";
  # PIDGEON_CLOUD_ID = "messenger";

  PIDGEON_DB_DOMAIN = "localhost";
  PIDGEON_DB_PORT = "5433";
  PIDGEON_DB_USER = "pidgeon";
  PIDGEON_DB_PASSWORD = "pidgeon";
  PIDGEON_DB_NAME = "pidgeon";

  # PIDGEON_NETWORK_IP_RANGE_START = "192.168.1.0";
  # PIDGEON_NETWORK_IP_RANGE_END = "192.168.1.255";

  packages = with pkgs; [
    # python - first because dvc python gets first in path
    poetry
    (pidgeonLib.poetry.mkEnvWrapper env "pyright")
    (pidgeonLib.poetry.mkEnvWrapper env "pyright-langserver")
    env

    # version control
    git
    dvc-with-remotes

    # scripts
    nushell
    just

    # misc
    nodePackages.prettier
    nodePackages.yaml-language-server
    marksman
    taplo

    # spelling
    nodePackages.cspell

    # documentation
    simple-http-server
    mdbook
    mdbook-plantuml
    plantuml
    openjdk
    pandoc
    pandoc-plantuml-filter

    # shell
    bashInteractive
    nodePackages.bash-language-server
    shfmt
    shellcheck

    # nix
    nil
    nixpkgs-fmt

    # rust
    llvmPackages.clangNoLibcxx
    lldb
    rustc
    cargo
    clippy
    rustfmt
    rust-analyzer
    cargo-edit
    cargo2nix.packages.${system}.default
    evcxr

    # build inputs
    pkg-config
    openssl
    systemd

    # tools
    usql
    postgresql_14
    openssh
    age
    sqlx-cli
    jq
    sops
    zip
    unzip
    zstd
    mbpoll
    i2c-tools
    nebula
    nixos-generators
  ];
}
