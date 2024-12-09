{ self, nixpkgs, ... } @inputs:

let
  imported =
    self.lib.import.importDirWrap
      (x: x.__import.value inputs)
      "${self}/src/flake/overlay";

  composed =
    nixpkgs.lib.foldl'
      nixpkgs.lib.composeExtensions
      (_: _: { })
      (builtins.map
        (x: x.__import.value)
        (nixpkgs.lib.collect
          (builtins.hasAttr "__import")
          (self.lib.import.importDirMeta "${self}/src/flake/overlay")));
in
{
  overlays = imported // { default = composed; };
}
