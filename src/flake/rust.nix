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
      name = "pidgeon-cli";
      pname = "pidgeon-cli";
      version = "0.1.0";
      src = self;
      buildInputs = with pkgs; [
        pkg-config
        openssl
        systemd
      ];
    };
}
