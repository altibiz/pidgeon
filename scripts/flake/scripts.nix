{
  flake.lib.scripts.mkDevShell = pkgs: pkgs.mkShell {
    packages = with pkgs; [
      bashInteractive
      nushell
      just
    ];
  };
}
