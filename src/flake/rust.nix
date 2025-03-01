{ self
, naersk
, ...
}:

{
  flake.lib.rust.mkPackage = pkgs:
    let
      naersk' = pkgs.callPackage naersk { };
    in
    naersk'.buildPackage {
      src = self;
    };
}
