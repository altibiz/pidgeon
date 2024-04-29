{
  description = "Pidgeon - Raspberry Pi message broker.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = { allowUnfree = true; };
          overlays = [
            (final: prev: {
              nodejs = prev.nodejs_20;
              dotnet-sdk = prev.dotnet-sdk_8;
            })
          ];
        };

        python = pkgs.writeShellApplication {
          name = "python";
          runtimeInputs = [ pkgs.poetry ];
          text = ''
            # shellcheck disable=SC1091
            source "$(poetry env info --path)/bin/activate"
            python "$@"
          '';
        };

        pyright = pkgs.writeShellApplication {
          name = "pyright";
          runtimeInputs = [ pkgs.poetry pkgs.nodejs ];
          text = ''
            # shellcheck disable=SC1091
            source "$(poetry env info --path)/bin/activate"
            pyright "$@"
          '';
        };

        pyright-langserver = pkgs.writeShellApplication {
          name = "pyright-langserver";
          runtimeInputs = [ pkgs.poetry pkgs.nodejs ];
          text = ''
            # shellcheck disable=SC1091
            source "$(poetry env info --path)/bin/activate"
            pyright-langserver "$@"
          '';
        };

        yapf = pkgs.writeShellApplication {
          name = "yapf";
          runtimeInputs = [ pkgs.poetry ];
          text = ''
            # shellcheck disable=SC1091
            source "$(poetry env info --path)/bin/activate"
            yapf "$@"
          '';
        };

        # NITPICK: https://github.com/astral-sh/ruff/issues/1699
        # ruff = pkgs.writeShellApplication {
        #   name = "ruff";
        #   runtimeInputs = [ pkgs.poetry ];
        #   text = ''
        #     # shellcheck disable=SC1091
        #     source "$(poetry env info --path)/bin/activate"
        #     ruff "$@"
        #   '';
        # };

        usql = pkgs.writeShellApplication {
          name = "usql";
          runtimeInputs = [ pkgs.usql ];
          text = ''
            usql pg://pidgeon:pidgeon@localhost:5433/pidgeon?sslmode=disable "$@"
          '';
        };
      in
      {
        devShells.check = pkgs.mkShell {
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
        };
        devShells.default = pkgs.mkShell {
          DATABASE_URL = "postgres://pidgeon:pidgeon@localhost:5433/pidgeon?sslmode=disable";
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

          # PIDGEON_CLOUD_SSL = "1";
          # PIDGEON_CLOUD_DOMAIN = "localhost:5001";
          # PIDGEON_CLOUD_API_KEY = "pidgeon";
          # PIDGEON_CLOUD_ID = "pidgeon";

          PIDGEON_DB_DOMAIN = "localhost";
          PIDGEON_DB_PORT = "5433";
          PIDGEON_DB_USER = "pidgeon";
          PIDGEON_DB_PASSWORD = "pidgeon";
          PIDGEON_DB_NAME = "pidgeon";

          # PIDGEON_NETWORK_IP_RANGE_START = "192.168.1.0";
          # PIDGEON_NETWORK_IP_RANGE_END = "192.168.1.255";

          packages = with pkgs; [
            # Nix
            nil
            nixpkgs-fmt

            # Python
            poetry
            python
            pyright
            pyright-langserver
            yapf
            ruff

            # Rust
            lldb
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer
            cargo-edit

            # Shell
            bashInteractiveFHS
            nodePackages.bash-language-server
            shfmt
            shellcheck

            # Misc
            nodePackages.prettier
            nodePackages.yaml-language-server
            marksman
            taplo

            # Tools
            nushell
            usql
            just
            openssh
            age
            pkg-config
            openssl
            sqlx-cli
            jq
            sops
          ];
        };
      });
}
