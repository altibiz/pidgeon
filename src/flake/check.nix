{ pkgs, ... }:

pkgs.mkShell {
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  packages = with pkgs; [
    # Nix
    nixpkgs-fmt

    # Python
    poetry
    pyright
    yapf
    ruff

    # Rust
    rustc
    cargo
    clippy
    rustfmt
    pkg-config
    openssl

    # Shell
    shfmt
    shellcheck

    # Misc
    nodePackages.prettier

    # Tools
    nushell
    just
  ];
}
