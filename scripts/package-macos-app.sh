#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package-macos-app.sh [--skip-build] [--no-smoke]

Build and package NEO Emacs as a macOS .app bundle inside a .dmg.

The binary auto-detects the .app bundle layout via the
Resources/neomacs/ path (see load.rs:runtime_project_root).

Environment:
  NO_DMG   If set to "1", produce the .app directory without
           wrapping it in a .dmg image.

Output:
  dist/neomacs-{version}-aarch64-apple-darwin.dmg
  dist/neomacs.app
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
product_name="NEO Emacs"
app_bundle_name="neomacs"
app_bundle="$dist_dir/$app_bundle_name.app"
dmg_name="neomacs-${version}-aarch64-apple-darwin"
dmg="$dist_dir/$dmg_name.dmg"

if ((skip_build == 0)); then
  cargo xtask fresh-build --release
fi

release_dir="$repo_root/target/release"

for required in "$release_dir/neomacs" "$release_dir/neomacs.pdump"; do
  if [[ ! -f "$required" ]]; then
    echo "missing required release artifact: $required" >&2
    echo "run cargo xtask fresh-build --release first, or pass --skip-build" >&2
    exit 1
  fi
done

rm -rf "$app_bundle"

mkdir -p "$app_bundle/Contents/MacOS"
mkdir -p "$app_bundle/Contents/Resources/neomacs"
mkdir -p "$app_bundle/Contents/Frameworks"

for binary in neomacs neomacs-temacs bootstrap-neomacs mock-display; do
  if [[ -f "$release_dir/$binary" ]]; then
    install -m 0755 "$release_dir/$binary" "$app_bundle/Contents/MacOS/$binary"
  fi
done

install -m 0644 "$release_dir/neomacs.pdump" \
  "$app_bundle/Contents/MacOS/neomacs.pdump"

fingerprint="$("$release_dir/neomacs" --fingerprint | tr -d '[:space:]')"
if [[ ! "$fingerprint" =~ ^[[:xdigit:]]{64}$ ]]; then
  echo "invalid neomacs fingerprint from $release_dir/neomacs --fingerprint: $fingerprint" >&2
  exit 1
fi
ln -f "$app_bundle/Contents/MacOS/neomacs.pdump" \
  "$app_bundle/Contents/MacOS/neomacs-${fingerprint}.pdump" \
  || install -m 0644 "$release_dir/neomacs.pdump" \
    "$app_bundle/Contents/MacOS/neomacs-${fingerprint}.pdump"

cp -a lisp "$app_bundle/Contents/Resources/neomacs/"
cp -a etc "$app_bundle/Contents/Resources/neomacs/"
cp -a leim "$app_bundle/Contents/Resources/neomacs/"
cp -a info "$app_bundle/Contents/Resources/neomacs/" 2>/dev/null || true

cat >"$app_bundle/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>neomacs</string>
  <key>CFBundleDisplayName</key>
  <string>${product_name}</string>
  <key>CFBundleExecutable</key>
  <string>neomacs</string>
  <key>CFBundleIdentifier</key>
  <string>org.neomacs</string>
  <key>CFBundleVersion</key>
  <string>${version}</string>
  <key>CFBundleShortVersionString</key>
  <string>${version}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>CFBundleIconFile</key>
  <string>neomacs</string>
</dict>
</plist>
PLIST

if [[ -f assets/logo-128.png ]]; then
  mkdir -p "$app_bundle/Contents/Resources"
  sips -s format icns assets/logo-128.png \
    --out "$app_bundle/Contents/Resources/neomacs.icns" \
    2>/dev/null || true
fi

install -m 0644 README.md "$app_bundle/Contents/Resources/README.md"
install -m 0644 COPYING "$app_bundle/Contents/Resources/COPYING"

if ((smoke)); then
  echo "smoke-testing .app bundle..."
  NEOMACS_RUNTIME_ROOT="$app_bundle/Contents/Resources/neomacs" \
    timeout 30s "$app_bundle/Contents/MacOS/neomacs" \
      --batch --eval "(kill-emacs 0)"
fi

if [[ "${NO_DMG:-}" == "1" ]]; then
  echo "wrote $app_bundle (NO_DMG=1, skipping .dmg)"
  exit 0
fi

echo "creating .dmg..."
rm -f "$dmg"

dmg_staging="$dist_dir/dmg-staging"
rm -rf "$dmg_staging"
mkdir -p "$dmg_staging"

cp -a "$app_bundle" "$dmg_staging/"

ln -sf /Applications "$dmg_staging/Applications"

# hdiutil can intermittently fail with "hdiutil: create failed - Resource
# busy" on CI runners (a leftover mount of the same volume, or Spotlight/mds
# indexing the source folder while it is read). Detach any stale volume and
# retry a few times before giving up.
create_dmg() {
  local vol="/Volumes/$app_bundle_name"
  [[ -d "$vol" ]] && hdiutil detach "$vol" -force >/dev/null 2>&1 || true
  hdiutil create \
    -volname "$app_bundle_name" \
    -srcfolder "$dmg_staging" \
    -ov \
    -format UDZO \
    "$dmg"
}

dmg_attempts=5
for attempt in $(seq 1 "$dmg_attempts"); do
  if create_dmg; then
    break
  fi
  if [[ "$attempt" -eq "$dmg_attempts" ]]; then
    echo "hdiutil create failed after $dmg_attempts attempts" >&2
    exit 1
  fi
  echo "hdiutil create failed (attempt $attempt/$dmg_attempts); retrying in 5s..." >&2
  sleep 5
done

rm -rf "$dmg_staging"

echo "wrote $dmg"
