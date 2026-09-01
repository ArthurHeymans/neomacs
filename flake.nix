{
  description = "Neomacs - GPU-accelerated Emacs written in Rust with a modern, multithreaded architecture";

  nixConfig = {
    extra-substituters = [
      "https://eval-exec.cachix.org"
      "https://nix-wpe-webkit.cachix.org"
    ];
    extra-trusted-public-keys = [
      "eval-exec.cachix.org-1:xvopUI7X7+Vt1gaSsWJ0PQFPP66vs8v5iIaz6boxf64="
      "nix-wpe-webkit.cachix.org-1:ItCjHkz1Y5QcwqI9cTGNWHzcox4EqcXqKvOygxpwYHE="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    # Rust toolchain
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
    };

    # Crane for incremental Rust builds (caches deps separately from source)
    crane.url = "github:ipetkov/crane";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # WPE WebKit standalone flake with Cachix binary cache
    # Do NOT use `inputs.nixpkgs.follows` here — the Cachix binary was built
    # with nix-wpe-webkit's own pinned nixpkgs, so follows would change the
    # derivation hash and cause a cache miss (rebuilding from source ~1 hour).
    nix-wpe-webkit = {
      url = "github:eval-exec/nix-wpe-webkit";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, crane, home-manager, nix-wpe-webkit }:
    let
      lib = nixpkgs.lib;

      supportedSystems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];

      forAllSystems = lib.genAttrs supportedSystems;

      workspaceManifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      productionCapabilityManifest =
        workspaceManifest.workspace.metadata.neomacs-production-capabilities;
      knownCargoCapabilities = [ "video" "webview" ];

      productionCapabilitiesFor = pkgs:
        let
          platform =
            if pkgs.stdenv.isLinux then "linux"
            else if pkgs.stdenv.isDarwin then "darwin"
            else throw "Neomacs has no production capability profile for ${pkgs.stdenv.hostPlatform.system}";
          profile = productionCapabilityManifest.${platform};
          cargoFeatures = profile.cargo-features;
          videoBackend = profile.video-backend;
          unknownFeatures = lib.subtractLists knownCargoCapabilities cargoFeatures;
        in
        assert lib.assertMsg (productionCapabilityManifest.schema-version == 1)
          "unsupported Neomacs production capability schema";
        assert lib.assertMsg (unknownFeatures == [ ])
          "unknown Neomacs production Cargo capabilities: ${lib.concatStringsSep ", " unknownFeatures}";
        assert lib.assertMsg (builtins.elem videoBackend [ "none" "dynamic-gstreamer" ])
          "unknown Neomacs production video backend: ${videoBackend}";
        assert lib.assertMsg (videoBackend != "dynamic-gstreamer" || builtins.elem "video" cargoFeatures)
          "dynamic-gstreamer requires the video Cargo capability";
        {
          inherit cargoFeatures videoBackend;
        };

      # Create pkgs with overlays for each system
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlays.default
          self.overlays.default
        ];
      };

      baseBuildInputsFor = pkgs: with pkgs; [
        ncurses
        gnutls
        zlib
        libxml2
        fontconfig
        freetype
        harfbuzz
        cairo
        pango
        glib
        libsoup_3
        glib-networking
        libjpeg
        libtiff
        giflib
        libpng
        librsvg
        libwebp
        poppler
        dbus
        sqlite
        tree-sitter
        gmp
      ] ++ lib.optionals pkgs.stdenv.isLinux (with pkgs; [
        # Rust dependencies may link C++ libraries even though Neomacs itself
        # is Rust. Keep libstdc++ in both the package and development runtime
        # closure so freshly linked bootstrap executables are runnable.
        stdenv.cc.cc.lib
        libotf
        alsa-lib
        libselinux
        libGL
        vulkan-loader
        libxkbcommon
        mesa
        libdrm
        libgbm
        wayland
        wayland-protocols
        libx11
        libxpm
        libxcursor
        libxrandr
        libxi
        libxinerama
      ]);

      videoBuildInputsFor = pkgs: with pkgs; [
        gst_all_1.gstreamer
        gst_all_1.gst-plugins-base
        gst_all_1.gst-plugins-good
        gst_all_1.gst-plugins-bad
        gst_all_1.gst-plugins-ugly
        gst_all_1.gst-libav
        gst_all_1.gst-plugins-rs
      ] ++ lib.optionals pkgs.stdenv.isLinux (with pkgs; [
        gst_all_1.gst-vaapi
        libva
      ]);

      webviewBuildInputsFor = pkgs:
        lib.optionals pkgs.stdenv.isLinux (with pkgs; [
          wpewebkit
          libwpe
          libwpe-fdo
          weston
          xdg-dbus-proxy
        ]);

      # Development exposes every optional native capability. Distribution
      # packages use the typed policy in Cargo.toml instead.
      commonBuildInputsFor = pkgs:
        baseBuildInputsFor pkgs
        ++ videoBuildInputsFor pkgs
        ++ webviewBuildInputsFor pkgs;

      productionBuildInputsFor = pkgs: capabilities:
        baseBuildInputsFor pkgs
        ++ lib.optionals (builtins.elem "video" capabilities.cargoFeatures)
          (videoBuildInputsFor pkgs)
        ++ lib.optionals (builtins.elem "webview" capabilities.cargoFeatures)
          (webviewBuildInputsFor pkgs);

      commonNativeBuildInputsFor = pkgs: [
        pkgs.rust-neomacs
        pkgs.rust-cbindgen
        pkgs.pkg-config
        pkgs.llvmPackages.clang
        pkgs.makeWrapper
      ] ++ lib.optionals pkgs.stdenv.isDarwin [
        # xtask's fresh-build pipeline re-signs role binaries after patching
        # the pdump fingerprint (xtask/src/main.rs:657). `codesign` isn't on
        # PATH in the Nix sandbox; sigtool provides a compatible shim.
        pkgs.darwin.sigtool
      ];

      mkNeomacsPackage = system:
        let
          pkgs = pkgsFor system;
          craneLib = (crane.mkLib pkgs).overrideToolchain pkgs.rust-neomacs;
          cargoSrc = craneLib.cleanCargoSource ./.;
          # The flake input is already a content-addressed, Git-filtered source
          # tree.  Refer to it directly: a second `builtins.path` copy can be
          # garbage-collected between evaluation and Crane reading Cargo.lock,
          # leaving an invalid transient store path during `nix flake check`.
          packageSrc = ./.;
          pname = "neomacs";
          version = self.shortRev or self.dirtyShortRev or self.lastModifiedDate or "0.0.1";
          productionCapabilities = productionCapabilitiesFor pkgs;
          buildsVideoBackend = productionCapabilities.videoBackend == "dynamic-gstreamer";
          cargoPackages = [ "-p" "neomacs" ]
            ++ lib.optionals buildsVideoBackend [ "-p" "neomacs-video-gstreamer" ];
          cargoFeatures = map (feature: "neomacs/${feature}")
            productionCapabilities.cargoFeatures;
          cargoFeatureArgs = lib.optionals (cargoFeatures != [ ]) [
            "--features"
            (lib.concatStringsSep "," cargoFeatures)
          ];
          cargoBuildArgs = lib.concatStringsSep " " (cargoPackages ++ cargoFeatureArgs);
          runtimeLibs = productionBuildInputsFor pkgs productionCapabilities;
          commonArgs = {
            inherit pname version;
            src = cargoSrc;
            strictDeps = true;
            cargoExtraArgs = cargoBuildArgs;
            nativeBuildInputs = commonNativeBuildInputsFor pkgs;
            buildInputs = runtimeLibs;
            doCheck = false;
          };
          depsArgs = commonArgs // {
            # Keep dependency artifacts stable across commits.  Let
            # `buildDepsOnly` synthesize its own dummy source: passing an
            # eagerly-created `mkDummySrc` back to Crane makes vendoring read a
            # derivation output during evaluation (IFD), which breaks cheap
            # evaluation of uncached foreign-system packages.
            version = "0.0.0";
          };
          cargoArtifacts = craneLib.buildDepsOnly depsArgs;
          hostEmulator = pkgs.stdenv.hostPlatform.emulator pkgs.buildPackages;
          fingerprintRunner = lib.optionalString (hostEmulator != null) "${hostEmulator} ";
          linuxWrapArgs = lib.optionals pkgs.stdenv.isLinux [
            "--set-default" "VK_DRIVER_FILES" "$(echo ${pkgs.mesa}/share/vulkan/icd.d/*.json | tr ' ' ':')"
          ] ++ lib.optionals (
            pkgs.stdenv.isLinux
            && builtins.elem "webview" productionCapabilities.cargoFeatures
          ) [
            "--set-default" "WPE_BACKEND_LIBRARY" "${pkgs.libwpe-fdo}/lib/libWPEBackend-fdo-1.0.so"
            "--set-default" "GIO_MODULE_DIR" "${pkgs.glib-networking}/lib/gio/modules"
            "--set-default" "WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS" "1"
            "--set-default" "WEBKIT_USE_SINGLE_WEB_PROCESS" "1"
            "--prefix" "PATH" ":" "${pkgs.wpewebkit}/libexec/wpe-webkit-2.0"
          ];
        in
        craneLib.buildPackage (commonArgs
          // {
            src = packageSrc;
            inherit cargoArtifacts;

            # After crane builds the Rust binaries, run the xtask bootstrap
            # pipeline (--skip-build reuses the binaries crane just built):
            # pbootstrap → COMPILE_FIRST → loaddefs → pdump
            postBuild = ''
              cargo xtask fresh-build --release --skip-build
            '';

            postInstall = ''
              mkdir -p "$out/share/neomacs"
              cp -r lisp "$out/share/neomacs/"
              cp -r etc "$out/share/neomacs/"
              chmod -R u+w "$out/share/neomacs"

              # GNU Emacs installs this version-independent site-lisp root.
              # Nixpkgs' `emacsPackagesFor` wrapper composes its generated
              # site-start with the wrapped editor's original file at this
              # exact path.  Neomacs' own runtime remains namespaced under
              # share/neomacs; these no-op files are the compatibility seam
              # required by Emacs package managers such as Home Manager.
              mkdir -p "$out/share/emacs/site-lisp"
              printf '%s\n' \
                ';;; site-start.el --- Nix Emacs package compatibility  -*- lexical-binding: t; -*-' \
                ';;; Commentary:' \
                ';; Neomacs runtime paths are configured by its executable wrapper.' \
                ';;; Code:' \
                ';;; site-start.el ends here' \
                > "$out/share/emacs/site-lisp/site-start.el"
              printf '%s\n' \
                ';;; subdirs.el --- Nix Emacs package compatibility  -*- lexical-binding: t; -*-' \
                > "$out/share/emacs/site-lisp/subdirs.el"

              # `emacsPackagesFor` preserves these standard Emacs share
              # directories when it constructs an Emacs-with-packages
              # wrapper.  Publish real desktop assets and valid empty
              # documentation roots so Home Manager can merge the result.
              mkdir -p "$out/share/info" "$out/share/man"
              ${lib.optionalString pkgs.stdenv.isLinux ''
                bash scripts/install-linux-desktop-assets.sh "$out"
              ''}

              final_pdump="target/release/neomacs.pdump"
              if [ ! -f "$final_pdump" ]; then
                echo "missing final pdump image: $final_pdump" >&2
                exit 1
              fi
              fingerprint="$(${fingerprintRunner}$out/bin/neomacs --fingerprint | tr -d '[:space:]')"
              if ! [[ "$fingerprint" =~ ^[[:xdigit:]]{64}$ ]]; then
                echo "invalid final pdump fingerprint: $fingerprint" >&2
                exit 1
              fi
              install -m 0644 "$final_pdump" "$out/bin/neomacs.pdump"
              install -m 0644 "$final_pdump" "$out/bin/neomacs-$fingerprint.pdump"

              ${lib.optionalString buildsVideoBackend ''
                video_backend="target/release/libneomacs_video_gstreamer.so"
                if [ ! -f "$video_backend" ]; then
                  echo "missing production video backend: $video_backend" >&2
                  exit 1
                fi
                install -m 0755 "$video_backend" "$out/bin/libneomacs_video_gstreamer.so"
              ''}

              ln -s neomacs "$out/bin/emacs"
              ln -s neomacsclient "$out/bin/emacsclient"

              wrapProgram "$out/bin/neomacs" \
                --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath runtimeLibs}" \
                --set-default RUST_LOG info \
                --set-default NEOMACS_RUNTIME_ROOT "$out/share/neomacs" \
                ${lib.concatStringsSep " \\\n                " linuxWrapArgs}
            '';

            passthru.productionCapabilities = productionCapabilities;

            meta = {
              description = "GPU-accelerated Emacs-compatible editor written in Rust";
              homepage = "https://github.com/eval-exec/neomacs";
              license = lib.licenses.gpl3Plus;
              mainProgram = "neomacs";
            };
          });

    in {
      # Overlay that provides wpewebkit (Linux only) and rust toolchain
      overlays.default = final: prev: {
        # Rust toolchain from rust-toolchain.toml (with extra extensions)
        rust-neomacs = (final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      } // (lib.optionalAttrs prev.stdenv.isLinux {
        # WPE WebKit from nix-wpe-webkit flake (with Cachix binary cache)
        # Only available on Linux — WPE WebKit does not support macOS.
        wpewebkit = nix-wpe-webkit.packages.${final.stdenv.hostPlatform.system}.wpewebkit;
      });

    # Development shell
      devShells = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          isLinux = pkgs.stdenv.isLinux;
          isDarwin = pkgs.stdenv.isDarwin;
          # Share one runtime-closure definition with the packaged wrapper.
          # ncurses remains RPATH-resolved because putting it in the shell's
          # global library path can contaminate the system shell\'s glibc.
          runtimeLibraryPath = pkgs.lib.makeLibraryPath (
            lib.remove pkgs.ncurses (commonBuildInputsFor pkgs)
          );
        in {
          default = pkgs.mkShell {
            name = "neomacs-dev";

            nativeBuildInputs = [
              # Rust toolchain
              pkgs.rust-neomacs
              pkgs.rust-cbindgen

              # Build tools
              pkgs.pkg-config

              # For bindgen (generates Rust bindings from C headers)
              pkgs.llvmPackages.clang

              # Frozen wall clock for date/time-sensitive oracle tests
              # (puts `faketime` on PATH; the .so path is exported below).
              pkgs.libfaketime
            ] ++ lib.optionals isLinux [
              # Record/replay debugger for the JIT wild-store hunt (reverse
              # watchpoints). Linux-only. Needs
              # `sysctl kernel.perf_event_paranoid=1` (or lower) at runtime.
              pkgs.rr
            ];

            buildInputs = commonBuildInputsFor pkgs
              ++ lib.optionals isLinux (with pkgs; [
                gcc
                xwininfo
              ]);

            # pkg-config paths for dev headers
            PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" (with pkgs; [
              glib.dev
              cairo.dev
              pango.dev
              gst_all_1.gstreamer.dev
              gst_all_1.gst-plugins-base.dev
              fontconfig.dev
              freetype.dev
              harfbuzz.dev
              libxml2.dev
              gnutls.dev
              zlib.dev
              ncurses.dev
              dbus.dev
              sqlite.dev
              tree-sitter
              gmp.dev
              libsoup_3.dev
              poppler.dev
            ]
            ++ lib.optionals isLinux [
              alsa-lib.dev
              libva
              libselinux.dev
              libGL.dev
              libxkbcommon.dev
              libdrm.dev
              mesa
              wayland.dev
              wpewebkit
              libwpe
              libwpe-fdo
            ]);

            # For bindgen to find libclang
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

            shellHook = ''
              export RUST_BACKTRACE=1

              # libfaketime shared object for frozen-clock oracle tests. Pinned
              # here so the resolver never has to guess the path from PATH.
              export NEOVM_LIBFAKETIME_SO="${pkgs.libfaketime}/lib/libfaketime.so.1"

              echo "=== Neomacs Development Environment ==="
              echo ""
              echo "Rust: $(rustc --version)"
              echo "Cargo: $(cargo --version)"
              echo "GStreamer: $(pkg-config --modversion gstreamer-1.0 2>/dev/null || echo 'not found')"
            ''
            # Linux-specific shell hook
            + lib.optionalString isLinux ''
              echo "xkbcommon: $(pkg-config --modversion xkbcommon 2>/dev/null || echo 'not found')"
              echo "WPE WebKit: $(pkg-config --modversion wpe-webkit-2.0 2>/dev/null || echo 'not found')"
              echo ""

              # Library path for runtime — DO NOT include ncurses here,
              # it causes glibc version contamination with system shell.
              # The linker adds RPATH for ncurses during compilation.
              export LD_LIBRARY_PATH="${runtimeLibraryPath}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

              # Vulkan ICD discovery — tell the Vulkan loader where Mesa's
              # driver JSON files are (e.g. intel_icd.x86_64.json for anv).
              # Without this, wgpu can't find Vulkan drivers and falls back to OpenGL.
              export VK_DRIVER_FILES="$(echo ${pkgs.mesa}/share/vulkan/icd.d/*.json | tr ' ' ':')"

              # WPE WebKit environment
              export WPE_BACKEND_LIBRARY="${pkgs.libwpe-fdo}/lib/libWPEBackend-fdo-1.0.so"
              export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
              export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
              export WEBKIT_USE_SINGLE_WEB_PROCESS=1
              export PATH="${pkgs.wpewebkit}/libexec/wpe-webkit-2.0:$PATH"

              # X11/Wayland display — preserve from parent env or detect from running session.
              # nix develop sanitizes env, so DISPLAY/XAUTHORITY may be lost.
              # Detect them from a running desktop session via /proc/<pid>/environ.
              _detect_display_env() {
                local _pid
                # NixOS wraps binaries, so process names may be e.g. ".kwin_x11-wrapp";
                # use substring match (no -x flag) to handle this.
                _pid=$(pgrep -u "$USER" kwin_x11 2>/dev/null | head -1)
                [ -z "$_pid" ] && _pid=$(pgrep -u "$USER" gnome-shell 2>/dev/null | head -1)
                [ -z "$_pid" ] && _pid=$(pgrep -u "$USER" Xorg 2>/dev/null | head -1)
                [ -z "$_pid" ] && _pid=$(pgrep -u "$USER" sway 2>/dev/null | head -1)
                [ -z "$_pid" ] && _pid=$(pgrep -u "$USER" Hyprland 2>/dev/null | head -1)
                if [ -n "$_pid" ] && [ -r "/proc/$_pid/environ" ]; then
                  if [ -z "$DISPLAY" ]; then
                    DISPLAY=$(tr '\0' '\n' < /proc/$_pid/environ | grep '^DISPLAY=' | head -1 | cut -d= -f2-)
                    [ -n "$DISPLAY" ] && export DISPLAY
                  fi
                  if [ -z "$XAUTHORITY" ] && [ -n "$DISPLAY" ]; then
                    XAUTHORITY=$(tr '\0' '\n' < /proc/$_pid/environ | grep '^XAUTHORITY=' | head -1 | cut -d= -f2-)
                    if [ -n "$XAUTHORITY" ] && [ -f "$XAUTHORITY" ]; then
                      export XAUTHORITY
                    elif [ -f "$HOME/.Xauthority" ]; then
                      export XAUTHORITY="$HOME/.Xauthority"
                    fi
                  fi
                  if [ -z "$WAYLAND_DISPLAY" ]; then
                    WAYLAND_DISPLAY=$(tr '\0' '\n' < /proc/$_pid/environ | grep '^WAYLAND_DISPLAY=' | head -1 | cut -d= -f2-)
                    [ -n "$WAYLAND_DISPLAY" ] && export WAYLAND_DISPLAY
                  fi
                fi
              }
              _detect_display_env
              unset -f _detect_display_env

              if [ -n "$DISPLAY" ]; then
                echo "Display: DISPLAY=$DISPLAY  XAUTHORITY=''${XAUTHORITY:-(unset)}"
                if ! timeout 2s ${pkgs.xdpyinfo}/bin/xdpyinfo >/dev/null 2>&1; then
                  export NEOMACS_X11_UNUSABLE=1
                  echo "Warning: X11 display handshake failed for DISPLAY=$DISPLAY."
                  echo "         GUI clients like winit/Neomacs may hang before the first window appears."
                  echo "         Run from a working desktop terminal, set a valid DISPLAY/XAUTHORITY,"
                  echo "         or use a private X server like Xvfb for automated probes."
                fi
              else
                echo "Display: (no X11/Wayland display detected)"
              fi
            ''
            # Darwin-specific shell hook
            + lib.optionalString isDarwin ''
              echo ""
              echo "Note: WPE WebKit is not available on macOS."
              echo "      WebKit-based features will be disabled."
            ''
            # Common shell hook (both platforms)
            + ''
              # Set default log levels (can be overridden before entering nix develop)
              export RUST_LOG="''${RUST_LOG:-debug}"

              echo ""
              echo "Build commands:"
              echo "  1. cargo xtask fresh-build --release"
              echo "  2. ./target/release/neomacs"
              echo ""
              echo "Logging (set before entering nix develop to override):"
              echo "  RUST_LOG=$RUST_LOG  (trace|debug|info|warn|error)"
              echo ""
            '';
          };
        }
      );

      packages = forAllSystems (system:
        let
          neomacs = mkNeomacsPackage system;
        in {
          default = neomacs;
          neomacs = neomacs;
        });

      apps = forAllSystems (system:
        let
          pkg = self.packages.${system}.default;
          neomacsApp = {
            type = "app";
            program = "${pkg}/bin/neomacs";
            meta = pkg.meta;
          };
        in {
          default = neomacsApp;
          neomacs = neomacsApp;
        });

      checks = forAllSystems (system:
        let
          pkgs = pkgsFor system;
        in
        import ./nix/checks {
          inherit lib pkgs;
          homeManagerLib = home-manager.lib;
          package = self.packages.${system}.default;
          app = self.apps.${system}.default;
          devShell = self.devShells.${system}.default;
        });
    };
}
