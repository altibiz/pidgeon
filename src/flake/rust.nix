{ self
, naersk ? null
, crane ? null
, ...
}:

let
  mkNaerskLib = pkgs: pkgs.callPackage naersk { };

  mkCraneLib = pkgs: rec {
    lib = crane.mkLib pkgs;

    src = pkgs.lib.fileset.toSource {
      root = ../../..;
      fileset = pkgs.lib.fileset.unions [
        ((mkCraneLib pkgs).fileset.commonCargoSources ../../..)
        ../../../src/cli/.sqlx
        ../../../src/cli/migrations
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
        pkgs.systemd
      ];
    };

    artifacts = (mkCraneLib pkgs).buildDepsOnly common;

    individual = common // {
      cargoArtifacts = artifacts;
      inherit ((mkCraneLib pkgs).crateNameFromCargoToml { inherit src; }) version;
    };

    mkCrateSrc = crate: extra: pkgs.lib.fileset.toSource {
      root = ../../..;
      fileset = pkgs.lib.fileset.unions ([
        ../../../Cargo.toml
        ../../../Cargo.lock
        (lib.fileset.commonCargoSources crate)
      ] ++ extra);
    };

    mkPackage = crate: name: extra: lib.buildPackage (individual // {
      pname = name;
      cargoExtraArgs = "-p ${name}";
      cargoArtifacts = lib.buildDepsOnly common;
      src = mkCrateSrc crate extra;
    });
  };
in
{
  # flake.lib.rust.mkPackage = pkgs:
  #   let
  #     naerskLib = mkNaerskLib pkgs;
  #   in
  #   naerskLib.buildPackage {
  #     name = "pidgeon-cli";
  #     pname = "pidgeon-cli";
  #     version = "0.1.0";
  #     src = self;
  #     buildInputs = with pkgs; [
  #       pkg-config
  #       openssl
  #       systemd
  #     ];
  #   };

  flake.lib.rust.mkPackage = pkgs:
    }

