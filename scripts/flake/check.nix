{ pkgs, ... }:

{
  integrate.devShell.devShell = pkgs.mkShell {
    packages = with pkgs; [
      # python
      uv

      # scripts
      nushell
      just

      # spelling
      nodePackages.cspell

      # misc
      nodePackages.prettier

      # shell
      shfmt
      shellcheck

      # nix
      nixpkgs-fmt

      # rust
      cargo
      clippy
      rustfmt
    ];
  };
}
