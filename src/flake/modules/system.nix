{ pkgs, id, ... }:

{
  nix.package = pkgs.nixFlakes;
  nix.extraOptions = "experimental-features = nix-command flakes";

  nixpkgs.config = { allowUnfree = true; };

  environment.systemPackages = with pkgs; [
    libraspberrypi
    raspberrypi-eeprom
    pkg-config
    openssl

    # admin
    man-pages
    man-pages-posix
    kitty
    git
    helix
    yazi
    lazygit

    # diag
    lm_sensors # NOTE: get sensor information
    dua # NOTE: get disk space usage interactively
    duf # NOTE: disk space usage overview
    pciutils # NOTE: lspci
    lsof # NOTE: lsof -ni for ports
    dmidecode # NOTE: sudo dmidecode for mobo info
    inxi # NOTE: overall hardware info
    hwinfo # NOTE: overall hardware info
    htop # NOTE: CPU process manager
    mbpoll # NOTE: debug modbus
    i2c-tools # NOTE: debug i2c
  ];

  environment.etc."id".text = id;

  environment.sessionVariables = {
    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
  };

  environment.shells = [ "${pkgs.bashInteractive}/bin/bash" ];

  location.provider = "geoclue2";
  time.timeZone = "Etc/UTC";
  i18n.defaultLocale = "en_US.UTF-8";

  services.openssh.enable = true;
  services.openssh.settings.PasswordAuthentication = false;

  networking.hostName = "pidgeon";

  programs.direnv.enable = true;
  programs.direnv.nix-direnv.enable = true;

  services.fstrim.enable = true;

  system.stateVersion = "23.11";
}
