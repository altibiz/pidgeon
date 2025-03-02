{ pkgs, config, ... }:

{
  users.mutableUsers = false;
  users.groups.altibiz = { };
  users.users.altibiz = {
    group = "altibiz";
    isNormalUser = true;
    hashedPasswordFile =
      config.sops.secrets."altibiz.pass.pub".path;
    extraGroups = [ "wheel" "dialout" ];
    packages = [
      pkgs.kitty
      pkgs.git
      pkgs.helix
      pkgs.yazi
      pkgs.lazygit
      pkgs.nushell
    ];
  };
  sops.secrets."altibiz.pass.pub".neededForUsers = true;

  services.openssh.enable = true;
  services.openssh.settings.PasswordAuthentication = false;

  sops.secrets."altibiz.ssh.pub" = {
    path = "${config.users.user.altibiz.home}/.ssh/authorized_keys";
    owner = config.users.users.altibiz.name;
    group = config.users.users.altibiz.group;
  };
}
