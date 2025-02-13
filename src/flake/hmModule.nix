{ self, ... }:

{
  mkHmModule = host:
    ({ lib, ... }: {
      imports =
        (builtins.map
          self.lib.module.mkHomeModule
          self.lib.nixosConfiguration.modules)
        ++ [
          (self.lib.module.mkHomeModule host.config)
          (self.lib.module.mkHomeModule host.sharedConfig)
          { pidgeon = host.static; }
        ]
      ;

      options = {
        pidgeon.static = lib.mkOption {
          type = lib.types.raw;
        };
      };

      config = {
        pidgeon.static = self.lib.static.parseDir "${self}/src/flake/host";

        sops.defaultSopsFile = host.secrets;
        sops.age.keyFile = "/root/host.scrt.key";

        home.stateVersion = host.version;
        home.username = "${host.user}";
        home.homeDirectory = "/home/${host.user}";
      };
    });
}
