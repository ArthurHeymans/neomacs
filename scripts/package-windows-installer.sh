#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package-windows-installer.sh [--skip-build] [--no-smoke]

Build and package NEO Emacs as a Windows .exe installer using NSIS.

Prerequisites:
  NSIS (makensis) must be on PATH.
  On GitHub Actions: choco install nsis.

Output:
  dist/neomacs-{version}-x86_64-pc-windows-msvc.exe
USAGE
}

skip_build=0
smoke=1

while (($#)); do
  case "$1" in
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
target_triple="x86_64-pc-windows-msvc"
package_name="neomacs-${version}-${target_triple}"
package_dir="$dist_dir/$package_name"
exe_name="neomacs-${version}-${target_triple}.exe"
exe_path="$dist_dir/$exe_name"

if ! command -v makensis &>/dev/null; then
  echo "makensis not found; install NSIS first" >&2
  exit 1
fi

pkg_args=(--target "$target_triple")
if ((skip_build)); then
  pkg_args+=(--skip-build)
fi
pkg_args+=(--no-smoke)

scripts/package-release.sh "${pkg_args[@]}"

nsi_script="$repo_root/assets/windows-installer.nsi"
if [[ ! -f "$nsi_script" ]]; then
  echo "NSIS script not found: $nsi_script" >&2
  exit 1
fi

unix2dos() {
  if command -v unix2dos &>/dev/null; then
    unix2dos "$@"
  elif command -v sed &>/dev/null; then
    for f in "$@"; do
      sed -i 's/$/\r/' "$f"
    done
  fi
}

echo "creating Windows installer..."

makensis -V2 \
  -DPRODUCT_VERSION="${version}" \
  -DSOURCE_DIR="$(cygpath -w "$package_dir" 2>/dev/null || echo "$package_dir")" \
  -DOUTPUT_FILE="$(cygpath -w "$exe_path" 2>/dev/null || echo "$exe_path")" \
  "$nsi_script"

if ((smoke)); then
  echo "smoke-testing installed binary..."
  NEOMACS_RUNTIME_ROOT="$package_dir/share/neomacs" \
    timeout 30s "$package_dir/bin/neomacs.exe" \
      --batch --eval "(kill-emacs 0)" || true
fi

echo "wrote $exe_path"
