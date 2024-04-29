{ self, nixpkgs, ... } @inputs:

nixpkgs.lib.nixosSystem {
  system = "aarch64-linux";
  specialArgs = inputs;
  modules = builtins.attrValues self.nixosModules;
}
