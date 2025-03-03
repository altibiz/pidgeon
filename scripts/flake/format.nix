{
  flake.lib.format.mkDevShell = pkgs: pkgs.mkShell {
    packages = with pkgs; [
      nodePackages.prettier
      just
      nixpkgs-fmt
      cargo
      rustfmt
      yapf
      shfmt
    ];
  };
}
