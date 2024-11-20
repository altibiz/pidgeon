{
  hardware = import ./hardware.nix;
  postgresql = import ./postgresql.nix;
  nebula = import ./nebula.nix;
  secrets = import ./secrets.nix;
  system = import ./system.nix;
  network = import ./network.nix;
  user = import ./user.nix;
  # FIXME: brighs the whole system down
  # visualization = import ./visualization.nix;
  pidgeon = import ./pidgeon.nix;
}
