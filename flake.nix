{
  description = "Pidgeon - Raspberry Pi message broker.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }: utils.lib.simpleFlake {
    inherit self nixpkgs;
    name = "pidgeon";
    config = { allowUnfree = true; };
    shell = { pkgs }: pkgs.mkShell {
      packages = with pkgs; let
        pyright = pkgs.writeShellApplication {
          name = "pyright-langserver";
          runtimeInputs = [ pkgs.poetry pkgs.nodePackages.pyright ];
          text = ''
            # shellcheck disable=SC1091
            source "$(poetry env info --path)/bin/activate"
            pyright-langserver "$@"
          '';
        };

        python = pkgs.writeShellApplication {
          name = "python";
          runtimeInputs = [ pkgs.poetry ];
          text = ''
            poetry run python "$@"
          '';
        };
      in
      [
        # Nix
        nil
        nixpkgs-fmt

        # Python
        poetry
        pyright
        python
        (python310.withPackages
          (pythonPackages: with pythonPackages; [
            yapf
          ]))

        # Rust
        llvmPackages.clangNoLibcxx
        llvmPackages.lldb
        rustc
        cargo
        clippy
        rustfmt
        rust-analyzer
        cargo-edit
        shfmt
        marksman
        taplo
      ];
    };
  };
}
