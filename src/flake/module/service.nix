{ self, config, ... }:

{
  system = {
    services.pidgeon.enable = true;

    services.pidgeon.configPath = "/etc/pidgeon/config.toml";
    environment.etc."pidgeon/config.toml" = {
      source = "${self}/assets/config.toml";
      user = config.systemd.services.pidgeon.serviceConfig.User;
      group = config.systemd.services.pidgeon.serviceConfig.Group;
    };

    services.pidgeon.envPath = config.sops.secrets."pidgeon.env".path;
    sops.secrets."pidgeon.env" = { };
  };
}
