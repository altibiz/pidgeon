{ pkgs, config, ... }:

{
  services.postgresql.enable = true;
  services.postgresql.package = pkgs.postgresql_14;
  services.postgresql.extraPlugins = with config.services.postgresql.package.pkgs; [
    timescaledb
  ];
  services.postgresql.settings.shared_preload_libraries = "timescaledb";

  services.postgresql.authentication = pkgs.lib.mkOverride 10 ''
    # NOTE: do not remove local privileges because that breaks timescaledb
    # TYPE    DATABASE    USER        ADDRESS         METHOD        OPTIONS
    local     all         all                         trust
    host      all         all         samehost        trust
    hostssl   all         all         192.168.0.0/16  scram-sha-256
    hostssl   all         all         10.255.255.0/24 scram-sha-256
  '';
  services.postgresql.enableTCPIP = true;
  services.postgresql.settings.port = 5433;
  networking.firewall.allowedTCPPorts = [ 5433 ];

  # NITPICK: cert renewal
  services.postgresql.settings.ssl = "on";
  services.postgresql.settings.ssl_cert_file = "/etc/postgresql/server.crt";
  sops.secrets."postgres.crt.pub" = {
    path = "/etc/postgresql/server.crt";
    owner = "postgres";
    group = "postgres";
    mode = "0600";
  };
  services.postgresql.settings.ssl_key_file = "/etc/postgresql/server.key";
  sops.secrets."postgres.crt" = {
    path = "/etc/postgresql/server.key";
    owner = "postgres";
    group = "postgres";
    mode = "0600";
  };
  services.postgresql.initialScript = "/etc/postgresql/init.sql";
  sops.secrets."postgres.sql" = {
    path = "/etc/postgresql/init.sql";
    owner = "postgres";
    group = "postgres";
    mode = "0600";
  };

  services.postgresql.settings = {
    checkpoint_timeout = "30min";
    checkpoint_completion_target = 0.9;
    max_wal_size = "1GB";
  };
}
