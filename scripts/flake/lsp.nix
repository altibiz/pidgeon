{
  flake.lib.lsp.mkDevShell = pkgs: pkgs.mkShell {
    packages = with pkgs; [
      nodePackages.yaml-language-server
      marksman
      taplo
      nodePackages.bash-language-server
      vscode-langservers-extracted
      nil
      cargo
      rustc
      lldb
    ];
  };
}
