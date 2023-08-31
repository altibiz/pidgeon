{ pkgs, username, ... }:

let
  nix-recreate = pkgs.writeScriptBin "nix-recreate"
    ''
      #!${pkgs.stdenv.shell}
      set -eo pipefail

      command=switch
      comment="$1"
      if [[ "$1" == "boot" ]]; then
        command=boot
        comment="$2"
      fi
      if [[ -z "$comment" ]]; then
        comment="WIP"
      fi

      if [[ ! -d ~/repos/pidgeon ]]; then
       mkdir -p ~/repos
       git clone https://github.com/altibiz/pidgeon ~/repos/pidgeon
      fi

      wd="$(pwd)"
      cd ~/repos/pidgeon
      git add .
      git commit -m "$comment"
      git push
      sudo nixos-rebuild "$command" --flake ~/repos/pidgeon#pidgeon
      cd "$wd"
    '';

  nix-update = pkgs.writeScriptBin "nix-update"
    ''
      #!${pkgs.stdenv.shell}
      set -eo pipefail

      command=switch
      comment="$1"
      if [[ "$1" == "boot" ]]; then
        command=boot
        comment="$2"
      fi
      if [[ -z "$comment" ]]; then
        comment="WIP"
      fi

      if [[ ! -d ~/repos/pidgeon ]]; then
       mkdir -p ~/repos
       git clone https://github.com/altibiz/pidgeon ~/repos/pidgeon
      fi

      wd="$(pwd)"
      cd ~/repos/pidgeon
      nix flake update
      git add .
      git commit -m "$comment"
      git push
      sudo nixos-rebuild "$command" --flake ~/repos/pidgeon#pidgeon
      cd "$wd"
    '';

  nix-clean = pkgs.writeScriptBin "nix-clean"
    ''
      #!${pkgs.stdenv.shell}
      set -eo pipefail

      nix-env --delete-generations 7d
      nix-store --gc
    '';

  poetry-pylsp = pkgs.writeScriptBin "poetry-pylsp"
    ''
      #!${pkgs.stdenv.shell}
      set -eo pipefail

      "${pkgs.poetry}/bin/poetry" "$@"
    '';
in
{
  programs.home-manager.enable = true;
  xdg.configFile."nixpkgs/config.nix".source = ./assets/config.nix;

  xdg.configFile."pidgeon/config.yaml".source = ./assets/pidgeon.yaml;

  home.username = "${username}";
  home.homeDirectory = "/home/${username}";
  home.sessionVariables = {
    VISUAL = "hx";
    EDITOR = "hx";
    PAGER = "bat";
  };
  home.shellAliases = {
    lg = "lazygit";
    cat = "bat";
    grep = "rg";
    rm = "rm -i";
    mv = "mv -i";
    la = "exa";

    pls = "sudo";
    bruh = "git";
    sis = "hx";
    yas = "yes";
  };
  home.packages = with pkgs; [
    # dev
    meld
    nil
    nixpkgs-fmt
    python310
    (poetry.override { python3 = python310; })
    poetry-pylsp
    python310Packages.python-lsp-server
    ruff
    python310Packages.python-lsp-ruff
    python310Packages.pylsp-rope
    python310Packages.yapf
    llvmPackages.clangNoLibcxx
    llvmPackages.lldb
    rustc
    cargo
    clippy
    rustfmt
    rust-analyzer
    nodePackages.bash-language-server
    nodePackages.yaml-language-server
    taplo
    marksman

    # tui
    direnv
    nix-direnv
    pciutils
    lsof
    dmidecode
    inxi
    hwinfo
    ncdu
    file
    fd
    duf
    unzip
    unrar
    sd
    tshark
    sqlx-cli

    # scripts
    nix-recreate
    nix-update
    nix-clean
  ];

  # dev
  programs.git.enable = true;
  programs.git.delta.enable = true;
  programs.git.attributes = [ "* text=auto eof=lf" ];
  programs.git.lfs.enable = true;
  programs.git.extraConfig = {
    interactive.singleKey = true;
    init.defaultBranch = "main";
    pull.rebase = true;
    push.default = "upstream";
    push.followTags = true;
    rerere.enabled = true;
    merge.tool = "meld";
    "mergetool \"meld\"".cmd = ''meld "$LOCAL" "$MERGED" "$REMOTE" --output "$MERGED"'';
    color.ui = "auto";
  };
  programs.helix.enable = true;
  programs.helix.languages = {
    language = [
      {
        name = "python";
        auto-format = true;
        formatter = { command = "yapf"; };
        config.pylsp.plugins = {
          rope = { enabled = true; };
          ruff = { enabled = true; };
          flake8 = { enabled = false; };
          pylint = { enabled = false; };
          pycodestyle = { enabled = false; };
          pyflakes = { enabled = false; };
          mccabe = { enabled = false; };
          yapf = { enabled = true; };
          autopep8 = { enabled = false; };
        };
      }
      {
        name = "nix";
        auto-format = true;
        formatter = { command = "nixpkgs-fmt"; };
      }
    ];
  };
  programs.helix.settings = {
    theme = "transparent";
    editor = {
      true-color = true;
      scrolloff = 999;
      auto-save = true;
      rulers = [ ];
      gutters = [ "diagnostics" "spacer" "diff" ];
    };
  };
  programs.helix.themes.transparent = {
    inherits = "everforest_dark";

    "ui.background" = { };
    "ui.statusline" = { fg = "fg"; };
  };

  # tui
  programs.direnv.enable = true;
  programs.direnv.enableNushellIntegration = true;
  programs.direnv.nix-direnv.enable = true;
  programs.nushell.enable = true;
  programs.nushell.extraEnv = ''
    def-env append-path [new_path: string] {
      let updated_env_path = (
        if ($env.PATH | split row ":" | any { |it| $it == $new_path }) {
          $env.PATH
        }
        else {
          $"($env.PATH):($new_path)"
        }
      )
      $env.PATH = $updated_env_path
    }
    def-env prepend-path [new_path: string] {
      let updated_env_path = (
        if ($env.PATH | split row ":" | any { |it| $it == $new_path }) {
          $env.PATH
        }
        else {
          $"($new_path):($env.PATH)"
        }
      )
      $env.PATH = $updated_env_path
    }

    prepend-path "/home/${username}/scripts"
    prepend-path "/home/${username}/bin"
    prepend-path "scripts"
    prepend-path "bin"
  '';
  programs.nushell.extraConfig = ''
    $env.config = {
      show_banner: false

      edit_mode: vi
      cursor_shape: {
        vi_insert: line
        vi_normal: underscore
      }

      hooks: {
        pre_prompt: [{ ||
          let direnv = (direnv export json | from json)
          let direnv = if ($direnv | length) == 1 { $direnv } else { {} }
          $direnv | load-env
        }]
      }
    }
  '';
  programs.nushell.environmentVariables = {
    PROMPT_INDICATOR_VI_INSERT = "'λ '";
    PROMPT_INDICATOR_VI_NORMAL = "' '";
  };
  programs.starship.enable = true;
  programs.starship.enableNushellIntegration = true;
  xdg.configFile."starship.toml".source = ./assets/starship.toml;
  programs.zoxide.enable = true;
  programs.zoxide.enableNushellIntegration = true;
  programs.lazygit.enable = true;
  programs.lazygit.settings = {
    notARepository = "quit";
    promptToReturnFromSubprocess = false;
    gui = {
      showIcons = true;
    };
  };
  programs.htop.enable = true;
  programs.lf.enable = true;
  programs.bat.enable = true;
  programs.bat.config = { style = "header,rule,snip,changes"; };
  programs.ripgrep.enable = true;
  programs.ripgrep.arguments = [
    "--max-columns=100"
    "--max-columns-preview"
    "--colors=auto"
    "--smart-case"
  ];
  programs.exa.enable = true;
  programs.exa.extraOptions = [
    "--all"
    "--list"
    "--color=always"
    "--group-directories-first"
    "--icons"
    "--group"
    "--header"
  ];

  # services
  programs.gpg.enable = true;
  services.gpg-agent.enable = true;
  services.gpg-agent.pinentryFlavor = "tty";
  programs.ssh.enable = true;
  programs.ssh.matchBlocks = {
    "github.com" = {
      user = "git";
    };
  };

  home.stateVersion = "23.11";
}
