#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/sign-macos-app.sh PATH/TO/neomacs.app

Sign nested Mach-O code first and then seal the outer application bundle.
Set MACOS_SIGNING_IDENTITY to a Developer ID Application identity for a
Gatekeeper-trusted release. Without it, builds receive an ad-hoc signature so
rewritten Apple-Silicon binaries remain executable, but users may need Apple's
per-app Open Anyway flow.
USAGE
}

if (($# != 1)); then
  usage >&2
  exit 2
fi

app="$1"
contents="$app/Contents"
identity="${MACOS_SIGNING_IDENTITY:--}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
entitlements="${MACOS_ENTITLEMENTS:-$script_dir/macos-entitlements.plist}"

# shellcheck source=./scripts/lib/macos-macho.sh
source "$script_dir/lib/macos-macho.sh"

if [[ "$(uname -s)" != Darwin ]]; then
  echo "macOS code signing must run on macOS" >&2
  exit 1
fi
if [[ "$app" != *.app || ! -d "$contents/MacOS" ]]; then
  echo "invalid macOS application bundle: $app" >&2
  exit 1
fi
if [[ ! -f "$entitlements" ]]; then
  echo "macOS entitlements file does not exist: $entitlements" >&2
  exit 1
fi

sign_args=(--force --sign "$identity")
if [[ "$identity" != - ]]; then
  sign_args+=(--timestamp --options runtime)
else
  echo "warning: MACOS_SIGNING_IDENTITY is unset; applying an ad-hoc signature" >&2
fi

# Apple requires nested code to be signed before its containing bundle.  Avoid
# `codesign --deep` as a signing operation: each code object has an explicit
# signature and the final app signature seals that graph.
for root in Frameworks Helpers PlugIns MacOS; do
  [[ -d "$contents/$root" ]] || continue
  while IFS= read -r -d '' image; do
    is_macho "$image" || continue
    case "$image" in
      "$contents/MacOS/neomacs"|\
      "$contents/MacOS/neomacs-temacs"|\
      "$contents/MacOS/bootstrap-neomacs"|\
      "$contents/MacOS/mock-display")
        codesign "${sign_args[@]}" --entitlements "$entitlements" "$image"
        ;;
      *)
        codesign "${sign_args[@]}" "$image"
        ;;
    esac
  done < <(find "$contents/$root" -type f -print0)
done

codesign "${sign_args[@]}" --entitlements "$entitlements" "$app"
codesign --verify --deep --strict --verbose=2 "$app"

echo "signed $app with identity: $identity"
