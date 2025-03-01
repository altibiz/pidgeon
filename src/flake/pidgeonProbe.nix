{ self, pkgs, ... }:

{
  integrate.package.package = self.lib.python.mkPackage pkgs;
}
