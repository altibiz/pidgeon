{ modulesPath, nixos-hardware, ... }:

{
  imports = [
    # TODO: make it work with `nixos-generate`
    "${modulesPath}/installer/sd-card/sd-image-aarch64.nix"
    nixos-hardware.nixosModules.raspberry-pi-4
  ];
}

