{ self, lib, pkgs, specialArgs, ... }:

# NOTE: this is long running version of local vm of pidgeon
# that simulates half stack of the pidgeon service
# build it with 'nix build .#nixosConfigurations.pidgeon-vm-x86_64-linux.config.system.build.vm -o result-vm'
# './result/bin/run-pidgeon-vm', state is kept in './pidgeon.qcow2'

{
  flake.nixosConfigurations.pidgeon-vm-x86_64-linux = lib.nixosSystem {
    system = "x86_64-linux";
    inherit specialArgs;
    modules = [
      self.nixosModules.pidgeonVm
    ];
  };

  branch.nixosModule.nixosModule = {
    imports = [
      self.nixosModules.pidgeonHost
    ];

    system.stateVersion = "24.11";
    networking.hostName = "pidgeon";
    nixpkgs.config.allowUnfreePredicate = pkg:
      builtins.elem (lib.getName pkg) [ "timescaledb" ];

    # NOTE: static config of sops to 'emulate' the ones provided on the pi

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
      PIDGEON_CLOUD_ID=pidgeon-vm
      PIDGEON_NETWORK_IP_RANGE_START=127.0.0.1
      PIDGEON_NETWORK_IP_RANGE_END=127.0.0.1
    ''}";
    services.grafana.settings.security.admin_password = "$__file{${pkgs.writeText "grafana-admin-pass" "admin"}}";

    # user

    users.users.altibiz = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      initialPassword = "altibiz";
    };
    security.sudo.wheelNeedsPassword = false;
    services.openssh.enable = true;
    environment.systemPackages = [ pkgs.curl ];

    # vm settings

    virtualisation.vmVariant = {
      virtualisation.memorySize = 2048;
      virtualisation.cores = 4;
      virtualisation.diskSize = 4096;
      virtualisation.graphics = false;
      virtualisation.forwardPorts = [
        { from = "host"; host.address = "127.0.0.1"; host.port = 2222; guest.port = 22; }
      ];
    };
  };
}
