{ pkgs, ... }:

pkgs.mkShell {
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

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

    # build inputs
    pkg-config
    openssl
    systemd
  ];
}
