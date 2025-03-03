{ self, pkgs, ... }:

{
  integrate.devShell.devShell = pkgs.mkShell {
    inputsFrom = [
      (self.lib.vcs.mkDevShell pkgs)
      (self.lib.scripts.mkDevShell pkgs)
    ];

    packages = with pkgs; [
      # documentation
      mdbook
      mdbook-plantuml
      plantuml
      openjdk

      # rust
      rustc
      cargo
    ];
  };
}

