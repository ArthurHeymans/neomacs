# Building NEO Emacs from Source

Prebuilt binaries for Linux, macOS, and Windows are available on the
[releases page](https://github.com/eval-exec/neomacs/releases) — building from source
is only needed for development or unsupported platforms.

## Prerequisites

- **Rust** (stable, pinned via `rust-toolchain.toml` — rustup installs it automatically)
- **GStreamer** (optional, for video playback)
- **WPE WebKit** (optional, for inline browser, Linux only)
- **VA-API** (optional, for hardware video decode on Linux)
- **GNU Emacs** (optional, for pre-compiling .el files — speeds up bootstrap ~17x)

Build commands in this document are run from the repository root.

## Quick Start

```bash
# Optional (recommended): use the repo dev shell (handles all dependencies)
nix develop --accept-flake-config

# Build NEO Emacs (compiles Rust, bootstraps Elisp, generates pdump)
cargo xtask fresh-build --release

# Run
./target/release/neomacs
```

## Testing

After a release fresh build, run the main parity suites with:

```bash
cargo nextest run -p neovm-core --no-fail-fast
cargo nextest run -p neovm-oracle-tests --no-fail-fast
cargo nextest run -p neomacs-tui-tests --release --no-fail-fast
```

The TUI harness uses `target/release/neomacs` by default, regardless of the
Cargo test profile. Set `NEOMACS_TUI_NEOMACS_BIN` to use a different binary.

## Linux (Arch Linux)

```bash
# Install dependencies
sudo pacman -S --needed \
  base-devel autoconf automake texinfo clang git pkg-config \
  gtk4 glib2 cairo \
  gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad \
  wpewebkit wpebackend-fdo \
  wayland wayland-protocols \
  mesa libva \
  libjpeg-turbo libtiff giflib libpng librsvg libwebp \
  ncurses gnutls libxml2 sqlite jansson tree-sitter \
  gmp acl libxpm \
  libgccjit

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build NEO Emacs (compiles Rust, bootstraps Elisp, generates pdump)
cargo xtask fresh-build --release

# Run
./target/release/neomacs
```

Other distributions should follow similar dependency installation with their
package manager.

## macOS (Experimental)

macOS support is experimental — see
[issue #22](https://github.com/eval-exec/neomacs/issues/22) for status.
Maintainers should use the reproducible signing, notarization, and artifact
verification flow in [releasing-macos.md](releasing-macos.md) rather than
uploading a locally assembled app bundle.

```bash
# Install dependencies (Homebrew)
brew install pkgconf \
  glib cairo \
  gstreamer gst-plugins-base gst-plugins-good \
  jpeg-turbo libtiff giflib libpng librsvg webp \
  gnutls libxml2 sqlite jansson tree-sitter gmp

# gmp-mpfr-sys is built with system GMP support. Its build script probes
# GMP with the C compiler directly, so Homebrew's keg must be visible to
# both the C compiler and linker.
export CPATH="$(brew --prefix gmp)/include${CPATH:+:$CPATH}"
export LIBRARY_PATH="$(brew --prefix gmp)/lib${LIBRARY_PATH:+:$LIBRARY_PATH}"

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build NEO Emacs
cargo xtask fresh-build --release

# Run
./target/release/neomacs
```

## NixOS / Nix

NEO Emacs uses [nix-wpe-webkit](https://github.com/eval-exec/nix-wpe-webkit) for the
WPE WebKit dependency. Pre-built binaries are available via Cachix (~60MB download
instead of ~1 hour build).

The `flake.nix` includes `nixConfig` for the Cachix cache. Pass
`--accept-flake-config` to use it automatically, or configure it system-wide:

**NixOS** — add to your configuration (e.g., `/etc/nixos/configuration.nix`):

```nix
{
  nix.settings.substituters = [ "https://nix-wpe-webkit.cachix.org" ];
  nix.settings.trusted-public-keys = [ "nix-wpe-webkit.cachix.org-1:ItCjHkz1Y5QcwqI9cTGNWHzcox4EqcXqKvOygxpwYHE=" ];
}
```

**Non-NixOS** — add to `~/.config/nix/nix.conf`:

```
extra-substituters = https://nix-wpe-webkit.cachix.org
extra-trusted-public-keys = nix-wpe-webkit.cachix.org-1:ItCjHkz1Y5QcwqI9cTGNWHzcox4EqcXqKvOygxpwYHE=
```

### Build with Nix

**Option 1** — Trust the `nixConfig` in `flake.nix` (simplest):

```bash
nix build --accept-flake-config

# Or enter development shell
nix develop --accept-flake-config
```

**Option 2** — Pass Cachix flags directly:

```bash
nix build \
  --extra-substituters "https://nix-wpe-webkit.cachix.org" \
  --extra-trusted-public-keys "nix-wpe-webkit.cachix.org-1:ItCjHkz1Y5QcwqI9cTGNWHzcox4EqcXqKvOygxpwYHE="
```

> **Note:** Both options require your user to be in `trusted-users` in
> `/etc/nix/nix.conf` (e.g., `trusted-users = root @wheel your-username`), or
> configure the cache system-wide as shown above.

### Manual build (inside dev shell)

```bash
cargo xtask fresh-build --release
```
