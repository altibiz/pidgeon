{ pkgs
, self
, ...
}:

{
  seal.defaults.overlay = "dev";
  seal.overlays.dev = (final: prev: {
    nodejs = prev.nodejs_20;
  });

  seal.defaults.devShell = "dev";
  integrate.devShell = {
    nixpkgs.config = {
      allowUnfree = true;
    };

    devShell = pkgs.mkShell {
      inputsFrom = [
        (self.lib.vcs.mkDevShell pkgs)
        (self.lib.scripts.mkDevShell pkgs)
        (self.lib.python.mkDevShell pkgs)
        (self.lib.rust.mkDevShell pkgs)
        self.devShells.${pkgs.system}.docs
        (self.lib.format.mkDevShell pkgs)
        (self.lib.lint.mkDevShell pkgs)
        (self.lib.tools.mkDevShell pkgs)
        (self.lib.lsp.mkDevShell pkgs)
      ];
    };
  };
}
