{
  lib,
  pkgs,
  homeManagerLib,
  package,
  app,
  devShell,
}:
let
  system = pkgs.stdenv.hostPlatform.system;
  productionCapabilities = package.productionCapabilities;
  startupContract = import ./startup-contract.nix;
  outputContract =
    assert lib.assertMsg (pkgs ? neomacs) "overlays.default must expose pkgs.neomacs";
    assert lib.assertMsg (
      pkgs.neomacs == package
    ) "packages.${system}.default must be built through overlays.default";
    assert lib.assertMsg (
      !pkgs.stdenv.isLinux || pkgs ? neomacs-wpewebkit
    ) "overlays.default must expose its pinned WPE package under a Neomacs-specific name";
    assert lib.assertMsg (
      package.type or null == "derivation"
    ) "packages.${system}.default must be a derivation";
    assert lib.assertMsg (app.type or null == "app") "apps.${system}.default must be an app";
    assert lib.assertMsg (
      devShell.type or null == "derivation"
    ) "devShells.${system}.default must be a derivation";
    assert lib.assertMsg (
      productionCapabilities ? cargoFeatures
    ) "packages.${system}.default must publish its Cargo capability set";
    assert lib.assertMsg (
      productionCapabilities ? videoBackend
    ) "packages.${system}.default must publish its video backend policy";
    pkgs.runCommand "neomacs-${system}-flake-output-contract" { } ''
      touch "$out"
    '';

  canRunPackage = pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform;

  packageContract =
    pkgs.runCommand "neomacs-${system}-installed-package-contract"
      {
        nativeBuildInputs = [
          pkgs.coreutils
          pkgs.gnugrep
        ];
      }
      ''
        test -x ${package}/bin/neomacs
        test -x ${package}/bin/neomacsclient
        test -L ${package}/bin/emacs
        test -L ${package}/bin/emacsclient
        test -d ${package}/share/neomacs/lisp
        test -d ${package}/share/neomacs/etc
        test -f ${package}/share/emacs/site-lisp/site-start.el
        test -f ${package}/share/emacs/site-lisp/subdirs.el
        test -d ${package}/share/applications
        test -d ${package}/share/icons
        test -d ${package}/share/info
        test -d ${package}/share/man
        ${lib.optionalString pkgs.stdenv.isLinux ''
          test -f ${package}/share/applications/neomacs.desktop
          test -f ${package}/share/icons/hicolor/scalable/apps/neomacs.svg
        ''}
        test -f ${package}/bin/neomacs.pdump
        ${lib.optionalString (productionCapabilities.videoBackend == "dynamic-gstreamer") ''
          test -x ${package}/bin/libneomacs_video_gstreamer.so
        ''}

        fingerprint="$(${package}/bin/neomacs --fingerprint | tr -d '[:space:]')"
        if ! [[ "$fingerprint" =~ ^[[:xdigit:]]{64}$ ]]; then
          echo "invalid installed Neomacs fingerprint: $fingerprint" >&2
          exit 1
        fi
        test -f "${package}/bin/neomacs-$fingerprint.pdump"

        ${startupContract {
          executable = "${package}/bin/neomacs";
          marker = "nix installed-package contract ok";
        }}

        touch "$out"
      '';
in
{
  flake-output-contract = outputContract;
}
// lib.optionalAttrs canRunPackage {
  installed-package-contract = packageContract;
  home-manager-contract = import ./home-manager.nix {
    inherit homeManagerLib pkgs package;
  };
}
