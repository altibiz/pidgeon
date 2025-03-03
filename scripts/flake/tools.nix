{ self, ... }:

{
  flake.lib.tools.mkDevShell = pkgs:
    let
      postgres = self.lib.dockerCompose.mkDockerComposePostgres pkgs;

      databaseUrl =
        let
          auth = "${postgres.user}:${postgres.password}";
          conn = "${postgres.host}:${postgres.port}";
          db = postgres.database;
        in
        "postgres://${auth}@${conn}/${db}?sslmode=disable";
    in
    pkgs.mkShell {
      packages = with pkgs; [
        # documentation
        simple-http-server
        pandoc
        pandoc-plantuml-filter

        # database
        (writeShellApplication {
          name = "usqll";
          runtimeInputs = [ usql ];
          text = ''
            usql ${databaseUrl} "$@"
          '';
        })
        usql
        postgresql_14
        sqlx-cli

        # integrations
        nebula
        mbpoll
        i2c-tools

        # e2e
        zip
        unzip

        # rpi
        zstd
        nixos-generators
        deploy-rs
        sshpass
      ] ++ lib.optionals
        (
          pkgs.stdenv.hostPlatform.isLinux
            && pkgs.stdenv.hostPlatform.isx86_64
        ) [
        libguestfs-with-appliance
      ] ++ [

        # secrets
        openssh
        age
        sops
        mkpasswd
      ];
    };
}
