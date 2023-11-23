{
  description = "Raspberry Pi message broker";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-stable.url = "github:NixOS/nixpkgs/release-23.05";

    nixos-hardware.url = "github:NixOS/nixos-hardware/master";

    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";
    sops-nix.inputs.nixpkgs-stable.follows = "nixpkgs-stable";
  };

  outputs = { self, nixpkgs, home-manager, sops-nix, ... } @ inputs: {
    # TODO: cli service, config, env
    # TODO: cert renewal?
    nixosConfigurations.pidgeon = nixpkgs.lib.nixosSystem {
      system = "aarch64-linux";
      specialArgs = inputs;
      modules = [
        sops-nix.nixosModules.sops

        ({ nixos-hardware, modulesPath, ... }: {
          imports = [
            nixos-hardware.nixosModules.raspberry-pi-4
            # NOTE: doesn't work without this for now
            # it should work with just `nixos-generate`though
            "${modulesPath}/installer/sd-card/sd-image-aarch64.nix"
          ];

          # TODO: https://github.com/NixOS/nixpkgs/issues/154163#issuecomment-1008362877  
          nixpkgs.overlays = [
            (final: super: {
              makeModulesClosure = x:
                super.makeModulesClosure (x // { allowMissing = true; });
            })
          ];
        })

        ({ pkgs, config, ... }: {
          sops.defaultSopsFile = ./secrets/secrets.enc.yaml;
          environment.etc."sops-nix/key.txt" = {
            text = (builtins.readFile ./secrets/pidgeon-age.key);
            mode = "0600";
          };
          sops.age.keyFile = "/etc/sops-nix/key.txt";

          nix.package = pkgs.nixFlakes;
          nix.extraOptions = "experimental-features = nix-command flakes";
          nixpkgs.config = { allowUnfree = true; };

          location.provider = "geoclue2";
          time.timeZone = "Etc/UTC";
          i18n.defaultLocale = "en_US.UTF-8";

          services.openssh.enable = true;
          networking.hostName = "pidgeon";

          environment.systemPackages = with pkgs; [
            libraspberrypi
            raspberrypi-eeprom
            man-pages
            man-pages-posix
            openssl
            pkg-config
            age
            ssh-to-age
            sops
            git
            helix
          ];
          environment.sessionVariables = {
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };

          services.postgresql.enable = true;
          services.postgresql.package = pkgs.postgresql_14;
          services.postgresql.extraPlugins = with config.services.postgresql.package.pkgs; [
            timescaledb
          ];
          services.postgresql.settings.shared_preload_libraries = "timescaledb";
          services.postgresql.ensureDatabases = [ "pidgeon" ];
          services.postgresql.ensureUsers = [
            {
              name = "pidgeon";
              ensurePermissions = {
                "DATABASE pidgeon" = "ALL PRIVILEGES";
              };
              ensureClauses = {
                login = true;
              };
            }
          ];
          services.postgresql.authentication = pkgs.lib.mkOverride 10 ''
            # NOTE: do not remove local privileges because that breaks timescaledb
            # TYPE    DATABASE    USER        ADDRESS         METHOD        OPTIONS
            local     all         all                         trust
            host      all         all         samehost        trust
            hostssl   all         all         192.168.0.0/16  scram-sha-256
            hostssl   all         all         10.255.255.0/24 scram-sha-256
          '';
          services.postgresql.enableTCPIP = true;
          services.postgresql.port = 5432;
          networking.firewall.allowedTCPPorts = [ 5432 ];
          services.postgresql.settings.ssl = "on";
          services.postgresql.settings.ssl_cert_file = "/etc/postgresql/server.crt";
          sops.secrets."postgres.crt".path = "/etc/postgresql/server.crt";
          sops.secrets."postgres.crt".owner = "postgres";
          sops.secrets."postgres.crt".group = "postgres";
          sops.secrets."postgres.crt".mode = "0600";
          services.postgresql.settings.ssl_key_file = "/etc/postgresql/server.key";
          sops.secrets."postgres.key".path = "/etc/postgresql/server.key";
          sops.secrets."postgres.key".owner = "postgres";
          sops.secrets."postgres.key".group = "postgres";
          sops.secrets."postgres.key".mode = "0600";
          services.postgresql.initialScript = "/etc/postgresql/passwords.sql";
          sops.secrets."passwords.sql".path = "/etc/postgresql/passwords.sql";
          sops.secrets."passwords.sql".owner = "postgres";
          sops.secrets."passwords.sql".group = "postgres";
          sops.secrets."passwords.sql".mode = "0600";

          users.users."pidgeon" = {
            isNormalUser = true;
            hashedPassword = (builtins.readFile ./secrets/password.pub);
            extraGroups = [ "wheel" ];
            shell = pkgs.bashInteractive;
            openssh.authorizedKeys.keys = [
              (builtins.readFile ./secrets/authorized.pub)
            ];
          };

          programs.direnv.enable = true;
          programs.direnv.nix-direnv.enable = true;

          system.stateVersion = "23.11";
        })
      ];
    };
  };
}
