{ pkgs, config, host, ... }:

{
  system = {
    services.openssh.enable = true;
    services.openssh.settings.PasswordAuthentication = false;

    programs.direnv.enable = true;
    programs.direnv.nix-direnv.enable = true;

    sops.secrets."${host.name}.ssh.pub" = {
      path = "/home/${host.user}/.ssh/authorized_keys";
      owner = config.users.users.${host.user}.name;
      group = config.users.users.${host.user}.group;
    };
  };

  home = {
    home.packages = [
      pkgs.kitty
      pkgs.git
      pkgs.helix
      pkgs.yazi
      pkgs.lazygit
    ];
  };
}
