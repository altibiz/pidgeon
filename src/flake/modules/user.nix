{ self, id, pkgs, ... }:

{
  users.defaultUserShell = "${pkgs.bashInteractive}/bin/bash";

  users.users.altibiz = {
    isNormalUser = true;
    createHome = true;
    hashedPasswordFile = "${self}/src/flake/pass/${id}";
    extraGroups = [ "wheel" "dialout" ];
    useDefaultShell = true;
  };

  sops.secrets."altibiz.ssh.pub" = {
    path = "/home/altibiz/.ssh/authorized_keys";
    owner = "altibiz";
    group = "users";
    mode = "0644";
  };
}
