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

    individual = cargoToml: common // (
      let
        crate = lib.crateNameFromCargoToml { inherit cargoToml; };
      in
      {
        cargoArtifacts = artifacts;
        pname = crate.pname;
        version = crate.version;
        name = crate.pname;
        cargoExtraArgs = "-p ${crate.pname}";
      }
    );

    mkCrateSrc = crate: extra: pkgs.lib.fileset.toSource {
      inherit root;
      fileset = pkgs.lib.fileset.unions ([
        (pkgs.lib.path.append root "Cargo.toml")
        (pkgs.lib.path.append root "Cargo.lock")
        (lib.fileset.commonCargoSources crate)
      ] ++ extra);
    };

    mkPackage = crate: extra:
      lib.buildPackage ((individual (pkgs.lib.path.append crate "Cargo.toml")) // {
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
      shellHook = ''
        export RUST_BACKTRACE="full";

        export DATABASE_URL="${databaseUrl}";

        export PIDGEON_DB_DOMAIN="${postgres.host}";
        export PIDGEON_DB_PORT="${postgres.port}";
        export PIDGEON_DB_USER="${postgres.user}";
        export PIDGEON_DB_PASSWORD="${postgres.password}";
        export PIDGEON_DB_NAME="${postgres.database}";

        export PIDGEON_CLOUD_DOMAIN="localhost:5000";
        export PIDGEON_CLOUD_API_KEY="messenger";
        export PIDGEON_CLOUD_ID="messenger";

        export PIDGEON_NETWORK_IP_RANGE_START="127.0.0.1";
        export PIDGEON_NETWORK_IP_RANGE_END="127.0.0.1";
        export PIDGEON_MODBUS_PORT="5020";
      '';

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

