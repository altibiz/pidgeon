{ self, ... }:

{
  mkNixosModule = host:
    ({ pkgs, lib, config, ... }: {
      imports =
        (builtins.map
          self.lib.module.mkSystemModule
          self.lib.nixosConfiguration.modules)
        ++ [
          (self.lib.module.mkSystemModule host.config)
          (self.lib.module.mkSystemModule host.sharedConfig)
          { pidgeon = host.static; }
        ]
      ;

      options = {
        pidgeon.static = lib.mkOption {
          type = lib.types.raw;
        };
      };

      config = {
        pidgeon.static = self.lib.static.parseDir "${self}/flake/src/host";

        sops.defaultSopsFile = host.secrets;
        sops.age.keyFile = "/root/host.scrt.key";

        networking.hostName = host;
        system.stateVersion = host.version;

        users.mutableUsers = false;
        users.groups.${host.group} = {
          gid = host.gid;
        };

        users.defaultUserShell = "${pkgs.bashInteractive}/bin/bash";
        sops.secrets."${host.name}.pass.pub".neededForUsers = true;
        users.users.${host.user} = {
          uid = host.uid;
          home = "/home/${host.user}";
          isNormalUser = true;
          createHome = true;
          hashedPasswordFile = config.sops.secrets."${host.name}.pass.pub".path;
          extraGroups = [ "wheel" "dialout" ];
          useDefaultShell = true;
        };
      };
    });
}
