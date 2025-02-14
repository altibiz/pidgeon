{ self, config, sops-nix, ... }:

{
  system = {
    imports = [
      sops-nix.nixosModules.default
    ];

    sops.defaultSopsFile = "${self}/src/flake/configurations/${config.host}/secrets.yaml";
    sops.age.keyFile = "/root/host.scrt.key";
  };

  home = {
    imports = [
      sops-nix.homeManagerModules.sops
    ];

    sops.defaultSopsFile = "${self}/src/flake/configurations/${config.host}/secrets.yaml";
    sops.age.keyFile = "/root/host.scrt.key";
  };
}
