#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/sign-macos-app.sh PATH/TO/neomacs.app

Sign nested code first and then seal the outer application bundle.
Set MACOS_SIGNING_IDENTITY to a Developer ID Application identity for a
Gatekeeper-trusted release. Without it, builds receive an ad-hoc signature so
rewritten Apple-Silicon binaries remain executable, but users may need Apple's
per-app Open Anyway flow.

"Nested code" is Apple's definition, not "Mach-O": everything under
Contents/MacOS, Contents/Frameworks, Contents/Helpers and Contents/PlugIns is
sealed as nested code and needs its own signature, whatever its file type.
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

# Which of our binaries are Emacs proper and need the entitlements: the main
# executable and the dumper/harness builds of it, wherever they are staged.
needs_entitlements() {
  case "$(basename "$1")" in
    neomacs|neomacs-temacs|bootstrap-neomacs|mock-display) return 0 ;;
    *) return 1 ;;
  esac
}

# Apple requires nested code to be signed before its containing bundle.  Avoid
# `codesign --deep` as a signing operation: each code object has an explicit
# signature and the final app signature seals that graph.
#
# The set of things that count as nested code is decided by codesign's default
# resource rules, not by file type.  The V2 rules
# (Security, OSX/libsecurity_codesigning/lib/bundlediskrep.cpp,
# BundleDiskRep::defaultResourceRules) carry
#
#   '^(Frameworks|SharedFrameworks|PlugIns|Plug-ins|XPCServices|Helpers|MacOS
#     |Library/(Automator|Spotlight|LoginItems))/' = {nested=#T, weight=10}
#
# and that pattern is applied with regexec (resources.cpp:426,437), i.e. as a
# search over the bundle-relative path, so it matches at ANY depth beneath
# those directories.  signer.cpp then routes every matching file through
# signNested, which throws errSecCSUnsigned -- "code object is not signed at
# all" -- for anything without a cdhash.  A `find -type f` filtered by
# is_macho therefore silently skips exactly the files that break the seal:
# our dump image, and any script or data file a vendoring step drops in.
#
# Symlinks are excluded on purpose: resources.cpp:221-236 strips the nested
# flag for them ("symlinks cannot ever be nested code"), and .DS_Store is
# omitted outright by a weight-2000 rule.
for root in Frameworks Helpers PlugIns MacOS; do
  [[ -d "$contents/$root" ]] || continue
  while IFS= read -r -d '' image; do
    [[ "$(basename "$image")" == .DS_Store ]] && continue
    if needs_entitlements "$image" && is_macho "$image"; then
      codesign "${sign_args[@]}" --entitlements "$entitlements" "$image"
    else
      codesign "${sign_args[@]}" "$image"
    fi
  done < <(find "$contents/$root" -type f -print0)
done

# Report EVERY unsigned nested item before handing over to codesign's own
# verify, which names one subcomponent per run.  A macOS release round trip
# costs about a quarter of an hour, so a check that surfaces one problem at a
# time costs a working day for a handful of files.
unsigned=0
for root in Frameworks Helpers PlugIns MacOS; do
  [[ -d "$contents/$root" ]] || continue
  while IFS= read -r -d '' image; do
    [[ "$(basename "$image")" == .DS_Store ]] && continue
    if ! codesign --verify --strict "$image" >/dev/null 2>&1; then
      echo "nested code is not validly signed: ${image#"$app/"}" >&2
      unsigned=$((unsigned + 1))
    fi
  done < <(find "$contents/$root" -type f -print0)
done
if ((unsigned > 0)); then
  echo "$unsigned nested item(s) under the code roots are unsigned or invalid" >&2
  exit 1
fi

# Seal the bundle only once the whole nested graph is known good, so a missed
# item is reported by name above rather than as codesign's one-at-a-time
# "In subcomponent:" line.
codesign "${sign_args[@]}" --entitlements "$entitlements" "$app"
codesign --verify --deep --strict --verbose=2 "$app"

echo "signed $app with identity: $identity"
