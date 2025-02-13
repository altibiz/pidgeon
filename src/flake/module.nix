{ ... }:

let
  mkDotObject = specialArgs: pidgeonModule:
    if builtins.isFunction pidgeonModule
    then (pidgeonModule specialArgs)
    else pidgeonModule;

  mkImports = mkModule: specialArgs: pidgeonObject: builtins.map
    (maybeImport:
      if (builtins.isPath maybeImport) || (builtins.isString maybeImport)
      then
        let
          module = (mkModule (import maybeImport) specialArgs);
        in
        if builtins.isAttrs module
        then module // { _file = maybeImport; }
        else module
      else mkModule maybeImport specialArgs)
    (if builtins.hasAttr "imports" pidgeonObject
    then pidgeonObject.imports
    else [ ]);

  mkOptions = specialArgs: pidgeonObject:
    if builtins.hasAttr "disabled" pidgeonObject
    then { }
    else if builtins.hasAttr "options" pidgeonObject
    then pidgeonObject.options
    else { };

  mkConfig = { lib, ... }: path: pidgeonObject:
    if builtins.hasAttr "disabled" pidgeonObject
    then { }
    else if builtins.hasAttr "config" pidgeonObject
    then
      let
        configObject = pidgeonObject.config;
      in
      if lib.hasAttrByPath path configObject
      then lib.getAttrFromPath path configObject
      else { }
    else
      if lib.hasAttrByPath path pidgeonObject
      then lib.getAttrFromPath path pidgeonObject
      else { };

  # NOTE: if pkgs here not demanded other modules don't get access...
  mkSystemModule = mkSystemModule: pidgeonModule: { pkgs, ... } @specialArgs:
    let
      pidgeonObject = mkDotObject specialArgs pidgeonModule;
      imports = mkImports mkSystemModule specialArgs pidgeonObject;
      options = mkOptions specialArgs pidgeonObject;
      config = mkConfig specialArgs [ "system" ] pidgeonObject;
      sharedConfig = mkConfig specialArgs [ "shared" ] pidgeonObject;
    in
    {
      imports = imports ++ [ sharedConfig ];
      inherit options config;
    };

  # NOTE: if pkgs here not demanded other modules don't get access...
  mkHomeModule = mkHomeModule: pidgeonModule: { pkgs, ... } @specialArgs:
    let
      pidgeonObject = mkDotObject specialArgs pidgeonModule;
      imports = mkImports mkHomeModule specialArgs pidgeonObject;
      options = mkOptions specialArgs pidgeonObject;
      config = mkConfig specialArgs [ "home" ] pidgeonObject;
      sharedConfig = mkConfig specialArgs [ "shared" ] pidgeonObject;
    in
    {
      imports = imports ++ [ sharedConfig ];
      inherit options config;
    };
in
{
  mkSystemModule = mkSystemModule mkSystemModule;
  mkHomeModule = mkHomeModule mkHomeModule;
}
