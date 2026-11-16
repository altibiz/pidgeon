{
  self,
  root,
  pyproject-nix,
  pyproject-build-systems,
  uv2nix,
  lib,
  ...
}:

let
  mkUvLib = pkgs: rec {
    workspace = uv2nix.lib.workspace.loadWorkspace {
      workspaceRoot = "${self}";
    };

    overlay = workspace.mkPyprojectOverlay {
      sourcePreference = "wheel";
    };

    pyprojectOverrides = final: prev: {
      numpy = prev.numpy.overrideAttrs (old: {
        buildInputs =
          (old.buildInputs or [ ])
          ++ (with pkgs; [
            libgcc
          ]);
      });

      smbus = prev.smbus.overrideAttrs (old: {
        buildInputs =
          (old.buildInputs or [ ])
          ++ (with prev; [
            setuptools
          ]);
      });
    };

    python = pkgs.python311;

    buildUtil = pkgs.callPackages pyproject-nix.build.util { };

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

    editableMembers = [
      "pidgeon-probe"
      "pidgeon-grafana"
    ];

    editableOverlay = workspace.mkEditablePyprojectOverlay {
      root = "$REPO_ROOT";
      members = editableMembers;
    };

    editablePythonSet = pythonSet.overrideScope (
      lib.composeManyExtensions [
        editableOverlay
        (
          final: prev:
          lib.genAttrs editableMembers (
            name:
            prev.${name}.overrideAttrs (old: {
              src = lib.cleanSource old.src;

              # NOTE: hatchling requirement
              nativeBuildInputs =
                old.nativeBuildInputs
                ++ final.resolveBuildSystem {
                  editables = [ ];
                };
            })
          )
        )
      ]
    );
  };
in
{
  flake.lib.python.mkGrafanaDashboards =
    pkgs:
    let
      src = lib.fileset.toSource {
        root = lib.path.append root "scripts/grafana/src";
        fileset = lib.path.append root "scripts/grafana/src";
      };
    in
    pkgs.runCommand "pidgeon-grafana-dashboards"
      {
        nativeBuildInputs = [ pkgs.python311 ];
      }
      ''
        PYTHONPATH=${src} python -m pidgeon_grafana --out $out
      '';

  flake.lib.python.mkPackage =
    pkgs:
    let
      uv = mkUvLib pkgs;

      venv = uv.pythonSet.mkVirtualEnv "pidgeon-env" uv.workspace.deps.default;
    in
    uv.buildUtil.mkApplication {
      venv = venv;
      package = uv.pythonSet.pidgeon-probe;
    };

  flake.lib.python.mkDevShell =
    pkgs:
    let
      uv = mkUvLib pkgs;

      venv = uv.editablePythonSet.mkVirtualEnv "pidgeon-dev-env" uv.workspace.deps.all;
    in
    pkgs.mkShell {
      packages = [
        (pkgs.writeShellApplication {
          name = "pyright";
          runtimeInputs = [
            pkgs.nodejs
            venv
          ];
          text = ''
            export PYTHONPREFIX=${venv}
            export PYTHONEXECUTABLE=${venv}/bin/python
            # shellcheck disable=SC2125
            export PYTHONPATH=${venv}/lib/**/site-packages

            pyright "$@"
          '';
        })
        (pkgs.writeShellApplication {
          name = "pyright-langserver";
          runtimeInputs = [
            pkgs.nodejs
            venv
          ];
          text = ''
            export PYTHONPREFIX=${venv}
            export PYTHONEXECUTABLE=${venv}/bin/python
            # shellcheck disable=SC2125
            export PYTHONPATH=${venv}/lib/**/site-packages

            pyright-langserver "$@"
          '';
        })
        venv
        pkgs.uv
        pkgs.git
      ];

      shellHook = ''
        export UV_NO_SYNC="1"
        export UV_PYTHON="${venv}/bin/python"
        export UV_PYTHON_DOWNLOADS="never"

        unset PYTHONPATH
        export REPO_ROOT=$(git rev-parse --show-toplevel)
      '';
    };
}
