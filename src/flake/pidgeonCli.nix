{ self, pkgs, ... }:

{
  integrate.package.package = self.lib.rust.mkPackage pkgs;
}
