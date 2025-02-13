{ ... }:

{
  disabled = true;

  system = {
    services.grafana.enable = true;
    services.grafana.settings = {
      server = {
        http_addr = "0.0.0.0";
        http_port = 3000;
      };
      date_formats = {
        default_timezone = "utc";
      };
    };

    networking.firewall.allowedTCPPorts = [
      3000
    ];

    services.grafana.provision.enable = true;
    services.grafana.provision.datasources.settings = {
      apiVersion = 1;
      datasources = [
        {
          name = "Pidgeon DB";
          type = "postgres";
          url = "localhost:5433";
          user = "pidgeon";
          jsonData = {
            database = "pidgeon";
            sslmode = "disable";
            version = 10; # NOTE: >=10
            timescaledb = true;
          };
        }
      ];
    };
  };
}
