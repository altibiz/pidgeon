{ self, perch, lib, specialArgs, sops-nix, ... }:

let
  pidgeons = builtins.fromJSON ./pidgeons.json;
in
{
  seal.overlays.raspberryPi4 =
    (final: prev: {
      # NOTE: https://github.com/NixOS/nixpkgs/issues/154163#issuecomment-1008362877  
      makeModulesClosure = x: prev.makeModulesClosure
        (x // { allowMissing = true; });
    });

  seal.deploy.nodes =
    builtins.listToAttrs
      (builtins.map
        (pidgeon: {
          name = "pidgeon-${pidgeon.id}";
          value = {
            hostname = pidgeon.ip;
            sshUser = "altibiz";
          };
        })
        pidgeons);

  flake.nixosModules = rec {
    default = pidgeon;
    pidgeon = import ./pidgeon.nix;
  };

  flake.nixosConfigurations =
    builtins.listToAttrs
      builtins.map
      (pidgeon:
        {
          name = "pidgeon-${pidgeon.id}";
          value = lib.nixosSystem {
            system = "aarch64-linux";
            inherit specialArgs;
            modules = [
              {
                nixpkgs.overlays = [
                  self.overlays.raspberryPi4
                ];
              }
              (perch.lib.import.importDirToFlatPathList ./.)
              sops-nix.nixosModules.default
              {
                sops.defaultSopsFile = ./secrets + "${pidgeon.id}.yaml";
                sops.age.keyFile = "/root/host.scrt.key";
              }
            ];
          };
        })
      pidgeons;
}
