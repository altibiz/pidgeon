{ ... }:

(final: prev: {
  nodejs = prev.nodejs_20;

  # NOTE: https://github.com/NixOS/nixpkgs/issues/154163#issuecomment-1008362877  
  makeModulesClosure = x: prev.makeModulesClosure
    (x // { allowMissing = true; });
})
