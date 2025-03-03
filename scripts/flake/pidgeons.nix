{ lib, root, ... }:

{
  flake.lib.pidgeons =
    builtins.fromJSON
      (lib.path.append
        root
        "assets/pidgeon/pidgeons.json");
}
