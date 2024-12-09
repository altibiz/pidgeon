{ self, nixpkgs, ... }:

{
  hosts =
    builtins.attrNames
      (nixpkgs.lib.filterAttrs
        (_: type: type == "directory")
        (builtins.readDir "${self}/src/flake/host"));

  mkHost = name: system:
    let
      staticPath = "${self}/src/flake/host/${name}/static.json";
      sharedStaticPath = "${self}/flake/src/host/static.json";

      configPath = "${self}/src/flake/host/${name}/config.nix";
      sharedConfigPath = "${self}/flake/src/host/config.nix";

      secrets = "${self}/src/flake/host/${name}/secrets.yaml";
    in
    let
      staticObject =
        if builtins.pathExists staticPath
        then builtins.fromJSON (builtins.readFile staticPath)
        else { };
      sharedStaticObject =
        if builtins.pathExists sharedStaticPath
        then builtins.fromJSON (builtins.readFile sharedStaticPath)
        else { };
    in
    let
      static = nixpkgs.lib.recursiveUpdate sharedStaticObject staticObject;
    in
    {
      version = "24.11";
      user = "altibiz";
      group = "users";
      uid = 1000;
      gid = 100;

      inherit name system;

      inherit static;

      config =
        if builtins.pathExists configPath
        then import configPath
        else { };
      sharedConfig =
        if builtins.pathExists sharedConfigPath
        then import sharedConfigPath
        else { };

      secrets = if builtins.pathExists secrets then secrets else null;
    };
}
