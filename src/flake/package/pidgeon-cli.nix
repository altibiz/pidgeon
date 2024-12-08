{ self, pkgs, ... }:

self.lib.cargo.mkPackage pkgs
