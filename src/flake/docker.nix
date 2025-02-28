{ self, ... }:

let
  mkDockerCompose = pkgs:
    let
      package = pkgs.runCommand
        "docker-compose-json"
        { buildInputs = [ pkgs.yq ]; }
        ''
          mkdir $out
          cat "${self}/docker-compose.yml" | yq > $out/result
        '';
    in
    builtins.fromJSON
      (builtins.readFile "${package}/result");
in
{
  inherit mkDockerCompose;

  mkDockerComposePostgres = pkgs:
    let
      dockerCompose = mkDockerCompose pkgs;

      fromEnv = name:
        builtins.elemAt
          (builtins.split "="
            (builtins.head
              (builtins.filter
                (var: (builtins.head (builtins.split "=" var))
                  == name)
                dockerCompose.services.postgres.environment))) 2;

      fromPorts = port:
        builtins.head
          (builtins.split ":"
            (builtins.head
              (builtins.filter
                (var: (builtins.elemAt (builtins.split ":" var) 2) == port)
                dockerCompose.services.postgres.ports)));
    in
    {
      host = "localhost";
      port = fromPorts "5432";
      database = fromEnv "POSTGRES_DB";
      user = fromEnv "POSTGRES_USER";
      password = fromEnv "POSTGRES_PASSWORD";
    };
}
