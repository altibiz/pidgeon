{ self
, nixpkgs
, deploy-rs
, ...
}:

{
  mkDeploy = host:
    let
      pkgs = import nixpkgs { system = host.system; };
      deployPkgs = import nixpkgs {
        system = host.system;
        overlays = [
          deploy-rs.overlay
          (self: super: {
            deploy-rs = {
              inherit (pkgs) deploy-rs;
              lib = super.deploy-rs.lib;
            };
          })
        ];
      };
    in
    {
      hostname = host.static.vpn.ip;
      sshUser = host.user;
      profiles.system = {
        path =
          deployPkgs.deploy-rs.lib.activate.nixos
            self.nixosConfigurations."${host.name}-${host.system}";
      };
    };
}
