#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package-windows-installer.sh [--target TRIPLE] [--skip-build] [--no-smoke]

Build and package NEO Emacs as a Windows .exe installer using NSIS.

Prerequisites:
  NSIS (makensis) must be on PATH.
  On GitHub Actions: choco install nsis.

Output:
  dist/neomacs-{version}-{target}-user-setup.exe
USAGE
}

target_triple="x86_64-pc-windows-msvc"
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

get_version() {
  local v
  v="$(git describe --tags --abbrev=0 2>/dev/null)" && echo "${v#v}" && return
  v="$(git rev-parse --short=12 HEAD 2>/dev/null)" && echo "$v" && return
  echo "0.0.0-dev"
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

dist_dir="$repo_root/dist"
version="$(get_version)"
case "$target_triple" in
  x86_64-pc-windows-msvc) product_arch="x86_64" ;;
  aarch64-pc-windows-msvc) product_arch="aarch64" ;;
  *)
    echo "unsupported Windows installer target: $target_triple" >&2
    exit 1
    ;;
esac
package_name="neomacs-${version}-${target_triple}"
package_dir="$dist_dir/$package_name"
exe_name="neomacs-${version}-${target_triple}-user-setup.exe"
exe_path="$dist_dir/$exe_name"

pkg_args=(--target "$target_triple")
if ((skip_build)); then
  pkg_args+=(--skip-build)
fi
pkg_args+=(--no-smoke)

scripts/package-release.sh "${pkg_args[@]}"

scripts/vendor-windows-gstreamer-runtime.sh \
  --package-root "$package_dir" \
  --bin-dir "$package_dir/bin"

echo "creating Windows installer..."
scripts/compile-windows-installer.sh \
  "$package_dir" "$version" "$exe_path" "$product_arch"

if ((smoke)); then
  echo "smoke-testing installed binary..."
  NEOMACS_RUNTIME_ROOT="$package_dir/share/neomacs" \
    timeout 30s "$package_dir/bin/neomacs.exe" \
      --batch --eval "(kill-emacs 0)" || true
fi
