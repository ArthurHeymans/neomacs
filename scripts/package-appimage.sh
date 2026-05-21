#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package-appimage.sh [--target NAME] [--skip-build] [--no-smoke]

Build and package a Neomacs Linux AppImage.

Options:
  --target NAME   Artifact target suffix. Defaults to linux-x86_64.
  --skip-build    Reuse existing target/release artifacts.
  --no-smoke      Do not smoke-test the AppImage.

Environment:
  LINUXDEPLOY_APPIMAGE   Path to linuxdeploy-x86_64.AppImage or linuxdeploy.
  APPIMAGETOOL_APPIMAGE  Path to appimagetool-x86_64.AppImage or appimagetool.

Output:
  dist/neomacs-NAME.AppImage
  dist/SHA256SUMS
USAGE
}

target_name="linux-x86_64"
skip_build=0
smoke=1

while (($#)); do
  case "$1" in
    --target)
      target_name="${2:?--target requires a value}"
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

dist_dir="$repo_root/dist"
package_name="neomacs-${target_name}"
package_dir="$dist_dir/$package_name"
appdir="$dist_dir/$package_name.AppDir"
appimage="$dist_dir/$package_name.AppImage"

linuxdeploy="${LINUXDEPLOY_APPIMAGE:-$(command -v linuxdeploy || true)}"
appimagetool="${APPIMAGETOOL_APPIMAGE:-$(command -v appimagetool || true)}"

if [[ -z "$linuxdeploy" || ! -x "$linuxdeploy" ]]; then
  echo "linuxdeploy not found; set LINUXDEPLOY_APPIMAGE to an executable path" >&2
  exit 1
fi
if [[ -z "$appimagetool" || ! -x "$appimagetool" ]]; then
  echo "appimagetool not found; set APPIMAGETOOL_APPIMAGE to an executable path" >&2
  exit 1
fi

if [[ ! -x "$package_dir/bin/neomacs" || ! -d "$package_dir/share/neomacs/lisp" ]]; then
  package_args=(--target "$target_name")
  if ((skip_build)); then
    package_args+=(--skip-build)
  fi
  package_args+=(--no-smoke)
  scripts/package-release.sh "${package_args[@]}"
elif ((skip_build == 0)); then
  scripts/package-release.sh --target "$target_name" --no-smoke
fi

rm -rf "$appdir" "$appimage"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/neomacs" "$appdir/usr/share/applications" \
  "$appdir/usr/share/icons/hicolor/128x128/apps"

cp -a "$package_dir/bin/." "$appdir/usr/bin/"
cp -a "$package_dir/share/neomacs/." "$appdir/usr/share/neomacs/"
install -m 0644 "$package_dir/README.md" "$appdir/usr/share/neomacs/README.md"
install -m 0644 "$package_dir/COPYING" "$appdir/usr/share/neomacs/COPYING"

desktop_file="$appdir/usr/share/applications/neomacs.desktop"
cat >"$desktop_file" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Neomacs
Comment=Extensible text editor
Exec=neomacs %F
Icon=neomacs
Terminal=false
Categories=Development;TextEditor;
StartupWMClass=neomacs
DESKTOP

install -m 0644 assets/logo-128.png "$appdir/usr/share/icons/hicolor/128x128/apps/neomacs.png"

cat >"$appdir/AppRun" <<'APPRUN'
#!/usr/bin/env sh
HERE="$(dirname "$(readlink -f "$0")")"
export NEOMACS_RUNTIME_ROOT="${NEOMACS_RUNTIME_ROOT:-$HERE/usr/share/neomacs}"
exec "$HERE/usr/bin/neomacs" "$@"
APPRUN
chmod 0755 "$appdir/AppRun"

"$linuxdeploy" \
  --appdir "$appdir" \
  --executable "$appdir/usr/bin/neomacs" \
  --desktop-file "$desktop_file" \
  --icon-file "$appdir/usr/share/icons/hicolor/128x128/apps/neomacs.png"

env -u SOURCE_DATE_EPOCH ARCH=x86_64 "$appimagetool" "$appdir" "$appimage"
chmod 0755 "$appimage"

if ((smoke)); then
  APPIMAGE_EXTRACT_AND_RUN=1 \
    NEOMACS_RUNTIME_ROOT= \
    timeout 30s "$appimage" --batch --eval "(kill-emacs 0)"
fi

(
  cd "$dist_dir"
  if [[ ! -f SHA256SUMS ]]; then
    : > SHA256SUMS
  fi
  tmp_sums="$(mktemp)"
  grep -v "  $(basename "$appimage")\$" SHA256SUMS > "$tmp_sums" || true
  sha256sum "$(basename "$appimage")" >> "$tmp_sums"
  sort "$tmp_sums" > SHA256SUMS
  rm -f "$tmp_sums"
)

echo "wrote $appimage"
echo "updated $dist_dir/SHA256SUMS"
