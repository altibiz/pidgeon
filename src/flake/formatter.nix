{ nixpkgs, ... }:

{
  mkFormatter = system:
    let
      pkgs = import nixpkgs { inherit system; };

      formatter = pkgs.writeShellApplication {
        name = "formatter";
        runtimeInputs = [
          pkgs.just
          pkgs.nodePackages.prettier
          pkgs.nixpkgs-fmt
          pkgs.shfmt
          pkgs.yapf
          pkgs.cargo
          pkgs.rustfmt
          pkgs.clippy
        ];
        text = ''
          cd "$(git rev-parse --show-toplevel)"
          just --unstable --fmt
          prettier --write "$(git rev-parse --show-toplevel)"
          nixpkgs-fmt "$(git rev-parse --show-toplevel)"
          shfmt --write "$(git rev-parse --show-toplevel)"
          yapf --recursive --in-place --parallel "$(git rev-parse --show-toplevel)"/src/probe
          cargo fmt --all
          cargo clippy --fix --allow-dirty --allow-staged
        '';
      };
    in
    formatter;
}
