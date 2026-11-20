{ self
, pkgs
, specialArgs
, ...
}:
{
  integrate.check.check = pkgs.testers.runNixOSTest {
    name = "pidgeon-host";

    node.specialArgs = specialArgs;
    node.pkgsReadOnly = false;

    nodes.pidgeon = { pkgs, lib, ... }: {
      imports = [ self.nixosModules.pidgeonHost ];

      nixpkgs.config.allowUnfreePredicate = pkg: builtins.elem (lib.getName pkg) [ "timescaledb" ];

      services.postgresql.initialScript = pkgs.writeText "init.sql" ''
        CREATE USER pidgeon PASSWORD 'pidgeon';
        CREATE DATABASE pidgeon OWNER pidgeon;
      '';
      services.postgresql.settings.ssl = lib.mkForce "off";
      services.pidgeon.envPath = "${pkgs.writeText "pidgeon.env" ''
        PIDGEON_DB_DOMAIN=localhost
        PIDGEON_DB_PORT=5433
        PIDGEON_DB_USER=pidgeon
        PIDGEON_DB_PASSWORD=pidgeon
        PIDGEON_DB_NAME=pidgeon
        PIDGEON_CLOUD_DOMAIN=localhost
        PIDGEON_CLOUD_ID=pidgeon-test
        PIDGEON_NETWORK_IP_RANGE_START=127.0.0.1
        PIDGEON_NETWORK_IP_RANGE_END=127.0.0.1
      ''}";
      services.grafana.settings.security.admin_password =
        "$__file{${pkgs.writeText "grafana-admin-pass" "test"}}";
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
      pidgeon.wait_until_succeeds(f"{psql} 'select count(*) from system_health' | grep -qv '^0$'", timeout=300)
      pidgeon.fail(f"{psql} 'delete from system_health'")

      # NOTE: every provisioned dashboard loads and all of its queries run
      # against the real schema, so a migration that breaks a panel fails here

      import glob
      import json
      import shlex

      def dashboard_queries(dashboard):
          panels = list(dashboard["panels"])
          while panels:
              panel = panels.pop()
              panels.extend(panel.get("panels", []))
              for target in panel.get("targets", []):
                  yield target["rawSql"], target["format"]
          for annotation in dashboard["annotations"]["list"]:
              if "target" in annotation:
                  yield annotation["target"]["rawSql"], annotation["target"]["format"]
          for variable in dashboard["templating"]["list"]:
              yield variable["query"], "table"

      grafana = "curl -sf -u admin:test -H 'Content-Type: application/json'"
      for path in sorted(glob.glob("${self.lib.python.mkGrafanaDashboards pkgs}/*.json")):
          with open(path) as file:
              dashboard = json.load(file)
          pidgeon.wait_until_succeeds(
              f"{grafana} http://127.0.0.1:3000/api/dashboards/uid/{dashboard['uid']}",
              timeout=60,
          )
          for sql, fmt in dashboard_queries(dashboard):
              body = json.dumps({
                  "from": "now-1h",
                  "to": "now",
                  "queries": [{
                      "refId": "A",
                      "datasource": {"uid": "pidgeon-db"},
                      "rawSql": sql.replace("$device", "'pidgeon-test'"),
                      "format": fmt,
                      "intervalMs": 60000,
                      "maxDataPoints": 500,
                  }],
              })
              pidgeon.succeed(
                  f"{grafana} -d {shlex.quote(body)} http://127.0.0.1:3000/api/ds/query"
              )
    '';
  };
}
