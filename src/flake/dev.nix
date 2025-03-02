{ lib
, pkgs
, self
, ...
}:

{
  seal.defaults.overlay = "dev";
  seal.overlays.dev = (final: prev: {
    nodejs = prev.nodejs_20;
  });

  seal.defaults.devShell = "dev";
  integrate.devShell.devShell = pkgs.mkShell {
    inputsFrom = [
      (self.lib.python.mkDevShell pkgs)
      (self.lib.rust.mkDevShell pkgs)
    ];

    packages = with pkgs; [
      # version control
      git
      dvc-with-remotes

      # scripts
      nushell
      just

      # misc
      nodePackages.prettier
      nodePackages.yaml-language-server
      marksman
      taplo

      # spelling
      nodePackages.cspell

      # documentation
      simple-http-server
      mdbook
      mdbook-plantuml
      plantuml
      openjdk
      pandoc
      pandoc-plantuml-filter

      # shell
      bashInteractive
      nodePackages.bash-language-server
      shfmt
      shellcheck

      # nix
      nil
      nixpkgs-fmt

      # tools
      (writeShellApplication {
        name = "usqll";
        runtimeInputs = [ usql ];
        text = ''
          usql ${databaseUrl} "$@"
        '';
      })
      usql
      postgresql_14
      openssh
      age
      sqlx-cli
      jq
      sops
      zip
      unzip
      zstd
      mbpoll
      i2c-tools
      nebula
      nixos-generators
      gum
      deploy-rs
      sshpass
      mkpasswd
    ] ++ lib.optionals
      (
        pkgs.stdenv.hostPlatform.isLinux
          && pkgs.stdenv.hostPlatform.isx86_64
      ) [
      libguestfs-with-appliance
    ];
  };
}
