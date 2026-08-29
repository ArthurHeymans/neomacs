#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package-release.sh [--target TRIPLE] [--skip-build] [--no-smoke]

Build and package a Neomacs binary release archive.

Options:
  --target TRIPLE Rust target triple, e.g. x86_64-unknown-linux-gnu.
                  Defaults to x86_64-unknown-linux-gnu on Linux,
                  aarch64-apple-darwin on macOS, x86_64-pc-windows-msvc or
                  aarch64-pc-windows-msvc on Windows.
  --skip-build    Package existing target/release artifacts without running
                  cargo xtask fresh-build --release.
  --no-smoke      Do not smoke-test the extracted archive.

Output:
  dist/neomacs-{version}-{target}.tar.gz
USAGE
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux*)   echo "${arch}-unknown-linux-gnu" ;;
    Darwin*)  echo "${arch}-apple-darwin" ;;
    MINGW*|MSYS*|CYGWIN*) echo "${arch}-pc-windows-msvc" ;;
    *)        echo "${arch}-unknown-$(echo "$os" | tr '[:upper:]' '[:lower:]')" ;;
  esac
}

get_version() {
  local v
  v="$(git describe --tags --abbrev=0 2>/dev/null)" && echo "${v#v}" && return
  v="$(git rev-parse --short=12 HEAD 2>/dev/null)" && echo "$v" && return
  echo "0.0.0-dev"
}

binary_ext_for_target() {
  case "$1" in
    *-windows-*) echo ".exe" ;;
    *) echo "" ;;
  esac
}

install_binary_if_present() {
  local name="$1"
  local ext="$2"
  local source="$release_dir/$name$ext"
  local dest="$package_dir/bin/$name$ext"
  if [[ -f "$source" ]]; then
    install -m 0755 "$source" "$dest"
  fi
}

target_triple="$(detect_target)"
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

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ((skip_build == 0)); then
  cargo xtask fresh-build --release
fi

release_dir="$repo_root/target/release"
dist_dir="$repo_root/dist"
version="$(get_version)"
package_name="neomacs-${version}-${target_triple}"
package_dir="$dist_dir/$package_name"
archive="$dist_dir/$package_name.tar.gz"
binary_ext="$(binary_ext_for_target "$target_triple")"

for required in \
  "$release_dir/neomacs$binary_ext" \
  "$release_dir/neomacsclient$binary_ext" \
  "$release_dir/neomacs.pdump"
do
  if [[ ! -f "$required" ]]; then
    echo "missing required release artifact: $required" >&2
    echo "run cargo xtask fresh-build --release first, or omit --skip-build" >&2
    exit 1
  fi
done

rm -rf "$package_dir" "$archive"
mkdir -p "$package_dir/bin" "$package_dir/share/neomacs"

for binary in neomacs neomacsclient neomacs-temacs bootstrap-neomacs mock-display; do
  install_binary_if_present "$binary" "$binary_ext"
done

install -m 0644 "$release_dir/neomacs.pdump" "$package_dir/bin/neomacs.pdump"

cp -a lisp "$package_dir/share/neomacs/"
cp -a etc "$package_dir/share/neomacs/"
cp -a leim "$package_dir/share/neomacs/"
cp -a info "$package_dir/share/neomacs/" 2>/dev/null || true

if [[ "$target_triple" == *-linux-* ]]; then
  scripts/install-linux-desktop-assets.sh "$package_dir"
fi

install -m 0644 README.md "$package_dir/README.md"
install -m 0644 COPYING "$package_dir/COPYING"

cat >"$package_dir/VERSION" <<VERSION
name: neomacs
target: $target_triple
git: $(git rev-parse --short=12 HEAD 2>/dev/null || echo unknown)
built: $(date -u +%Y-%m-%dT%H:%M:%SZ)
VERSION

if ((smoke)); then
  smoke_base="${TMPDIR:-$repo_root/tmp}"
  mkdir -p "$smoke_base"
  smoke_dir="$(mktemp -d "$smoke_base/neomacs-release-smoke.XXXXXX")"
  trap 'rm -rf "$smoke_dir"' EXIT
  tar -C "$dist_dir" -czf "$archive" "$package_name"
  tar -C "$smoke_dir" -xzf "$archive"
  NEOMACS_RUNTIME_ROOT="$smoke_dir/$package_name/share/neomacs" \
    timeout 30s "$smoke_dir/$package_name/bin/neomacs" \
      --batch --eval "(kill-emacs 0)"
else
  tar -C "$dist_dir" -czf "$archive" "$package_name"
fi

echo "wrote $archive"
