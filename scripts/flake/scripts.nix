{
  flake.lib.scripts.mkDevShell = pkgs: pkgs.mkShell {
    packages = with pkgs; [
      just
      bash
      nushell
      gum
    ];
  };
}
