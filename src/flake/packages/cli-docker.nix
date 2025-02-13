{ self, pkgs, ... }:

let
  package = self.packages.${pkgs.system}.cli;

  base = self.lib.gns3.mkBaseDockerImage pkgs;

  config = pkgs.writeTextFile {
    name = "config";
    destination = "/share/config.toml";
    text = builtins.readFile "${self}/assets/config.toml";
  };

  run = pkgs.writeShellApplication {
    name = "pidgeon-docker";
    runtimeInputs = [ package ];
    text = ''
      ${package}/bin/pidgeon "$@" --config '${config}/share/config.toml'
    '';
  };
in
pkgs.dockerTools.buildImage {
  name = "altibiz/pidgeon";
  tag = "latest";
  created = "now";
  fromImage = base;
  copyToRoot = pkgs.buildEnv {
    name = "image-root";
    paths = [ run config ];
    pathsToLink = [ "/bin" "/share" ];
  };
  config = {
    Cmd = [ "pidgeon-docker" ];
  };
}
