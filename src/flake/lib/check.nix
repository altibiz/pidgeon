{ self, nixpkgs, deploy-rs, ... }:

{
  mkChecks = system:
    let
      pkgs = import nixpkgs { inherit system; };

      deployChecks = deploy-rs.lib.${system}.deployChecks self.deploy;

      selfChecks = {
        just = pkgs.writeShellApplication {
          name = "just";
          runtimeInputs = [ pkgs.just ];
          text = ''
            cd "$(git rev-parse --show-toplevel)"
            just --unstable --fmt --check
          '';
        };
        prettier = pkgs.writeShellApplication {
          name = "prettier";
          runtimeInputs = [ pkgs.nodePackages.prettier ];
          text = ''
            prettier --check "$(git rev-parse --show-toplevel)"
          '';
        };
        nixpkgs-fmt = pkgs.writeShellApplication {
          name = "nixpkgs-fmt";
          runtimeInputs = [ pkgs.nixpkgs-fmt ];
          text = ''
            nixpkgs-fmt "$(git rev-parse --show-toplevel)"
          '';
        };
        cspell = pkgs.writeShellApplication {
          name = "cspell";
          runtimeInputs = [ pkgs.nodePackages.cspell ];
          text = ''
            cspell lint "$(git rev-parse --show-toplevel)" --no-progress
          '';
        };
        shellcheck = pkgs.writeShellApplication {
          name = "shellcheck";
          runtimeInputs = [ pkgs.nodePackages.cspell ];
          text = ''
            for script in "$(git rev-parse --show-toplevel)"/scripts/*.sh; do
              shellcheck "$script"
            done
          '';
        };
        ruff = pkgs.writeShellApplication {
          name = "ruff";
          runtimeInputs = [ pkgs.ruff ];
          text = ''
            ruff check "$(git rev-parse --show-toplevel)"/src/probe
          '';
        };
        pyright = pkgs.writeShellApplication {
          name = "pyright";
          runtimeInputs = [ pkgs.pyright ];
          text = ''
            cd "$(git rev-parse --show-toplevel)"/src/probe
            pyright .
          '';
        };
        clippy = pkgs.writeShellApplication {
          name = "clippy";
          runtimeInputs = [ pkgs.cargo pkgs.clippy ];
          text = ''
            cd "$(git rev-parse --show-toplevel)"
            cargo clippy -- -D warnings
          '';
        };
      };
    in
    selfChecks // deployChecks;
}
