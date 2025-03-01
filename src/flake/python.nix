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

    pyprojectOverrides = final: prev: {
      numpy = prev.numpy.overridePythonAttrs (old: {
        buildInputs = (old.buildInputs or [ ]) ++ (with pkgs; [
          libgcc
        ]);
      });

      pyright = prev.pyright.overridePythonAttrs (old: {
        postInstall = (old.postInstall or "") + ''
          wrapProgram $out/bin/pyright \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nodejs ]}
          wrapProgram $out/bin/pyright-langserver \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nodejs ]}
        '';
      });

      smbus = prev.smbus.overridePythonAttrs (old: {
        buildInputs = (old.buildInputs or [ ]) ++ (with prev; [
          setuptools
        ]);
      });
    };

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
