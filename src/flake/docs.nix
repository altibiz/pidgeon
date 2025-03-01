{ pkgs, ... }:

{
  integrate.devShell.devShell = pkgs.mkShell {
    packages = with pkgs; [
      # scripts
      nushell
      just

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

