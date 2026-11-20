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
- system health db table with weekly compression and monthly retention
- system health process that stores raspberry pi health every minute
- configurable system health schedule and sensor paths
- grafana with meters, overview and system health dashboards
- grafana dashboard generator and local grafana in docker compose
- grafana viewer user on raspberry pis
- nixos test for the pidgeon host stack and its dashboards
- local pidgeon vm configuration
- command to seed wifi configuration for a new pidgeon
- wifi watchdog that reloads the driver and reboots on a wedged radio
- per pidgeon uplink setting to turn the wifi watchdog off on ethernet
- hardware watchdog and forced reboot timeout on raspberry pis

### Changed

- probe package fix
- actually run time process to change meter clock time
- use release branch for doc generation
- ping and discovery default timeout
- wifi share fix
- concentrator 4 routing
- better raspberry pi arguments
- shared pidgeon host module for raspberry pis, vm and tests
- postgres and grafana only reachable over the vpn
- renamed temperature_monitor hardware setting to temperature_monitor_path

### Removed

- leftover plaintext license

## [1.0.0] - 2025-03-5

### Added

- init

[1.0.0]: https://github.com/altibiz/pidgeon/releases/tag/1.0.0
