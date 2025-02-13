{ self, pkgs, ... }:

self.lib.cargo.mkPackage
  pkgs
  ../../../src/cli
  "pidgeon-cli"
  [
    ../../../src/cli/.sqlx
    ../../../src/cli/migrations
  ]
