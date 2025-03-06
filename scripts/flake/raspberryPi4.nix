{ self
, root
, lib
, specialArgs
, sops-nix
, pkgs
, config
, nixos-hardware
, ...
}:

{
  seal.overlays.raspberryPi4 =
    (final: prev: {
      # NOTE: https://github.com/NixOS/nixpkgs/issues/154163#issuecomment-1008362877  
      makeModulesClosure = x: prev.makeModulesClosure
        (x // { allowMissing = true; });
    });

  seal.deploy.nodes =
    builtins.listToAttrs
      (builtins.map
        (pidgeon: {
          name = "pidgeon-${pidgeon.id}-raspberryPi4";
          value = {
            hostname = pidgeon.ip;
            sshUser = "altibiz";
          };
        })
        self.lib.pidgeons);

  flake.nixosConfigurations =
    builtins.listToAttrs
      (builtins.map
        (pidgeon:
          {
            name = "pidgeon-${pidgeon.id}-raspberryPi4-aarch64-linux";
            value = lib.nixosSystem {
              system = "aarch64-linux";
              inherit specialArgs;
              modules = [
                self.nixosModules.raspberryPi4
                {
                  options.pidgeon.id = lib.mkOption {
                    type = lib.types.str;
                    default = pidgeon.id;
                  };
                  options.pidgeon.ip = lib.mkOption {
                    type = lib.types.str;
                    default = pidgeon.ip;
                  };
                  options.pidgeon.hostName = lib.mkOption {
                    type = lib.types.str;
                    default = "pidgeon-${pidgeon.id}";
                  };
                }
              ];
            };
          })
        self.lib.pidgeons);

  branch.nixosModule.nixosModule =
    let
      name = "pidgeon-${config.pidgeon.id}-raspberryPi4-aarch64-linux";
      secrets = self.lib.secrets.${name};
      secretKeys = secrets.keys;
    in
    {
      nixpkgs.config = {
        allowUnfree = true;
      };
      nixpkgs.overlays = [
        self.overlays.raspberryPi4
      ];

      imports = [
        nixos-hardware.nixosModules.raspberry-pi-4
        sops-nix.nixosModules.default
        self.nixosModules.pidgeonCli
      ];

      # system

      system.stateVersion = "24.11";

      nix.extraOptions = "experimental-features = nix-command flakes";
      nix.gc.automatic = true;
      nix.gc.options = "--delete-older-than 30d";
      nix.settings.auto-optimise-store = true;
      nix.settings.trusted-users = [ "@wheel" ];
      nix.package = pkgs.nixVersions.stable;

      sops.defaultSopsFile = lib.path.append root secrets.sopsFilePrefix;
      sops.age.keyFile = secrets.ageKeyFile;

      networking.hostName = config.pidgeon.hostName;

      fileSystems."/firmware" = {
        device = "/dev/disk/by-label/FIRMWARE";
        fsType = "vfat";
      };
      fileSystems."/" = {
        device = "/dev/disk/by-label/NIXOS_SD";
        fsType = "ext4";
      };

      environment.systemPackages = with pkgs; [
        libraspberrypi
        raspberrypi-eeprom
        man-pages
        man-pages-posix
        self.packages.${pkgs.system}.pidgeonProbe
        self.packages.${pkgs.system}.pidgeonCli
        mbpoll
      ];

      # service

      services.pidgeon.enable = true;
      services.pidgeon.debug = true;
      services.pidgeon.configPath = "/etc/pidgeon/config.toml";
      services.pidgeon.envPath = config.sops.secrets.${secretKeys.pidgeonEnv}.path;
      environment.etc."pidgeon/config.toml" = {
        source = "${self}/assets/pidgeon/config.toml";
        user = config.systemd.services.pidgeon.serviceConfig.User;
        group = config.systemd.services.pidgeon.serviceConfig.Group;
      };
      sops.secrets.${secretKeys.pidgeonEnv} = { };

      # user

      users.mutableUsers = false;
      users.groups.altibiz = { };
      users.users.altibiz = {
        group = "altibiz";
        isNormalUser = true;
        hashedPasswordFile =
          config.sops.secrets.${secretKeys.userHashedPasswordFile}.path;
        extraGroups = [ "wheel" "dialout" ];
        packages = [
          pkgs.kitty
          pkgs.git
          pkgs.helix
          pkgs.yazi
          pkgs.lazygit
          pkgs.nushell
        ];
      };
      sops.secrets.${secretKeys.userHashedPasswordFile}.neededForUsers = true;

      services.openssh.enable = true;
      services.openssh.settings.PasswordAuthentication = false;

      sops.secrets.${secretKeys.userAuthorizedKeys} = {
        path = "${config.users.users.altibiz.home}/.ssh/authorized_keys";
        owner = config.users.users.altibiz.name;
        group = config.users.users.altibiz.group;
      };

      # database

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
      networking.firewall.allowedTCPPorts = [ 5433 ];

      services.postgresql.settings.ssl = "on";
      services.postgresql.settings.ssl_cert_file = config.sops.secrets.${secretKeys.postgresSslCertFile}.path;
      sops.secrets.${secretKeys.postgresSslCertFile} = {
        owner = config.systemd.services.postgresql.serviceConfig.User;
        group = config.systemd.services.postgresql.serviceConfig.Group;
      };
      services.postgresql.settings.ssl_key_file = config.sops.secrets.${secretKeys.postgresSslKeyFile}.path;
      sops.secrets."postgres.crt" = {
        owner = config.systemd.services.postgresql.serviceConfig.User;
        group = config.systemd.services.postgresql.serviceConfig.Group;
      };
      services.postgresql.initialScript = config.sops.secrets.${secretKeys.postgresInitialScript}.path;
      sops.secrets.${secretKeys.postgresInitialScript} = {
        owner = config.systemd.services.postgresql.serviceConfig.User;
        group = config.systemd.services.postgresql.serviceConfig.Group;
      };

      services.postgresql.settings = {
        checkpoint_timeout = "30min";
        checkpoint_completion_target = 0.9;
        max_wal_size = "1GB";
      };

      # vpn

      services.nebula.networks.ozds-vpn = {
        enable = true;
        cert = config.sops.secrets.${secretKeys.nebulaCert}.path;
        key = config.sops.secrets.${secretKeys.nebulaKey}.path;
        ca = config.sops.secrets.${secretKeys.nebulaCa}.path;
        firewall.inbound = [
          {
            host = "any";
            port = "any";
            proto = "any";
          }
        ];
        firewall.outbound = [
          {
            host = "any";
            port = "any";
            proto = "any";
          }
        ];
        lighthouses = [ "10.8.0.1" ];
        staticHostMap = {
          "10.8.0.1" = [
            "ozds-vpn.altibiz.com:4242"
          ];
        };
        settings = {
          relay = {
            relays = [ "10.8.0.1" ];
          };
          punchy = {
            punch = true;
            delay = "1s";
            respond = true;
            respond_delay = "5s";
          };
          handshakes = {
            try_interval = "1s";
          };
          static_map = {
            cadence = "1m";
            lookup_timeout = "10s";
          };
          logging = {
            level = "debug";
          };
        };
      };
      sops.secrets.${secretKeys.nebulaCert} = {
        owner = config.systemd.services."nebula@ozds-vpn".serviceConfig.User;
        group = config.systemd.services."nebula@ozds-vpn".serviceConfig.Group;
      };
      sops.secrets.${secretKeys.nebulaKey} = {
        owner = config.systemd.services."nebula@ozds-vpn".serviceConfig.User;
        group = config.systemd.services."nebula@ozds-vpn".serviceConfig.Group;
      };
      sops.secrets.${secretKeys.nebulaCa} = {
        owner = config.systemd.services."nebula@ozds-vpn".serviceConfig.User;
        group = config.systemd.services."nebula@ozds-vpn".serviceConfig.Group;
      };

      # network

      networking.firewall.enable = true;
      networking.networkmanager.enable = true;
      networking.nameservers = [ "1.1.1.1" "1.0.0.1" ];

      networking.networkmanager.wifi.powersave = false;
      networking.networkmanager.ensureProfiles.profiles = {
        "wifi" = {
          connection = {
            id = "wifi";
            permissions = "";
            type = "wifi";
            interface-name = "wlan0";
          };
          ipv4 = {
            dns-search = "";
            method = "auto";
          };
          ipv6 = {
            addr-gen-mode = "stable-privacy";
            dns-search = "";
            method = "auto";
          };
          wifi = {
            mac-address-blacklist = "";
            mode = "infrastructure";
            ssid = "$WIFI_SSID";
          };
          wifi-security = {
            auth-alg = "open";
            key-mgmt = "wpa-psk";
            psk = "$WIFI_PASS";
          };
        };
      };

      networking.networkmanager.ensureProfiles.environmentFiles = [
        config.sops.secrets.${secretKeys.networkManagerEnvironmentFile}.path
      ];
      sops.secrets.${secretKeys.networkManagerEnvironmentFile} = { };

      # hardware

      services.fstrim.enable = true;

      boot.kernelModules = [ "i2c_dev" "spidev" ];
      hardware.deviceTree.overlays = [
        {
          # NOTE: https://github.com/raspberrypi/linux/blob/rpi-6.6.y/arch/arm/boot/dts/overlays/i2c-bcm2708-overlay.dts
          name = "i2c1-okay-overlay";
          dtsText = ''
            /dts-v1/;
            /plugin/;
            / {
              compatible = "brcm,bcm2711";
              fragment@0 {
                target = <&i2c1>;
                __overlay__ {
                  status = "okay";
                };
              };
            };
          '';
        }
        {
          # NOTE: https://github.com/raspberrypi/linux/blob/rpi-6.6.y/arch/arm/boot/dts/overlays/sc16is752-spi1-overlay.dts
          name = "sc16is752-spi1-overlay";
          dtsText = ''
            /dts-v1/;
            /plugin/;

            / {
              compatible = "brcm,bcm2711";

            	fragment@0 {
            		target = <&gpio>;
            		__overlay__ {
            			spi1_pins: spi1_pins {
            				brcm,pins = <19 20 21>;
            				brcm,function = <3>; /* alt4 */
            			};

            			spi1_cs_pins: spi1_cs_pins {
            				brcm,pins = <18>;
            				brcm,function = <1>; /* output */
            			};

            			int_pins: int_pins@18 {
            					brcm,pins = <24>;
            					brcm,function = <0>; /* in */
            					brcm,pull = <0>; /* none */
            			};
            		};
            	};

            	fragment@1 {
            		target = <&spi1>;
            		__overlay__ {
            			#address-cells = <1>;
            			#size-cells = <0>;
            			pinctrl-names = "default";
            			pinctrl-0 = <&spi1_pins &spi1_cs_pins>;
            			cs-gpios = <&gpio 18 1>;
            			status = "okay";

            			sc16is752: sc16is752@0 {
            				compatible = "nxp,sc16is752";
            				reg = <0>; /* CE0 */
            				clocks = <&sc16is752_clk>;
            				interrupt-parent = <&gpio>;
            				interrupts = <24 2>; /* IRQ_TYPE_EDGE_FALLING */
            				pinctrl-0 = <&int_pins>;
            				pinctrl-names = "default";
            				gpio-controller;
            				#gpio-cells = <2>;
            				spi-max-frequency = <4000000>;
            			};
            		};
            	};

            	fragment@2 {
            		target = <&aux>;
            		__overlay__ {
            			status = "okay";
            		};
            	};

            	fragment@3 {
            		target-path = "/";
            		__overlay__ {
            			sc16is752_clk: sc16is752_spi1_0_clk {
            				compatible = "fixed-clock";
            				#clock-cells = <0>;
            				clock-frequency = <14745600>;
            			};
            		};
            	};

            	__overrides__ {
            		int_pin = <&sc16is752>,"interrupts:0", <&int_pins>,"brcm,pins:0",
            			  <&int_pins>,"reg:0";
            		xtal = <&sc16is752_clk>,"clock-frequency:0";
            	};
            };
          '';
        }
      ];

      # visualization

      # services.grafana.enable = true;
      # services.grafana.settings = {
      #   server = {
      #     http_addr = "0.0.0.0";
      #     http_port = 3000;
      #   };
      #   date_formats = {
      #     default_timezone = "utc";
      #   };
      # };

      # networking.firewall.allowedTCPPorts = [
      #   3000
      # ];

      # services.grafana.provision.enable = true;
      # services.grafana.provision.datasources.settings = {
      #   apiVersion = 1;
      #   datasources = [
      #     {
      #       name = "Pidgeon DB";
      #       type = "postgres";
      #       url = "localhost:5433";
      #       user = "pidgeon";
      #       jsonData = {
      #         database = "pidgeon";
      #         sslmode = "disable";
      #         version = 10; # NOTE: >=10
      #         timescaledb = true;
      #       };
      #     }
      #   ];
      # };
    };
}
