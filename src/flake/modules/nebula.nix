{ ... }:

{
  # NOTE: these values are not used but nix evaluates them for some reason
  services.nebula.networks.ozds-vpn = {
    enable = true;
    cert = "/etc/nebula/host.crt";
    key = "/etc/nebula/host.key";
    ca = "/etc/nebula/ca.crt";
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
      "ozds-vpn.altibiz.com" = [
        "10.8.0.1:4242"
      ];
    };
    settings = {
      handshakes = {
        try_interval = "1s";
      };
      static_map = {
        cadence = "5m";
        lookup_timeout = "10s";
      };
    };
  };
  sops.secrets."nebula.ca.pub" = {
    path = "/etc/nebula/ca.crt";
    owner = "nebula-ozds-vpn";
    group = "nebula-ozds-vpn";
    mode = "0644";
  };
  sops.secrets."nebula.crt.pub" = {
    path = "/etc/nebula/host.crt";
    owner = "nebula-ozds-vpn";
    group = "nebula-ozds-vpn";
    mode = "0644";
  };
  sops.secrets."nebula.crt" = {
    path = "/etc/nebula/host.key";
    owner = "nebula-ozds-vpn";
    group = "nebula-ozds-vpn";
    mode = "0400";
  };
}
