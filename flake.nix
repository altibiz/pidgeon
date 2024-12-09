{
  description = "Pidgeon - Raspberry Pi message broker.";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";

    nixpkgs.url = "github:nixos/nixpkgs/release-24.11";

    deploy-rs.url = "github:serokell/deploy-rs";
    deploy-rs.inputs.nixpkgs.follows = "nixpkgs";
    deploy-rs.inputs.utils.follows = "flake-utils";

    home-manager.url = "github:nix-community/home-manager/release-24.11";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";

    nixos-hardware.url = "github:NixOS/nixos-hardware/master";

    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";

    poetry2nix.url = "github:nix-community/poetry2nix";

    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { self
    , flake-utils
    , nixpkgs
    , deploy-rs
    , ...
    } @ rawInputs:
    let
      inputs = rawInputs;

      libPart = {
        lib = nixpkgs.lib.mapAttrs'
          (name: value: { inherit name; value = value inputs; })
          (((import "${self}/src/flake/lib/import.nix") inputs).importDir "${self}/src/flake/lib");

        overlays = self.lib.overlay.overlays;
      };

      systemPart = flake-utils.lib.eachDefaultSystem (system:
        if nixpkgs.lib.hasSuffix "darwin" system then { }
        else {
          devShells = self.lib.devShell.mkDevShells system;
          formatter = self.lib.formatter.mkFormatter system;
          checks = self.lib.check.mkChecks system;

          packages = self.lib.package.mkPackages system;
          apps = self.lib.app.mkApps system;
        });

      hostPart =
        let
          invokeForHostSystemMatrix = mk: nixpkgs.lib.mergeAttrsList
            (builtins.map
              ({ host, system }: {
                "${host}-${system}" = mk (self.lib.host.mkHost host system);
              })
              (nixpkgs.lib.cartesianProduct {
                host = self.lib.host.hosts;
                system =
                  builtins.filter
                    (system:
                      (nixpkgs.lib.hasPrefix "aarch64" system)
                      && (nixpkgs.lib.hasSuffix "linux" system))
                    flake-utils.lib.defaultSystems;
              }));
        in
        {
          nixosModules = invokeForHostSystemMatrix self.lib.nixosModule.mkNixosModule;
          hmModules = invokeForHostSystemMatrix self.lib.hmModule.mkHmModule;
          nixosConfigurations = invokeForHostSystemMatrix self.lib.nixosConfiguration.mkNixosConfiguration;
          deploy.nodes = invokeForHostSystemMatrix self.lib.deploy.mkDeploy;
        };
    in
    libPart // systemPart // hostPart;
}
