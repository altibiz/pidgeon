{ pkgs, ... }:

{
  shared = {
    nix.extraOptions = "experimental-features = nix-command flakes";
    nix.gc.automatic = true;
    nix.gc.options = "--delete-older-than 30d";
    nix.settings.auto-optimise-store = true;
    nix.settings.trusted-users = [
      "@wheel"
    ];
    nixpkgs.config = {
      allowUnfree = true;
    };
  };

  system = {
    nix.package = pkgs.nixVersions.stable;

    fileSystems."/firmware" = {
      device = "/dev/disk/by-label/FIRMWARE";
      fsType = "vfat";
    };
    fileSystems."/" = {
      device = "/dev/disk/by-label/NIXOS_SD";
      fsType = "ext4";
    };

    environment.systemPackages = with pkgs; [
      man-pages
      man-pages-posix
    ];

    environment.shells = [ "${pkgs.bashInteractive}/bin/bash" ];

    time.timeZone = "Etc/UTC";
    i18n.defaultLocale = "en_US.UTF-8";

    system.stateVersion = "24.11";
  };

  home = {
    home.stateVersion = "24.11";
  };
}
