{ pkgs, config, ... }:

{
  system = {
    services.postgresql.enable = true;
    services.postgresql.package = pkgs.postgresql_16;
    services.postgresql.extensions = with config.services.postgresql.package.pkgs; [
      timescaledb
    ];
    services.postgresql.settings.shared_preload_libraries = "timescaledb";

    services.postgresql.authentication = pkgs.lib.mkOverride 10 ''
      # NOTE: do not remove local privileges because that breaks timescaledb
      # TYPE    DATABASE    USER        ADDRESS         METHOD        OPTIONS
      local     all         all                         trust
      host      all         all         samehost        trust
      hostssl   all         all         192.168.0.0/16  scram-sha-256
      hostssl   all         all         10.8.0.0/16     scram-sha-256
    '';
    services.postgresql.enableTCPIP = true;
    services.postgresql.settings.port = 5433;
    networking.firewall.allowedTCPPorts = [ 5433 ];

    services.postgresql.settings.ssl = "on";
    services.postgresql.settings.ssl_cert_file = config.sops.secrets."postgres.crt.pub".path;
    sops.secrets."postgres.crt.pub" = {
      owner = config.systemd.services.postgresql.serviceConfig.User;
      group = config.systemd.services.postgresql.serviceConfig.Group;
    };
    services.postgresql.settings.ssl_key_file = config.sops.secrets."postgres.crt".path;
    sops.secrets."postgres.crt" = {
      owner = config.systemd.services.postgresql.serviceConfig.User;
      group = config.systemd.services.postgresql.serviceConfig.Group;
    };
    services.postgresql.initialScript = config.sops.secrets."postgres.sql".path;
    sops.secrets."postgres.sql" = {
      owner = config.systemd.services.postgresql.serviceConfig.User;
      group = config.systemd.services.postgresql.serviceConfig.Group;
    };

    services.postgresql.settings = {
      checkpoint_timeout = "30min";
      checkpoint_completion_target = 0.9;
      max_wal_size = "1GB";
    };
  };
}
