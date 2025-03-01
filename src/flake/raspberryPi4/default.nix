{ self, perch, lib, specialArgs, sops-nix, ... }:

let
  pidgeons = [
    {
      id = "grFvxoW5xQs69gfQfCSTIND8NT1L70tI";
      ip = "10.8.0.12";
    }
    {
      id = "TulPr4fOqSE82Ro6jyRSlEv92NFiN3fo";
      ip = "10.8.0.11";
    }
    {
      id = "PayZMy3Y0JIHJzwfwMRiH45NkXjAH7GT";
      ip = "10.8.0.15";
    }
    {
      id = "6k73Uam8eNOg0KXhVdXj3p1C7N5bMpqf";
      ip = "10.8.0.10";
    }
    {
      id = "EvztBTwG5Zryo7UJPZFjtYwMVuEqQyZ4";
      ip = "10.8.0.16";
    }
    {
      id = "K2iADfcQngJoZvssNxiZDvbErFq0w3jn";
      ip = "10.8.0.17";
    }
    {
      id = "HHeJNW843H35uc0gdh6sbJUftnLNklyj";
      ip = "10.8.0.19";
    }
    {
      id = "nEIcKYdZ8KG5Cm3qXTb0xhcSooccc6td";
      ip = "10.8.0.14";
    }
    {
      id = "8RHOU6R8p732mNDytEL9minBqdo2ax77";
      ip = "10.8.0.13";
    }
    {
      id = "n5t4DIM4zK2wB89NFKlfUJd2yAfoilFi";
      ip = "10.8.0.20";
    }
    {
      id = "shdJz2n5imgs87CefXClpi5DDpjbMKsT";
      ip = "10.8.0.18";
    }
  ];
in
{
  seal.overlays.raspberryPi4 =
    (final: prev: {
      # NOTE: https://github.com/NixOS/nixpkgs/issues/154163#issuecomment-1008362877  
      makeModulesClosure = x: prev.makeModulesClosure
        (x // { allowMissing = true; });
    });

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
              (perch.lib.import.importDirToFlatPathList ./.)
              sops-nix.nixosModules.default
              {
                sops.defaultSopsFile = "${self}/src/flake/secrets/${pidgeon.id}.yaml";
                sops.age.keyFile = "/root/host.scrt.key";
              }
            ];
          };
        })
      pidgeons;
}
