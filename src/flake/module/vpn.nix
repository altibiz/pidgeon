{ lib, config, host, ... }:

{
  options = {
    pidgeon.vpn.ip = lib.mkOption {
      type = lib.types.str;
    };
  };

  system = {
    services.nebula.networks.ozds-vpn = {
      enable = true;
      cert = config.sops.secrets."${host.name}.vpn.crt.pub".path;
      key = config.sops.secrets."${host.name}.vpn.crt".path;
      ca = config.sops.secrets."shared.vpn.ca.pub".path;
      firewall.inbound = [
        {
          host = "any";
          port = "any";
          proto = "any";
        }
      ];
      firewall.outbound = [
        {
          host = "any";
          port = "any";
          proto = "any";
        }
      ];
      lighthouses = [ "10.8.0.1" ];
      staticHostMap = {
        "10.8.0.1" = [
          "ozds-vpn.altibiz.com:4242"
        ];
      };
      settings = {
        relay = {
          relays = [ "10.8.0.1" ];
        };
        punchy = {
          punch = true;
          delay = "1s";
          respond = true;
          respond_delay = "5s";
        };
        handshakes = {
          try_interval = "1s";
        };
        static_map = {
          cadence = "1m";
          lookup_timeout = "10s";
        };
        logging = {
          level = "debug";
        };
      };
    };
    sops.secrets."${host.name}.vpn.crt.pub" = {
      owner = config.systemd.services."nebula@ozds-vpn".serviceConfig.User;
      group = config.systemd.services."nebula@ozds-vpn".serviceConfig.Group;
    };
    sops.secrets."${host.name}.vpn.crt" = {
      owner = config.systemd.services."nebula@ozds-vpn".serviceConfig.User;
      group = config.systemd.services."nebula@ozds-vpn".serviceConfig.Group;
    };
    sops.secrets."shared.vpn.ca.pub" = {
      owner = config.systemd.services."nebula@ozds-vpn".serviceConfig.User;
      group = config.systemd.services."nebula@ozds-vpn".serviceConfig.Group;
    };
  };
}
