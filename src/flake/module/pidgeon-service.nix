{ self, pkgs, lib, config, ... }:

let
  cfg = config.services.pidgeon;

  package = self.packages.${pkgs.system}.default;

  service = pkgs.writeShellApplication {
    name = "pidgeon-service";
    runtimeInputs = [ package ];
    text = ''
      echo "Starting: $PIDGEON_CLOUD_ID"
      pidgeon-cli --config '${cfg.configPath}'
    '';
  };
in
{
  options.services.pidgeon = {
    enable = lib.mkEnableOption "pidgeon";

    configPath = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = "Path to config. This config will be overwritten in memory"
        + " when pidgeon successfully polls the server";
    };

    envPath = lib.mkOption {
      type = lib.types.str;
      description = "Path to environment variables file."
        + " This file will be sourced before pidgeon is ran.";
    };
  };

  config = {
    system = {
      environment.systemPackages = [
        pkgs.openssl
      ];

      environment.sessionVariables = {
        PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
      };

      users.groups.pidgeon = { };

      users.users.pidgeon = {
        isSystemUser = true;
        description = "Pidgeon service user";
        group = "pidgeon";
        extraGroups = [ "dialout" ];
      };

      systemd.services.pidgeon = {
        description = "Pidgeon - Raspberry Pi message broker.";
        after = [ "network.target" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          EnvironmentFile = cfg.envPath;
          ExecStart = "${service}/bin/pidgeon-service";
          Restart = "always";
          User = "pidgeon";
          Group = "pidgeon";
        };
      };
    };
  };
}
