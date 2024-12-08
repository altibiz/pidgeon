{ self, nixpkgs, ... } @rawInputs:

let
  mkImported = system:
    let
      pkgs = import nixpkgs { inherit system; };
      inputs = rawInputs // { inherit pkgs; };
    in
    self.lib.import.importDirWrap
      (import: import.__import.value inputs)
      "${self}/src/flake/shell";
in
{
  mkDevShells = system:
    let
      imported = mkImported system;
    in
    imported // { default = imported.dev; };
}
