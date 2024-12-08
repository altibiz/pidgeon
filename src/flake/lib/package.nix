{ self, nixpkgs, ... } @rawInputs:

let
  mkImported = system:
    let
      pkgs = import nixpkgs { inherit system; };
      inputs = rawInputs // { inherit pkgs; };
    in
    self.lib.import.importDirWrap
      (x: x.__import.value inputs)
      "${self}/src/flake/package";
in
{
  mkPackages = system:
    let
      imported = mkImported system;
    in
    imported // { default = imported.cli; };
}
