{ self, config, sops-nix, ... }:

{
  imports = [{
    system = sops-nix.nixosModules.default;
    home = sops-nix.homeManagerModules.sops;
  }];

  system = {
    sops.defaultSopsFile = "${self}/src/flake/configurations/${config.host}/secrets.yaml";
    sops.age.keyFile = "/root/host.scrt.key";
  };

  home = {
    sops.defaultSopsFile = "${self}/src/flake/configurations/${config.host}/secrets.yaml";
    sops.age.keyFile = "/root/host.scrt.key";
  };
}
