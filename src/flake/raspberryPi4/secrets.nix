{ self, config, sops-nix, ... }:

{
  branch.nixosModule.nixosModule = {
    imports = [
      sops-nix.nixosModules.default
    ];

    sops.defaultSopsFile = "${self}/src/flake/configurations/${config.host}/secrets.yaml";
    sops.age.keyFile = "/root/host.scrt.key";
  };
}
