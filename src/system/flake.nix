{
  description = "Raspberry Pi message broker";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-stable.url = "github:NixOS/nixpkgs/release-23.05";

    nixos-hardware.url = "github:NixOS/nixos-hardware/master";

    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";
    sops-nix.inputs.nixpkgs-stable.follows = "nixpkgs-stable";

    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, home-manager, sops-nix, ... } @ inputs:
    let
      hostname = "pidgeon";
      username = "pidgeon";
    in
    {
      nixosConfigurations.pidgeon = nixpkgs.lib.nixosSystem {
        system = "aarch64-linux";
        specialArgs = inputs // { hostname = hostname; username = username; };
        modules = [
          ./hardware-configuration.nix
          ./configuration.nix
          sops-nix.nixosModules.sops
          home-manager.nixosModules.home-manager
          {
            home-manager.useGlobalPkgs = true;
            home-manager.useUserPackages = true;
            home-manager.extraSpecialArgs = inputs // { username = username; };
            home-manager.users."${username}" = { ... }: {
              imports = [
                ./home.nix
              ];
            };
          }
        ];
      };
    };
}
