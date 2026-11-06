{ self, pkgs, lib, ...} : {
  integrate.check.check = pkgs.testers.pidgeonHost {
    name = "pidgeon-host";

    nodes.pidgeon = { pkgs, ... }: {
      imports = [ self.nixosModules.pidgeonHost ];

      services.postgresql.initialScript = pkgs.writeText "init.sql" ''
          CREATE USER pidgeon PASSWORD 'pidgeon';
          CREATE DATABASE pidgeon OWNER pidgeon;
        '';
        services.postgresql.settings.ssl = lib.mkForce "off";
        services.pidgeon.envPath = pkgs.writeText "pidgeon.env" ''
          PIDGEON_DB_DOMAIN=localhost
          PIDGEON_DB_PORT=5433
          PIDGEON_DB_USER=pidgeon
          PIDGEON_DB_PASSWORD=pidgeon
          PIDGEON_DB_NAME=pidgeon
          PIDGEON_CLOUD_DOMAIN=localhost
          PIDGEON_NETWORK_IP_RANGE_START=127.0.0.1
          PIDGEON_NETWORK_IP_RANGE_END=127.0.0.1
        '';
        services.grafana.settings.security.admin_password = "test";
        environment.systemPackages = [ pkgs.curl ];
        virtualisation.memorySize = 2048;
    };

    nodes.lan = { pkgs, ... }: {
        environment.systemPackages = [ pkgs.curl ];
    };

    testScript = ''
        start_all()
        pidgeon.wait_for_unit("postgresql.service")
        pidgeon.wait_for_unit("pidgeon.service")
        pidgeon.wait_for_unit("grafana.service")
        pidgeon.wait_for_open_port(3000, "127.0.0.1")

        pidgeon.succeed("curl -sf http://127.0.0.1:3000/api/health")
        lan.fail("curl -sf --max-time 5 http://pidgeon:3000/api/health")

        psql = "psql -h localhost -p 5433 -U grafana -d pidgeon -tAc"
        pidgeon.wait_until_succeeds(f"{psql} 'select count(*) from
  system_health' | grep -qv '^0$'", timeout=300)
        pidgeon.fail(f"{psql} 'delete from system_health'")
      '';

  };
}
