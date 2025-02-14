let
  ip = "10.8.0.13";
in
{
  hostname = ip;
  users = [ "altibiz" ];
  systems = [ "aarch64-linux" ];

  system = {
    pidgeon.vpn.ip = ip;
  };
}
