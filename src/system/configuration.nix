{ pkgs, config, hostname, ... }:

{
  sops.defaultSopsFile = ./assets/secrets.yaml;
  sops.age.sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
  sops.age.keyFile = "/var/lib/sops-nix/key.txt";
  sops.age.generateKey = true;

  nix.package = pkgs.nixFlakes;
  nix.extraOptions = "experimental-features = nix-command flakes";
  nixpkgs.config = import ./assets/config.nix;

  services.openssh.enable = true;
  services.openssh.settings.PasswordAuthentication = true;
  networking.hostName = hostname;

  environment.systemPackages = with pkgs; [
    libraspberrypi
    raspberrypi-eeprom
    vim-full
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
  services.postgresql.ensureDatabases = [ "mess" ];
  services.postgresql.ensureUsers = [
    {
      name = "mess";
      ensurePermissions = {
        "DATABASE mess" = "ALL PRIVILEGES";
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
    hostssl   all         all         192.168.1.0/24  scram-sha-256
  '';
  services.postgresql.enableTCPIP = true;
  services.postgresql.port = 5432;
  networking.firewall.allowedTCPPorts = [ 5432 ];
  services.postgresql.settings.ssl = "on";
  sops.secrets."server.crt".path = "/var/lib/postgresql/14/server.crt";
  sops.secrets."server.crt".owner = "postgres";
  sops.secrets."server.crt".group = "postgres";
  sops.secrets."server.crt".mode = "0600";
  sops.secrets."server.key".path = "/var/lib/postgresql/14/server.key";
  sops.secrets."server.key".owner = "postgres";
  sops.secrets."server.key".group = "postgres";
  sops.secrets."server.key".mode = "0600";
  sops.secrets."passwords.sql".path = "/var/lib/postgresql/14/passwords.sql";
  sops.secrets."passwords.sql".owner = "postgres";
  sops.secrets."passwords.sql".group = "postgres";
  sops.secrets."passwords.sql".mode = "0600";
  services.postgresql.initialScript = "/var/lib/postgresql/14/passwords.sql";

  system.stateVersion = "23.11";
}
