{ pkgs, self, ... }:

let
  postgres = self.lib.docker.mkDockerComposePostgres pkgs;

  databaseUrl =
    let
      auth = "${postgres.user}:${postgres.password}";
      conn = "${postgres.host}:${postgres.port}";
      db = postgres.database;
    in
    "postgres://${auth}@${conn}/${db}?sslmode=disable";
in
pkgs.mkShell {
  RUST_BACKTRACE = "full";

  DATABASE_URL = databaseUrl;

  PIDGEON_DB_DOMAIN = postgres.host;
  PIDGEON_DB_PORT = postgres.port;
  PIDGEON_DB_USER = postgres.user;
  PIDGEON_DB_PASSWORD = postgres.password;
  PIDGEON_DB_NAME = postgres.database;

  PIDGEON_CLOUD_DOMAIN = "localhost:5000";
  PIDGEON_CLOUD_API_KEY = "messenger";
  PIDGEON_CLOUD_ID = "messenger";

  PIDGEON_NETWORK_IP_RANGE_START = "127.0.0.1";
  PIDGEON_NETWORK_IP_RANGE_END = "127.0.0.1";
  PIDGEON_MODBUS_PORT = "5020";

  packages = with pkgs; [
    # python - first because dvc python gets first in path
    poetry
    (self.lib.poetry.mkEnvWrapper pkgs "pyright")
    (self.lib.poetry.mkEnvWrapper pkgs "pyright-langserver")
    (self.lib.poetry.mkEnvWrapper pkgs "yapf")
    (self.lib.poetry.mkEnvWrapper pkgs "python")
    (self.lib.poetry.mkEnv pkgs)

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
    evcxr

    # build inputs
    pkg-config
    openssl
    systemd

    # tools
    (pkgs.writeShellApplication {
      name = "usql";
      runtimeInputs = [ pkgs.usql ];
      text = ''
        usql ${databaseUrl} "$@"
      '';
    })
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
    gum
    deploy-rs
  ] ++ pkgs.lib.optionals
    (
      pkgs.stdenv.hostPlatform.isLinux
        && pkgs.stdenv.hostPlatform.isx86_64
    ) [
    libguestfs-with-appliance
  ];
}
