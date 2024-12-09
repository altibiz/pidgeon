{ nixpkgs, crane, ... }:

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
    pname = "pidgeon";
    version = "0.1.0";

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

  mkPackage = pkgs: (mkCraneLib pkgs).buildPackage ((mkCommon pkgs) // {
    cargoArtifacts = (mkCraneLib pkgs).buildDepsOnly (mkCommon pkgs);
  });
in
{
  inherit mkPackage;
}
