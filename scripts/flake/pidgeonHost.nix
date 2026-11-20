{ self
, config
, pkgs
, lib
, ...
}:
{
  branch.nixosModule.nixosModule = {
    imports = [
      self.nixosModules.pidgeonCli
    ];

    services.pidgeon.enable = true;
    services.pidgeon.debug = true;
    services.pidgeon.configPath = "/etc/pidgeon/config.toml";

    environment.etc."pidgeon/config.toml" = {
      source = "${self}/assets/pidgeon/config.toml";
      user = config.systemd.services.pidgeon.serviceConfig.User;
      group = config.systemd.services.pidgeon.serviceConfig.Group;
    };

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
    services.postgresql.settings.ssl = "on";

    services.postgresql.settings = {
      checkpoint_timeout = "30min";
      checkpoint_completion_target = 0.9;
      max_wal_size = "1GB";
    };

    # TODO: implement this later for limiting queries $PSQL -d pidgeon -tAc "ALTER ROLE grafana SET statement_timeout = '30s'"
    # but it probably needs fine tuning for some of the queries
    services.postgresql.ensureUsers = [{ name = "grafana"; }];
    systemd.services.postgresql.postStart = lib.mkAfter ''
      $PSQL -d pidgeon -tAc 'GRANT USAGE ON SCHEMA public TO grafana'
      $PSQL -d pidgeon -tAc 'GRANT SELECT ON ALL TABLES IN SCHEMA public TO grafana'
      $PSQL -d pidgeon -tAc 'ALTER DEFAULT PRIVILEGES FOR ROLE pidgeon IN SCHEMA public GRANT SELECT ON TABLES TO grafana'
    '';

    networking.firewall.enable = true;

    services.grafana.enable = true;
    services.grafana.settings = {
      server = {
        http_addr = "127.0.0.1";
        http_port = 3000;
      };
      security = {
        disable_gravatar = true;
      };
      analytics = {
        reporting_enabled = false;
        check_for_updates = false;
        check_for_plugin_updates = false;
        feedback_links_enabled = false;
      };
      users = {
        allow_sign_up = false;
        allow_org_create = false;
        auto_assign_org_role = "Viewer";
        viewers_can_edit = false;
      };
      snapshots.external_enabled = false;
      public_dashboards.enabled = false;
      plugins.plugin_admin_enabled = false;
      news.news_feed_enabled = false;
      date_formats.default_timezone = "utc";
    };

    services.grafana.provision.enable = true;

    services.grafana.provision.dashboards.settings = {
      apiVersion = 1;
      providers = [
        {
          name = "pidgeon";
          folder = "Pidgeon";
          type = "file";
          disableDeletion = true;
          allowUiUpdates = false; # NOTE: the updates are done manually with whole system update
          options.path = self.lib.python.mkGrafanaDashboards pkgs;
        }
      ];
    };

    services.grafana.provision.datasources.settings = {
      apiVersion = 1;
      datasources = [
        {
          name = "Pidgeon DB";
          uid = "pidgeon-db";
          type = "postgres";
          url = "localhost:5433";
          user = "grafana";
          isDefault = true;
          jsonData = {
            database = "pidgeon";
            sslmode = "disable";
            postgresVersion = 1500;
            timescaledb = true;
          };
        }
      ];
    };
  };
}
