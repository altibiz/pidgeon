{ self, nixpkgs, ... } @inputs:

let
  imported =
    self.lib.import.importDirWrap
      (x: x.__import.value inputs)
      "${self}/src/flake/overlay";

  composed =
    nixpkgs.lib.composeManyExtensions
      (builtins.map
        (x: x.__import.value inputs)
        (nixpkgs.lib.collect
          (builtins.hasAttr "__import")
          (self.lib.import.importDirMeta "${self}/src/flake/overlay")));
in
{
  overlays = imported // { default = composed; };
}
