{ self, pkgs, ... }:

let
  unwrapped = pkgs.rustPlatform.buildRustPackage {
    pname = "pidgeon-cli";
    version = "0.1.0";
    src = self;
    cargoHash = "sha256-boUe+RMZyUzDdtdFAYt5v34ESATxDLghzNgFk/jDIEE=";
    nativeBuildInputs = [
      pkgs.pkg-config
    ];
    buildInputs = [
      pkgs.openssl
    ];
    meta = {
      description = "Raspberry Pi message broker";
      homepage = "https://github.com/altibiz/pidgeon";
      license = pkgs.lib.licenses.mit;
    };
  };
in
pkgs.writeShellApplication {
  name = "pidgeon";
  runtimeInputs = [ unwrapped ];
  text = ''
    ${unwrapped}/bin/pidgeon-cli "$@"
  '';
}
