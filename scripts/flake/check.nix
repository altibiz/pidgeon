{ self, pkgs, ... }:

{
  integrate.devShell.devShell = pkgs.mkShell {
    inputsFrom = [
      (self.lib.vcs.mkDevShell pkgs)
      (self.lib.scripts.mkDevShell pkgs)
      (self.lib.lint.mkDevShell pkgs)
      (self.lib.format.mkDevShell pkgs)
    ];
  };
}
