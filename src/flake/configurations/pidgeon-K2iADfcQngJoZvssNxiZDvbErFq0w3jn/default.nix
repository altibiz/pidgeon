let
  ip = "10.8.0.17";
in
{
  hostname = ip;
  users = [ "altibiz" ];
  systems = [ "aarch64-linux" ];

  system = {
    vpn.ip = ip;
  };
}
