{ self
, naersk
, ...
}:

{
  flake.lib.rust.package = pkgs:
    let
      naersk' = pkgs.callPackage naersk { };
    in
    naersk'.buildPackage {
      src = self;
    };
}
