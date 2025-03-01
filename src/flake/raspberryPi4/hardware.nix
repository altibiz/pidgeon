{ pkgs
, nixos-hardware
, ...
}:

{
  branch.nisoxModule.nixosModule = {
    imports = [
      nixos-hardware.nixosModules.raspberry-pi-4
    ];

    environment.systemPackages = with pkgs; [
      libraspberrypi
      raspberrypi-eeprom
    ];

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
  };
}

