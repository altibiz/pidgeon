{ self, pkgs, sops-nix, id, ... }:

{
  imports = [
    sops-nix.nixosModules.sops
  ];

  environment.systemPackages = with pkgs; [
    age
    ssh-to-age
    sops
  ];

  sops.defaultSopsFile = "${self}/src/flake/enc/${id}";
  sops.age.keyFile = "/root/.sops/secrets.age";
}
