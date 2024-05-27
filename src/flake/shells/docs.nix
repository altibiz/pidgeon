{ pkgs, ... }:

pkgs.mkShell {
  packages = with pkgs; [
    # Rust
    rustc
    cargo

    # Tools
    nushell
    just
  ];
}
