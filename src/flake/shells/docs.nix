{ pkgs, ... }:

pkgs.mkShell {
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  packages = with pkgs; [
    # Rust
    rustc
    cargo

    # Documentation
    mdbook
    mdbook-plantuml
    plantuml
    openjdk

    # Tools
    nushell
    just
    pkg-config
    openssl
  ];
}
