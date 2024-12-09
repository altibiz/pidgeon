{ self, poetry2nix, ... }:

# TODO: per package options

let
  mkPoetry2nixLib = pkgs: poetry2nix.lib.mkPoetry2Nix { inherit pkgs; };

  common = pkgs: {
    projectDir = "${self}/src/probe";
    preferWheels = true;
    checkGroups = [ ];
    python = pkgs.python311;
    overrides = (mkPoetry2nixLib pkgs).defaultPoetryOverrides.extend (final: prev: {
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
    });
  };

  mkEnv = pkgs: (mkPoetry2nixLib pkgs).mkPoetryEnv (common pkgs);

  mkApp = pkgs: (mkPoetry2nixLib pkgs).mkPoetryApplication (common pkgs);
in
{
  inherit mkEnv mkApp;

  mkEnvWrapper = pkgs: bin: pkgs.writeShellApplication (
    let
      env = mkEnv pkgs;
    in
    {
      name = bin;
      runtimeInputs = [ env ];
      text = ''
        export PYTHONPREFIX=${env}
        export PYTHONEXECUTABLE=${env}/bin/python

        # shellcheck disable=SC2125
        export PYTHONPATH=${env}/lib/**/site-packages

        ${bin} "$@"
      '';
    }
  );
}
