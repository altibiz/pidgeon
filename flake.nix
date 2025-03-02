{
  description = "Pidgeon - Raspberry Pi message broker.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/release-24.11";

    perch.url = "github:altibiz/perch/refs/tags/2.0.1";
    perch.inputs.nixpkgs.follows = "nixpkgs";

    deploy-rs.url = "github:serokell/deploy-rs";
    deploy-rs.inputs.nixpkgs.follows = "nixpkgs";

    nixos-hardware.url = "github:NixOS/nixos-hardware/master";

    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";

    naersk.url = "github:nix-community/naersk";

    crane.url = "github:ipetkov/crane";

    pyproject-nix.url = "github:pyproject-nix/pyproject.nix";
    pyproject-nix.inputs.nixpkgs.follows = "nixpkgs";

    uv2nix.url = "github:pyproject-nix/uv2nix";
    uv2nix.inputs.pyproject-nix.follows = "pyproject-nix";
    uv2nix.inputs.nixpkgs.follows = "nixpkgs";

    pyproject-build-systems.url = "github:pyproject-nix/build-system-pkgs";
    pyproject-build-systems.inputs.pyproject-nix.follows = "pyproject-nix";
    pyproject-build-systems.inputs.uv2nix.follows = "uv2nix";
    pyproject-build-systems.inputs.nixpkgs.follows = "nixpkgs";

    rumor.url = "github:altibiz/rumor/refs/tags/1.1.3";
    rumor.inputs.nixpkgs.follows = "nixpkgs";
    rumor.inputs.perch.follows = "perch";
  };

  outputs = { perch, ... } @inputs:
    perch.lib.flake.make {
      inherit inputs;
      root = ./.;
      prefix = "src/flake";
    };
}
