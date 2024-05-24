{ pkgs, ... }:

pkgs.mkShell {
  DATABASE_URL = "postgres://pidgeon:pidgeon@localhost:5433/pidgeon?sslmode=disable";
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  # PIDGEON_CLOUD_SSL = "1";
  # PIDGEON_CLOUD_DOMAIN = "localhost:5001";
  # PIDGEON_CLOUD_API_KEY = "pidgeon";
  # PIDGEON_CLOUD_ID = "pidgeon";

  PIDGEON_DB_DOMAIN = "localhost";
  PIDGEON_DB_PORT = "5433";
  PIDGEON_DB_USER = "pidgeon";
  PIDGEON_DB_PASSWORD = "pidgeon";
  PIDGEON_DB_NAME = "pidgeon";

  # PIDGEON_NETWORK_IP_RANGE_START = "192.168.1.0";
  # PIDGEON_NETWORK_IP_RANGE_END = "192.168.1.255";

  packages = with pkgs; [
    # Nix
    nil
    nixpkgs-fmt

    # Python
    poetry
    python
    pyright
    pyright-langserver
    yapf
    ruff

    # Rust
    llvmPackages.clangNoLibcxx
    lldb
    rustc
    cargo
    clippy
    rustfmt
    rust-analyzer
    cargo-edit

    # Shell
    bashInteractive
    nodePackages.bash-language-server
    shfmt
    shellcheck

    # Spelling
    nodePackages.cspell

    # Documentation
    simple-http-server

    # Misc
    nodePackages.prettier
    nodePackages.yaml-language-server
    marksman
    taplo

    # Tools
    nushell
    usql
    just
    openssh
    age
    pkg-config
    openssl
    sqlx-cli
    jq
    sops
  ];
}
