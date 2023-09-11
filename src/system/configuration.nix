{ pkgs, config, hostname, ... }:

{
  sops.defaultSopsFile = ./assets/secrets.yaml;
  sops.age.sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
  sops.age.keyFile = "/var/lib/sops-nix/key.txt";
  sops.age.generateKey = true;

  environment.etc."test" = (import ./secrets.nix).password;

  nix.package = pkgs.nixFlakes;
  nix.extraOptions = "experimental-features = nix-command flakes";
  nixpkgs.config = import ./assets/config.nix;

  location.provider = "geoclue2";
  time.timeZone = "Etc/UTC";
  i18n.defaultLocale = "en_US.UTF-8";

  services.openssh.enable = true;
  services.openssh.settings.PasswordAuthentication = true;
  networking.hostName = hostname;

  environment.systemPackages = with pkgs; [
    libraspberrypi
    raspberrypi-eeprom
    helix
    git
    man-pages
    man-pages-posix
    openssl
    pkg-config
    age
    ssh-to-age
    sops
  ];
  environment.sessionVariables = {
    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
  };

  services.postgresql.enable = true;
  services.postgresql.package = pkgs.postgresql_14;
  services.postgresql.extraPlugins = with config.services.postgresql.package.pkgs; [
    timescaledb
  ];
  services.postgresql.settings.shared_preload_libraries = "timescaledb";
  services.postgresql.ensureDatabases = [ "pidgeon" ];
  services.postgresql.ensureUsers = [
    {
      name = "pidgeon";
      ensurePermissions = {
        "DATABASE pidgeon" = "ALL PRIVILEGES";
      };
      ensureClauses = {
        login = true;
      };
    }
  ];
  services.postgresql.authentication = pkgs.lib.mkOverride 10 ''
    # NOTE: do not remove local privileges because that breaks timescaledb
    # TYPE    DATABASE    USER        ADDRESS         METHOD        OPTIONS
    local     all         all                         trust
    host      all         all         samehost        trust
    hostssl   all         all         192.168.0.0/16  scram-sha-256
  '';
  services.postgresql.enableTCPIP = true;
  services.postgresql.port = 5432;
  networking.firewall.allowedTCPPorts = [ 5432 ];
  services.postgresql.settings.ssl = "on";
  services.postgresql.settings.ssl_cert_file = "/etc/postgresql/server.crt";
  sops.secrets."server.crt".path = "/etc/postgresql/server.crt";
  sops.secrets."server.crt".owner = "postgres";
  sops.secrets."server.crt".group = "postgres";
  sops.secrets."server.crt".mode = "0600";
  services.postgresql.settings.ssl_key_file = "/etc/postgresql/server.key";
  sops.secrets."server.key".path = "/etc/postgresql/server.key";
  sops.secrets."server.key".owner = "postgres";
  sops.secrets."server.key".group = "postgres";
  sops.secrets."server.key".mode = "0600";
  services.postgresql.initialScript = "/etc/postgresql/init.sql";
  sops.secrets."init.sql".path = "/etc/postgresql/init.sql";
  sops.secrets."init.sql".owner = "postgres";
  sops.secrets."init.sql".group = "postgres";
  sops.secrets."init.sql".mode = "0600";

  system.stateVersion = "23.11";
}
