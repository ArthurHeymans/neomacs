#!/usr/bin/env bash
set -euo pipefail

if (($# != 3)); then
  echo "usage: $0 PACKAGE_DIR PRODUCT_VERSION OUTPUT_FILE" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="$(cd "$1" && pwd -P)"
version="$2"
output_dir="$(cd "$(dirname "$3")" && pwd -P)"
output_file="$output_dir/$(basename "$3")"

if ! command -v makensis &>/dev/null; then
  echo "makensis not found; install NSIS first" >&2
  exit 1
fi

uninstall_include="$(mktemp "$output_dir/neomacs-uninstall-files.XXXXXX.nsh")"
trap 'rm -f "$uninstall_include"' EXIT
"$repo_root/scripts/generate-nsis-uninstall-include.sh" \
  "$package_dir" \
  "$uninstall_include"

makensis -V2 \
  -DPRODUCT_VERSION="$version" \
  -DSOURCE_DIR="$(cygpath -w "$package_dir" 2>/dev/null || echo "$package_dir")" \
  -DOUTPUT_FILE="$(cygpath -w "$output_file" 2>/dev/null || echo "$output_file")" \
  -DUNINSTALL_INCLUDE="$(cygpath -w "$uninstall_include" 2>/dev/null || echo "$uninstall_include")" \
  "$repo_root/assets/windows-installer.nsi"

echo "wrote $output_file"
