{ self, pkgs, ... }:

let
  package = self.packages.${pkgs.system}.pidgeonProbe;

  base = self.lib.gns3.mkBaseDockerImage pkgs;

  config = pkgs.writeTextFile {
    name = "config";
    destination = "/share/config.toml";
    text = builtins.readFile "${self}/assets/config.toml";
  };

  run = pkgs.writeShellApplication {
    name = "pidgeon-probe-docker";
    runtimeInputs = [ package ];
    text = ''
      export PIDGEON_PROBE_ENV=production
      ${package}/bin/pidgeon-probe "$@" --config '${config}/share/config.toml'
    '';
  };
in
{
  integrate.package.package = pkgs.dockerTools.buildImage {
    name = "altibiz/pidgeon-probe";
    tag = "latest";
    created = "now";
    fromImage = base;
    copyToRoot = pkgs.buildEnv {
      name = "image-root";
      paths = [ run config ];
      pathsToLink = [ "/bin" "/share" ];
    };
    config = {
      Cmd = [ "pidgeon-probe-docker" ];
    };
  };
}
