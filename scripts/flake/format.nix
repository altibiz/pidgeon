{ self, ... }:

{
  flake.lib.format.mkDevShell = pkgs: pkgs.mkShell {
    inputsFrom = [
      (self.lib.python.mkDevShell pkgs)
      (self.lib.rust.mkDevShell pkgs)
    ];
    packages = with pkgs; [
      nodePackages.prettier
      just
      nixpkgs-fmt
      shfmt
    ];
  };
}
