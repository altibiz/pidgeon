{ self, ... }:

{
  flake.lib.lint.mkDevShell = pkgs: pkgs.mkShell {
    inputsFrom = [
      (self.lib.python.mkDevShell pkgs)
    ];
    packages = with pkgs; [
      nodePackages.prettier
      nodePackages.cspell
      just
      nixpkgs-fmt
      cargo
      clippy
      shellcheck
    ];
  };
}
