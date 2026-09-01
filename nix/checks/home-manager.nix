{
  homeManagerLib,
  pkgs,
  package,
}:
let
  configuration = homeManagerLib.homeManagerConfiguration {
    inherit pkgs;
    modules = [
      {
        home.username = "neomacs-nix-check";
        home.homeDirectory = "/tmp/neomacs-home-manager-contract";
        home.stateVersion = "24.11";
        # This rolling contract intentionally follows the flake's locked
        # nixpkgs while exercising Home Manager's current Emacs module.  Their
        # development version labels can differ between release branch points.
        home.enableNixpkgsReleaseCheck = false;

        programs.emacs = {
          enable = true;
          package = package;
        };

        # Keep this fixture focused on the package integration contract.
        manual.manpages.enable = false;
        news.display = "silent";
      }
    ];
  };
  finalPackage = configuration.config.programs.emacs.finalPackage;
in
pkgs.runCommand "neomacs-home-manager-contract" {
  nativeBuildInputs = [ pkgs.coreutils pkgs.gnugrep ];
} ''
  test -x ${configuration.activationPackage}/activate
  test -x ${finalPackage}/bin/emacs
  test -x ${finalPackage}/bin/emacsclient

  export HOME="$TMPDIR/clean-home"
  export XDG_CACHE_HOME="$HOME/.cache"
  export XDG_CONFIG_HOME="$HOME/.config"
  export XDG_DATA_HOME="$HOME/.local/share"
  mkdir -p "$HOME" "$XDG_CACHE_HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME"

  output="$(${finalPackage}/bin/emacs --batch --eval \
    '(progn (princ "home-manager neomacs contract ok\n") (kill-emacs 0))')"
  grep -Fqx "home-manager neomacs contract ok" <<<"$output"

  touch "$out"
''
