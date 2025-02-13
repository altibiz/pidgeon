{ self, pkgs, ... }:

self.lib.poetry.mkApp pkgs
