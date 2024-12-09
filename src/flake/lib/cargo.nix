{ nixpkgs, crane, ... }:

# NOTE: just ignore the pname/version warnings

let
  mkCraneLib = pkgs: crane.mkLib pkgs;

  mkSrc = pkgs: nixpkgs.lib.fileset.toSource {
    root = ../../..;
    fileset = nixpkgs.lib.fileset.unions [
      ((mkCraneLib pkgs).fileset.commonCargoSources ../../..)
      ../../../src/cli/.sqlx
      ../../../src/cli/migrations
    ];
  };

  mkCommon = pkgs: {
    src = mkSrc pkgs;
    strictDeps = true;

    nativeBuildInputs = [
      pkgs.pkg-config
    ];
    buildInputs = [
      pkgs.openssl
      pkgs.systemd
    ];
  };

  mkCargoArtifacts = pkgs: (mkCraneLib pkgs).buildDepsOnly (mkCommon pkgs);

  mkIndividual = pkgs: (mkCommon pkgs) // {
    cargoArtifacts = mkCargoArtifacts pkgs;
    inherit ((mkCraneLib pkgs).crateNameFromCargoToml { src = mkSrc pkgs; }) version;
  };

  mkCrateSrc = pkgs: crate: extra: nixpkgs.lib.fileset.toSource {
    root = ../../..;
    fileset = nixpkgs.lib.fileset.unions ([
      ../../../Cargo.toml
      ../../../Cargo.lock
      ((mkCraneLib pkgs).fileset.commonCargoSources crate)
    ] ++ extra);
  };

  mkPackage = pkgs: crate: name: extra: (mkCraneLib pkgs).buildPackage ((mkIndividual pkgs) // {
    pname = name;
    cargoExtraArgs = "-p ${name}";
    cargoArtifacts = (mkCraneLib pkgs).buildDepsOnly (mkCommon pkgs);
    src = mkCrateSrc pkgs crate extra;
  });
in
{
  inherit mkPackage;
}
