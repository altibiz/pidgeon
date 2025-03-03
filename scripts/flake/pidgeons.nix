{ lib, root, ... }:

{
  flake.lib.pidgeons =
    builtins.map
      (pidgeon: pidgeon // {
        wifi =
          if pidgeon ? wifi
          then pidgeon.wifi
          else pidgeon.id;
      })
      (builtins.fromJSON
        (lib.path.append
          root
          "assets/pidgeon/pidgeons.json"));
}
