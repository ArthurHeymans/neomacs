#!/usr/bin/env bash
set -euo pipefail

if (($# != 2)); then
  echo "usage: $0 PACKAGE_DIR OUTPUT_FILE" >&2
  exit 2
fi

package_dir="$(cd "$1" && pwd -P)"
output_file="$2"
mkdir -p "$(dirname "$output_file")"
temporary_file="$(mktemp "${output_file}.XXXXXX")"
trap 'rm -f "$temporary_file"' EXIT

windows_relative_path() {
  local path="$1"
  local relative="${path#"$package_dir"/}"

  case "$relative" in
    *'"'*|*'$'*|*'\'*|*$'\n'*|*$'\r'*)
      echo "package path cannot be represented safely in NSIS: $relative" >&2
      return 1
      ;;
  esac

  printf '%s' "${relative//\//\\}"
}

{
  printf '; Generated from the packaged payload. Do not edit.\n'

  while IFS= read -r -d '' path; do
    relative="$(windows_relative_path "$path")"
    printf 'Delete "$INSTDIR\\%s"\n' "$relative"
  done < <(find "$package_dir" \( -type f -o -type l \) -print0 | sort -z)

  # Reverse lexical order puts children before their parents for normalized
  # package paths. RMDir without /r preserves any directory containing a file
  # that was created or changed after installation.
  while IFS= read -r -d '' path; do
    relative="$(windows_relative_path "$path")"
    printf 'RMDir "$INSTDIR\\%s"\n' "$relative"
  done < <(find "$package_dir" -mindepth 1 -type d -print0 | sort -zr)

  printf 'Delete "$INSTDIR\\uninstall.exe"\n'
  printf 'RMDir "$INSTDIR"\n'
} > "$temporary_file"

mv "$temporary_file" "$output_file"
trap - EXIT
