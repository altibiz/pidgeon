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

    python = pkgs.python311;

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

    editableOverlay = workspace.mkEditablePyprojectOverlay {
      root = "$REPO_ROOT";
      members = [ "pidgeon-probe" ];
    };

    editablePythonSet = pythonSet.overrideScope (
      lib.composeManyExtensions [
        editableOverlay

        (final: prev: {
          pidgeon-probe = prev.pidgeon-probe.overrideAttrs (old: {
            src = lib.fileset.toSource {
              root = old.src;
              fileset = lib.fileset.unions [
                (old.src + "/pyproject.toml")
                (old.src + "/README.md")
                (old.src + "/src/probe/pyproject.toml")
                (old.src + "/src/probe/README.md")
                (old.src + "/src/probe/src/**/*.py")
              ];
            };

            # NOTE: hatchling requirement
            nativeBuildInputs =
              old.nativeBuildInputs
              ++ final.resolveBuildSystem {
                editables = [ ];
              };
          });

        })
      ]
    );
  };
in
{
  flake.lib.python.mkPackage = pkgs:
    let
      uv = mkUv pkgs;

      venv =
        uv.pythonSet.mkVirtualEnv
          "pidgeon-env"
          uv.workspace.deps.default;
    in
    venv."pidgeon-probe";

  flake.lib.python.mkDevShell = pkgs:
    let
      uv = mkUv pkgs;

      venv =
        uv.editablePythonSet.mkVirtualEnv
          "pidgeon-dev-env"
          uv.workspace.deps.all;
    in
    pkgs.mkShell {
      packages = [
        venv
        pkgs.uv
        pkgs.git
      ];

      env = {
        UV_NO_SYNC = "1";
        UV_PYTHON = "${venv}/bin/python";
        UV_PYTHON_DOWNLOADS = "never";
      };

      shellHook = ''
        unset PYTHONPATH
        export REPO_ROOT=$(git rev-parse --show-toplevel)
      '';
    };
}
