{ pyproject-nix
, pyproject-build-systems
, uv2nix
, lib
, ...
}:

let
  mkUv = pkgs: rec {
    workspace = uv2nix.lib.workspace.loadWorkspace {
      workspaceRoot = ./.;
    };

    overlay = workspace.mkPyprojectOverlay {
      sourcePreference = "wheel";
    };

    pyprojectOverrides = _final: _prev: { };

    python = pkgs.python312;

    pythonSet =
      (pkgs.callPackage pyproject-nix.build.packages {
        inherit python;
      }).overrideScope
        (
          lib.composeManyExtensions [
            pyproject-build-systems.overlays.default
            overlay
            pyprojectOverrides
          ]
        );
  };
in
{
  flake.lib.python.env = pkgs:
    let
      uv = mkUv pkgs;
    in
    uv.pythonSet.mkVirtualEnv
      "hello-world-env"
      uv.workspace.deps.default;

  flake.lib.python.devShell = pkgs:
    let
      uv = mkUv pkgs;
    in
    uv.pythonSet.mkVirtualEnv
      "hello-world-env"
      uv.workspace.deps.default;
}
