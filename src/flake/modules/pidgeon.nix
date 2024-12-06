{ self, config, ... }:

{
  services.pidgeon.enable = true;

  services.pidgeon.configPath = "/etc/pidgeon/config.toml";
  environment.etc."pidgeon/config.toml" = {
    source = "${self}/assets/config.toml";
    user = "pidgeon";
    group = "pidgeon";
    mode = "0644";
  };

  services.pidgeon.envPath = config.sops.secrets."pidgeon.env".path;
  sops.secrets."pidgeon.env" = { };
}
