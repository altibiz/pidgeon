{ pkgs, ... }:

{
  users.defaultUserShell = "${pkgs.bashInteractive}/bin/bash";

  users.users.altibiz = {
    isNormalUser = true;
    createHome = true;
    hashedPasswordFile = "/home/altibiz/pass.pub";
    extraGroups = [ "wheel" ];
    useDefaultShell = true;
  };

  sops.secrets."altibiz.pass.pub" = {
    path = "/home/altibiz/pass.pub";
    owner = "altibiz";
    group = "users";
    mode = "0644";
  };

  sops.secrets."altibiz.ssh.pub" = {
    path = "/home/altibiz/.ssh/authorized_keys";
    owner = "altibiz";
    group = "users";
    mode = "0644";
  };
}
