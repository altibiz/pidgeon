{ pkgs, config, ... }:

let
  firstUser =
    builtins.head
      (builtins.attrValues
        config.users.users);
in
{
  system = {
    services.openssh.enable = true;
    services.openssh.settings.PasswordAuthentication = false;

    programs.direnv.enable = true;
    programs.direnv.nix-direnv.enable = true;

    users.mutableUsers = false;
    users.users.${firstUser.name} = {
      uid = 1000;
      gid = 100;
      hashedPasswordFile =
        config.sops.secrets."${firstUser.name}.pass.pub".path;
      extraGroups = [ "wheel" "dialout" ];
      useDefaultShell = true;
    };

    sops.secrets."${firstUser.name}.ssh.pub" = {
      path = "${firstUser.home}/.ssh/authorized_keys";
      owner = config.users.users.${firstUser.name}.name;
      group = config.users.users.${firstUser.name}.group;
    };
  };

  home = {
    home.packages = [
      pkgs.kitty
      pkgs.git
      pkgs.helix
      pkgs.yazi
      pkgs.lazygit
      pkgs.nushell
    ];
  };
}
