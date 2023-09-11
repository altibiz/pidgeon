{ pkgs, config, hostname, username, ... }:

# TODO: .env, cli, cert renewal every year

{
  sops.defaultSopsFile = ./secrets/secrets.enc.yaml;
  # TODO: figure this out
  environment.etc."sops-nix/key.txt" = {
    text = (builtins.readFile ./secrets/pidgeon-age.key);
    mode = "0600";
  };
  sops.age.keyFile = "/etc/sops-nix/key.txt";

  nix.package = pkgs.nixFlakes;
  nix.extraOptions = "experimental-features = nix-command flakes";
  nixpkgs.config = import ./assets/config.nix;

  location.provider = "geoclue2";
  time.timeZone = "Etc/UTC";
  i18n.defaultLocale = "en_US.UTF-8";

  services.openssh.enable = true;
  # TODO: remove?
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
  sops.secrets."postgres.crt".path = "/etc/postgresql/server.crt";
  sops.secrets."postgres.crt".owner = "postgres";
  sops.secrets."postgres.crt".group = "postgres";
  sops.secrets."postgres.crt".mode = "0600";
  services.postgresql.settings.ssl_key_file = "/etc/postgresql/server.key";
  sops.secrets."postgres.key".path = "/etc/postgresql/server.key";
  sops.secrets."postgres.key".owner = "postgres";
  sops.secrets."postgres.key".group = "postgres";
  sops.secrets."postgres.key".mode = "0600";
  services.postgresql.initialScript = "/etc/postgresql/passwords.sql";
  sops.secrets."passwords.sql".path = "/etc/postgresql/passwords.sql";
  sops.secrets."passwords.sql".owner = "postgres";
  sops.secrets."passwords.sql".group = "postgres";
  sops.secrets."passwords.sql".mode = "0600";

  users.users."${username}" = {
    isNormalUser = true;
    # TODO: through sops!
    initialPassword = (builtins.readFile ./secrets/password.key);
    # TODO: remove?
    extraGroups = [ "wheel" ];
    shell = pkgs.nushell;
    openssh.authorizedKeys.keys = [
      # TODO: through sops!
      (builtins.readFile ./secrets/authorized.pub)
    ];
  };

  system.stateVersion = "23.11";
}
