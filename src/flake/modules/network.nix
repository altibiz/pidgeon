{ ... }:

{
  networking.firewall.enable = true;
  networking.networkmanager.enable = true;
  networking.nameservers = [ "1.1.1.1" "1.0.0.1" ];

  networking.networkmanager.wifi.powersave = false;
  networking.networkmanager.ensureProfiles.profiles = {
    "wifi" = {
      connection = {
        id = "wifi";
        permissions = "";
        type = "wifi";
        interface-name = "wlan0";
      };
      ipv4 = {
        dns-search = "";
        method = "auto";
      };
      ipv6 = {
        addr-gen-mode = "stable-privacy";
        dns-search = "";
        method = "auto";
      };
      wifi = {
        mac-address-blacklist = "";
        mode = "infrastructure";
        ssid = "$WIFI_SSID";
      };
      wifi-security = {
        auth-alg = "open";
        key-mgmt = "wpa-psk";
        psk = "$WIFI_PASS";
      };
    };
  };

  networking.networkmanager.ensureProfiles.environmentFiles = [
    "/etc/NetworkManager/env/wifi.env"
  ];
  sops.secrets."wifi.env" = {
    path = "/etc/NetworkManager/env/wifi.env";
    owner = "root";
    group = "root";
    mode = "0600";
  };
}
