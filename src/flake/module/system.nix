{ pkgs, config, host, lib, ... }:

let
  path = "${config.xdg.dataHome}/dot";

  ensure = ''
    if [ ! -d "${path}/.git" ]; then
      ${pkgs.git}/bin/git clone \
        -c user.name=haras \
        -c user.email=social@haras.anonaddy.me \
        https://github.com/haras-unicorn/dot \
        "${path}"
    fi
  '';

  rebuild = pkgs.writeShellApplication {
    name = "rebuild";
    runtimeInputs = [ ];
    text = ''
      ${ensure}

      sudo nixos-rebuild switch \
        --flake "${path}#${host.name}-${host.system}" \
        "$@"
    '';
  };

  rebuild-wip = pkgs.writeShellApplication {
    name = "rebuild-wip";
    runtimeInputs = [ ];
    text = ''
      ${ensure}

      cd "${path}" && ${pkgs.git}/bin/git add "${path}"
      cd "${path}" && ${pkgs.git}/bin/git commit -m WIP
      cd "${path}" && ${pkgs.git}/bin/git push

      sudo nixos-rebuild switch \
        --flake "${path}#${host.name}-${host.system}" \
        "$@"
    '';
  };

  rebuild-trace = pkgs.writeShellApplication {
    name = "rebuild-trace";
    runtimeInputs = [ ];
    text = ''
      ${ensure}

      sudo nixos-rebuild switch \
        --flake "${path}#${host.name}-${host.system}" \
        --show-trace \
        --option eval-cache false \
        "$@"
    '';
  };
in
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
  };

  home = {
    home.packages = [ rebuild rebuild-wip rebuild-trace ];

    home.activation = {
      ensurePulledAction = lib.hm.dag.entryAfter [ "writeBoundary" ] ensure;
    };
  };
}
