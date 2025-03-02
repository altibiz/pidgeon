{ self, pkgs, ... }:

{
  seal.defaults.package = "pidgeonCli";
  seal.defaults.app = "pidgeonCli";
  integrate.package.package = self.lib.rust.mkPackage pkgs;
}
