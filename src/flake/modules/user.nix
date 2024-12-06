{ pkgs, config, ... }:

{
  services.openssh.enable = true;
  services.openssh.settings.PasswordAuthentication = false;

  programs.direnv.enable = true;
  programs.direnv.nix-direnv.enable = true;

  environment.systemPackages = [
    pkgs.kitty
    pkgs.git
    pkgs.helix
    pkgs.yazi
    pkgs.lazygit
  ];

  users.defaultUserShell = "${pkgs.bashInteractive}/bin/bash";

  sops.secrets."altibiz.pass.pub".neededForUsers = true;
  users.users.altibiz = {
    isNormalUser = true;
    createHome = true;
    hashedPasswordFile = config.sops.secrets."altibiz.pass.pub".path;
    extraGroups = [ "wheel" "dialout" ];
    useDefaultShell = true;
  };

  sops.secrets."altibiz.ssh.pub" = {
    path = "/home/altibiz/.ssh/authorized_keys";
    owner = config.users.users.altibiz.name;
    group = config.users.users.altibiz.group;
  };
}
