{ self, nixpkgs, ... } @rawInputs:

let
  mkImported = system:
    let
      pkgs = import nixpkgs { inherit system; };
      inputs = rawInputs // { inherit pkgs; };
    in
    self.lib.import.importDirWrap
      (x: {
        type = "app";
        program = nixpkgs.lib.getExe
          (x.__import.value
            (inputs // { inherit pkgs; }));
      })
      "${self}/src/flake/package";
in
{
  mkApps = system:
    let
      imported = mkImported system;
    in
    imported // { default = imported.cli; };
}
