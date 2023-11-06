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
          runtimeInputs = [ pkgs.poetry ];
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

        python = pkgs.writeShellApplication {
          name = "python";
          runtimeInputs = [ pkgs.poetry ];
          text = ''
            # shellcheck disable=SC1091
            source "$(poetry env info --path)/bin/activate"
            python "$@"
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
        yapf

        # Rust
        llvmPackages.clangNoLibcxx
        llvmPackages.lldb
        rustc
        cargo
        clippy
        rustfmt
        rust-analyzer
        cargo-edit

        # Shell
        nodePackages.bash-language-server
        shfmt

        # Misc
        nodePackages.prettier
        nodePackages.yaml-language-server
        marksman
        taplo
      ];

      DATABASE_URL = "postgres://pidgeon:@localhost/pidgeon?sslmode=disable";

      # PIDGEON_CLOUD_SSL = "1";
      # PIDGEON_CLOUD_DOMAIN = "localhost:5001";
      # PIDGEON_CLOUD_API_KEY = "pidgeon";
      # PIDGEON_CLOUD_ID = "pidgeon";

      PIDGEON_DB_DOMAIN = "localhost";
      PIDGEON_DB_USER = "pidgeon";
      PIDGEON_DB_PASSWORD = "pidgeon";
      PIDGEON_DB_NAME = "pidgeon";

      # PIDGEON_NETWORK_IP_RANGE_START = "192.168.1.0";
      # PIDGEON_NETWORK_IP_RANGE_END = "192.168.1.255";
    };
  };
}
