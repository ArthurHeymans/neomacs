#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package-deb.sh [--target TRIPLE] [--skip-build] [--no-smoke]

Build and package a .deb for NEO Emacs.

Options:
  --target TRIPLE Rust target triple. Defaults to host.
  --skip-build    Reuse existing target/release artifacts.
  --no-smoke      Do not smoke-test the binary.

Output:
  dist/neomacs_{version}_{arch}.deb
USAGE
}

get_version() {
  local v
  v="$(git describe --tags --abbrev=0 2>/dev/null)" && echo "${v#v}" && return
  v="$(git rev-parse --short=12 HEAD 2>/dev/null)" && echo "$v" && return
  echo "0.0.0-dev"
}

target_triple="x86_64-unknown-linux-gnu"
skip_build=0
smoke=1

while (($#)); do
  case "$1" in
    --target)
      target_triple="${2:?--target requires a value}"
      shift 2
      ;;
    --skip-build)
      skip_build=1
      shift
      ;;
    --no-smoke)
      smoke=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

arch_from_triple() {
  case "$1" in
    x86_64-*)  echo "amd64" ;;
    aarch64-*) echo "arm64" ;;
    armv7-*)   echo "armhf" ;;
    *)         echo "unknown" ;;
  esac
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

dist_dir="$repo_root/dist"
version="$(get_version)"
deb_arch="$(arch_from_triple "$target_triple")"
deb_name="neomacs_${version}_${deb_arch}.deb"
deb_path="$dist_dir/$deb_name"
pkg_dir="$dist_dir/deb-staging"

pkg_args=(--target "$target_triple")
if ((skip_build)); then
  pkg_args+=(--skip-build)
fi
pkg_args+=(--no-smoke)

scripts/package-release.sh "${pkg_args[@]}"

release_tree="$dist_dir/neomacs-${version}-${target_triple}"
if [[ ! -d "$release_tree" ]]; then
  echo "release tree not found: $release_tree" >&2
  exit 1
fi

echo "building .deb package..."

rm -rf "$pkg_dir"
mkdir -p "$pkg_dir/DEBIAN"
mkdir -p "$pkg_dir/usr/bin"
mkdir -p "$pkg_dir/usr/share/neomacs"
mkdir -p "$pkg_dir/usr/share/doc/neomacs"

install -m 0755 "$release_tree/bin/neomacs" "$pkg_dir/usr/bin/neomacs"
for bin in neomacsclient neomacs-temacs bootstrap-neomacs mock-display; do
  if [[ -x "$release_tree/bin/$bin" ]]; then
    install -m 0755 "$release_tree/bin/$bin" "$pkg_dir/usr/bin/$bin"
  fi
done

install -m 0644 "$release_tree/bin/neomacs.pdump" "$pkg_dir/usr/bin/neomacs.pdump"

cp -a "$release_tree/share/neomacs/." "$pkg_dir/usr/share/neomacs/"

install -m 0644 README.md "$pkg_dir/usr/share/doc/neomacs/README.md"
install -m 0644 COPYING "$pkg_dir/usr/share/doc/neomacs/copyright"

scripts/install-linux-desktop-assets.sh "$pkg_dir/usr"

installed_size="$(du -sk "$pkg_dir" | cut -f1)"

cat >"$pkg_dir/DEBIAN/control" <<CONTROL
Package: neomacs
Version: ${version}
Section: editors
Priority: optional
Architecture: ${deb_arch}
Installed-Size: ${installed_size}
Maintainer: eval-exec <noreply@github.com>
Homepage: https://github.com/eval-exec/neomacs
Description: NEO Emacs
 Extensible, programmable text editor based on Emacs Lisp
 and the Neovim virtual machine, built with Rust.
Depends: libcairo2, libfontconfig1, libglib2.0-0, libpango-1.0-0
CONTROL

cat >"$pkg_dir/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q /usr/share/icons/hicolor 2>/dev/null || true
fi
POSTINST
chmod 0755 "$pkg_dir/DEBIAN/postinst"

cat >"$pkg_dir/DEBIAN/postrm" <<'POSTRM'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q /usr/share/icons/hicolor 2>/dev/null || true
fi
POSTRM
chmod 0755 "$pkg_dir/DEBIAN/postrm"

dpkg-deb --build "$pkg_dir" "$deb_path"
rm -rf "$pkg_dir"

if ((smoke)); then
  echo "smoke-testing binary..."
  NEOMACS_RUNTIME_ROOT="$release_tree/share/neomacs" \
    timeout 30s "$release_tree/bin/neomacs" --batch --eval "(kill-emacs 0)"
fi

echo "wrote $deb_path"
