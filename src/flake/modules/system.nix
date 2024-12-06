{ pkgs, id, ... }:

{
  nix.package = pkgs.nixVersions.stable;
  nix.extraOptions = "experimental-features = nix-command flakes";

  fileSystems."/firmware" = {
    device = "/dev/disk/by-label/FIRMWARE";
    fsType = "vfat";
  };
  fileSystems."/" = {
    device = "/dev/disk/by-label/NIXOS_SD";
    fsType = "ext4";
  };

  nixpkgs.config = { allowUnfree = true; };

  environment.systemPackages = with pkgs; [
    man-pages
    man-pages-posix
  ];

  environment.etc."id".text = id;

  environment.shells = [ "${pkgs.bashInteractive}/bin/bash" ];

  time.timeZone = "Etc/UTC";
  i18n.defaultLocale = "en_US.UTF-8";

  system.stateVersion = "24.11";
}
