{ self
, root
, lib
, naersk
, crane
, ...
}:

let
  mkNaerskLib = pkgs: pkgs.callPackage naersk { };

  mkCraneLib = pkgs: rec {
    lib = crane.mkLib pkgs;

    src = pkgs.lib.fileset.toSource {
      inherit root;
      fileset = pkgs.lib.fileset.unions [
        (lib.fileset.commonCargoSources root)
        (pkgs.lib.path.append root "src/cli/.sqlx")
        (pkgs.lib.path.append root "src/cli/migrations")
      ];
    };

    common = {
      inherit src;
      strictDeps = true;

      nativeBuildInputs = [
        pkgs.pkg-config
      ];

      buildInputs = [
        pkgs.openssl
        pkgs.systemdLibs
      ];
    };

    artifacts = lib.buildDepsOnly common;

    individual = cargoToml: common // (rec {
      cargoArtifacts = artifacts;
      inherit (lib.crateNameFromCargoToml { inherit cargoToml; }) pname version;
      cargoExtraArgs = "-p ${pname}";
    });

    mkCrateSrc = crate: extra: pkgs.lib.fileset.toSource {
      inherit root;
      fileset = pkgs.lib.fileset.unions ([
        (pkgs.lib.path.append root "Cargo.toml")
        (pkgs.lib.path.append root "Cargo.lock")
        (lib.fileset.commonCargoSources crate)
      ] ++ extra);
    };

    mkPackage = crate: cargoToml: extra: lib.buildPackage ((individual cargoToml) // {
      cargoArtifacts = lib.buildDepsOnly common;
      src = mkCrateSrc crate extra;
    });
  };
in
{
  flake.lib.rust.mkNaerskPackage = pkgs:
    let
      naerskLib = mkNaerskLib pkgs;
    in
    naerskLib.buildPackage {
      name = "pidgeon-cli";
      pname = "pidgeon-cli";
      version = "0.1.0";
      src = self;

      nativeBuildInputs = [
        pkgs.pkg-config
      ];

      buildInputs = [
        pkgs.openssl
        pkgs.systemdLibs
      ];
    };

  flake.lib.rust.mkPackage = pkgs:
    let
      craneLib = mkCraneLib pkgs;
    in
    craneLib.mkPackage
      (lib.path.append root "src/cli")
      (lib.path.append root "src/cli/Cargo.toml")
      [
        (lib.path.append root "src/cli/.sqlx")
        (lib.path.append root "src/cli/migrations")
      ];

  flake.lib.rust.mkDevShell = pkgs:
    let
      package = self.lib.rust.mkPackage pkgs;

      postgres = self.lib.dockerCompose.mkDockerComposePostgres pkgs;

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

      buildInputs = [
        pkgs.pkg-config
        pkgs.openssl
        pkgs.systemd
      ];

      packages = with pkgs; [
        llvmPackages.clangNoLibcxx
        lldb
        rustc
        cargo
        clippy
        rustfmt
        rust-analyzer
        cargo-edit
        evcxr
        package
      ];
    };
}

