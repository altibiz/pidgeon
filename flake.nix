{
  description = "Pidgeon - Raspberry Pi message broker.";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";

    nixpkgs.url = "github:nixos/nixpkgs/release-24.11";

    perch.url = "git+file:/home/haras/repos/altibiz/perch";
    perch.inputs.nixpkgs.follows = "nixpkgs";
    perch.inputs.flake-utils.follows = "flake-utils";

    deploy-rs.url = "github:serokell/deploy-rs";
    deploy-rs.inputs.nixpkgs.follows = "nixpkgs";
    deploy-rs.inputs.utils.follows = "flake-utils";

    home-manager.url = "github:nix-community/home-manager/release-24.11";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";

    nixos-hardware.url = "github:NixOS/nixos-hardware/master";

    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";

    poetry2nix.url = "github:nix-community/poetry2nix";

    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, perch, ... } @inputs:
    perch.lib.flake.mkFlake {
      inherit inputs;
      dir = ./src/flake;
    };
}
