{ ... }:

{
  system = {
    # NOTE: these values are not used but nix evaluates them for some reason
    services.nebula.networks.nebula = {
      enable = true;
      cert = "/etc/nebula/host.crt";
      key = "/etc/nebula/host.key";
      ca = "/etc/nebula/ca.crt";
      firewall.inbound = [
        {
          host = "all";
          port = "all";
          proto = "any";
        }
      ];
      firewall.outbound = [
        {
          host = "all";
          port = "all";
          proto = "any";
        }
      ];
      lighthouses = [ "10.8.0.1" ];
      staticHostMap = {
        "ozds-vpn.altibiz.com" = [
          "10.8.0.1:4242"
        ];
      };
      settings = {
        static_map = {
          cadence = "5m";
          lookup_timeout = "10s";
        };
      };
    };
    sops.secrets."vpn.ca" = {
      path = "/etc/nebula/ca.crt";
      owner = "nebula-nebula";
      group = "nebula-nebula";
      mode = "0644";
    };
    sops.secrets."vpn.crt.pub" = {
      path = "/etc/nebula/host.crt";
      owner = "nebula-nebula";
      group = "nebula-nebula";
      mode = "0644";
    };
    sops.secrets."vpn.crt" = {
      path = "/etc/nebula/host.key";
      owner = "nebula-nebula";
      group = "nebula-nebula";
      mode = "0400";
    };
  };
}
