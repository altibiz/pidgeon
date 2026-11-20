{
  pkgs,
  config,
  lib,
  ...
}:

{
  branch.nixosModule.nixosModule =
    let
      cfg = config.services.wifiWatchdog;
    in
    {
      options.services.wifiWatchdog = {
        enable = lib.mkEnableOption "Wi-Fi radio watchdog";

        interface = lib.mkOption {
          type = lib.types.str;
          default = "wlan0";
          description = "Wireless interface to watch";
        };

        interval = lib.mkOption {
          type = lib.types.str;
          default = "1min";
          description = "Time between checks, counted from the end of the previous check";
        };

        bootDelay = lib.mkOption {
          type = lib.types.str;
          default = "3min";
          description = "Time after boot before the first check";
        };
      };

      config = lib.mkIf cfg.enable {
        systemd.services.wifi-watchdog = {
          description = "Wi-Fi radio watchdog";
          path = with pkgs; [
            iw
            iproute2
            iputils
            networkmanager
            kmod
            procps
            bash
            gawk
            libraspberrypi
          ];
          serviceConfig = {
            Type = "oneshot";
            ExecStart = "${pkgs.nushell}/bin/nu ${./wifiWatchdog.nu} --interface ${cfg.interface}";
            StateDirectory = "wifi-watchdog";
            TimeoutStartSec = "5min";
          };
        };

        systemd.timers.wifi-watchdog = {
          description = "Periodic Wi-Fi radio watchdog check";
          wantedBy = [ "timers.target" ];
          timerConfig = {
            OnBootSec = cfg.bootDelay;
            OnUnitInactiveSec = cfg.interval;
            AccuracySec = "5s";
          };
        };
      };
    };
}
