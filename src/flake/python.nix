{ self
, pyproject-nix
, pyproject-build-systems
, uv2nix
, lib
, ...
}:

let
  mkUv = pkgs: rec {
    workspace = uv2nix.lib.workspace.loadWorkspace {
      workspaceRoot = "${self}";
    };

    overlay = workspace.mkPyprojectOverlay {
      sourcePreference = "wheel";
    };

    pyprojectOverrides = final: prev: {
      numpy = prev.numpy.overrideAttrs (old: {
        buildInputs = (old.buildInputs or [ ]) ++ (with pkgs; [
          libgcc
        ]);
      });

      pyright = prev.pyright.overrideAttrs (old: {
        nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [
          pkgs.makeWrapper
        ];
        postInstall = (old.postInstall or "") + ''
          wrapProgram $out/bin/pyright \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nodejs ]}
          wrapProgram $out/bin/pyright-langserver \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nodejs ]}
        '';
      });

      smbus = prev.smbus.overrideAttrs (old: {
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
            src = lib.cleanSource old.src;

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
  flake.lib.python.mkApp = pkgs:
    let
      uv = mkUv pkgs;
      venv =
        uv.editablePythonSet.mkVirtualEnv
          "pidgeon-env"
          uv.workspace.deps.default;
    in
    {
      type = "app";
      program = "${venv}/bin/pidgeon-probe";
    };

  flake.lib.python.mkPackage = pkgs:
    let
      uv = mkUv pkgs;
    in
    uv.editablePythonSet.pidgeon-probe.override {
      pyprojectHook = uv.editablePythonSet.pyprojectDistHook;
    };

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
