{ self
, nixpkgs
, sops-nix
, home-manager
, ...
} @inputs:

{
  modules =
    builtins.map
      (x: x.__import.value)
      (builtins.filter
        (x: x.__import.type == "default")
        (nixpkgs.lib.collect
          (builtins.hasAttr "__import")
          (self.lib.import.importDirMeta "${self}/src/flake/module")));

  mkNixosConfiguration = host:
    let
      specialArgs = inputs // { inherit host; };
    in
    nixpkgs.lib.nixosSystem {
      inherit specialArgs;
      system = host.system;
      modules = [
        sops-nix.nixosModules.default
        home-manager.nixosModules.default
        self.nixosModules."${host.name}-${host.system}"
        {
          home-manager.backupFileExtension = "backup";
          home-manager.useUserPackages = true;
          home-manager.extraSpecialArgs = specialArgs;
          home-manager.sharedModules = [
            sops-nix.homeManagerModules.sops
          ];
          home-manager.users."${host.user}" =
            self.hmModules."${host.user}-${host.system}";

        }
      ];
    };
}
