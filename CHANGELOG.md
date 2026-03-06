<!-- markdownlint-disable MD024 -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- packages from this flake to nixos configurations
- s3 nix binary cache substituters to this flake and nixos configurations
- function to make a new developer vpn config
- functions to connect to s3 nix binary cache bucket
- reading and serving time related registers to probe
- push process throttling
- update/install commands for raspberry pis

### Changed

- probe package fix
- actually run time process to change meter clock time
- use release branch for doc generation
- ping and discovery default timeout
- wifi share fix
- concentrator 4 routing

### Removed

- leftover plaintext license

## [1.0.0] - 2025-03-5

### Added

- init

[1.0.0]: https://github.com/altibiz/pidgeon/releases/tag/1.0.0
