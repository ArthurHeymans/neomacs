#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/vendor-macos-runtime.sh PATH/TO/neomacs.app

Vendor Neomacs' non-system dynamic-library closure and GStreamer runtime into
a macOS application bundle.  The resulting Mach-O install names are relative
to Contents/Frameworks so the app can be moved to any directory.

Requires macOS, pkg-config, file, install_name_tool, lipo, and otool.
USAGE
}

if (($# != 1)); then
  usage >&2
  exit 2
fi

app="$1"
contents="$app/Contents"
macos_dir="$contents/MacOS"
frameworks_dir="$contents/Frameworks"
helpers_dir="$contents/Helpers"
gst_plugins_dir="$contents/PlugIns/gstreamer-1.0"
gio_modules_dir="$contents/PlugIns/gio"
fontconfig_dir="$contents/Resources/fontconfig"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=./scripts/lib/macos-macho.sh
source "$script_dir/lib/macos-macho.sh"

if [[ "$(uname -s)" != Darwin ]]; then
  echo "macOS runtime vendoring must run on macOS" >&2
  exit 1
fi
if [[ "$app" != *.app || ! -d "$macos_dir" ]]; then
  echo "invalid macOS application bundle: $app" >&2
  exit 1
fi

for command in file install_name_tool lipo otool pkg-config; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "$command is required to vendor the macOS runtime" >&2
    exit 1
  fi
done

pkg_config_variable() {
  local module="$1"
  local variable="$2"
  local value
  value="$(pkg-config --variable="$variable" "$module")"
  if [[ -z "$value" ]]; then
    echo "pkg-config module $module has no $variable value" >&2
    return 1
  fi
  printf '%s\n' "$value"
}

copy_macho_tree() {
  local source_root="$1"
  local destination_root="$2"
  local label="$3"
  local copied=0
  local source relative destination

  [[ -d "$source_root" ]] || return 0
  while IFS= read -r -d '' source; do
    is_macho "$source" || continue
    relative="${source#"$source_root"/}"
    destination="$destination_root/$relative"
    mkdir -p "$(dirname "$destination")"
    install -m 0755 "$source" "$destination"
    copied=$((copied + 1))
  done < <(find -L "$source_root" -type f -print0)

  echo "vendored $copied $label Mach-O images"
}

copy_flat_macho_dir() {
  local source_root="$1"
  local destination_root="$2"
  local copied=0
  local source destination

  # The official runtime keeps shared libraries directly below libdir.  Its
  # plug-ins and GIO modules are copied into their semantic bundle locations
  # separately, so deliberately do not recurse here.
  for source in "$source_root"/*; do
    [[ -f "$source" ]] || continue
    is_macho "$source" || continue
    destination="$destination_root/$(basename "$source")"
    if [[ -e "$destination" ]] && ! cmp -s "$source" "$destination"; then
      echo "conflicting runtime libraries share a basename: $source" >&2
      echo "  destination: $destination" >&2
      exit 1
    fi
    install -m 0755 "$source" "$destination"
    copied=$((copied + 1))
  done

  if ((copied == 0)); then
    echo "no Mach-O libraries found in $source_root" >&2
    exit 1
  fi
  echo "vendored $copied GStreamer runtime libraries"
}

gst_plugins_source="$(pkg_config_variable gstreamer-1.0 pluginsdir)"
gst_libexec_dir="$(pkg_config_variable gstreamer-1.0 libexecdir)"
gst_libdir="$(pkg_config_variable gstreamer-1.0 libdir)"
gst_scanner_source="$gst_libexec_dir/gstreamer-1.0/gst-plugin-scanner"

if [[ ! -d "$gst_plugins_source" ]]; then
  echo "GStreamer plugin directory does not exist: $gst_plugins_source" >&2
  exit 1
fi
if [[ ! -f "$gst_scanner_source" ]]; then
  echo "GStreamer plugin scanner does not exist: $gst_scanner_source" >&2
  exit 1
fi

# Resolve every destination before removing old packaged content.  All targets
# are fixed children of the validated .app rather than user-controlled globs.
rm -rf \
  "$frameworks_dir" \
  "$helpers_dir" \
  "$gst_plugins_dir" \
  "$gio_modules_dir" \
  "$fontconfig_dir"
mkdir -p "$frameworks_dir" "$helpers_dir" "$gst_plugins_dir"

copy_flat_macho_dir "$gst_libdir" "$frameworks_dir"
copy_macho_tree "$gst_plugins_source" "$gst_plugins_dir" "GStreamer plugin"
install -m 0755 "$gst_scanner_source" "$helpers_dir/gst-plugin-scanner"

gio_modules_source="$(pkg-config --variable=giomoduledir gio-2.0 2>/dev/null || true)"
if [[ -n "$gio_modules_source" && -d "$gio_modules_source" ]]; then
  mkdir -p "$gio_modules_dir"
  copy_macho_tree "$gio_modules_source" "$gio_modules_dir" "GIO module"
fi

fontconfig_source="$(pkg_config_variable fontconfig confdir)"
if [[ ! -f "$fontconfig_source/fonts.conf" ]]; then
  echo "Fontconfig configuration does not exist: $fontconfig_source/fonts.conf" >&2
  exit 1
fi
mkdir -p "$fontconfig_dir"
cp -RL "$fontconfig_source/." "$fontconfig_dir/"

bundle_arch="${MACOS_BUNDLE_ARCH:-$(uname -m)}"
[[ "$bundle_arch" == aarch64 ]] && bundle_arch=arm64
for root in MacOS Frameworks Helpers PlugIns; do
  [[ -d "$contents/$root" ]] || continue
  while IFS= read -r -d '' image; do
    is_macho "$image" || continue
    archs="$(lipo -archs "$image")"
    if [[ "$archs" == "$bundle_arch" ]]; then
      continue
    fi
    if [[ " $archs " != *" $bundle_arch "* ]]; then
      echo "$image does not contain required architecture $bundle_arch: $archs" >&2
      exit 1
    fi
    thin_image="$image.neomacs-thin"
    lipo -thin "$bundle_arch" "$image" -output "$thin_image"
    mv "$thin_image" "$image"
  done < <(find "$contents/$root" -type f -print0)
done

# The official SDK uses both absolute GStreamer.framework paths and @rpath
# spellings.  Flatten the runtime dylibs into Contents/Frameworks and map every
# non-system load command to one explicit bundle-relative identity.  Copying
# the complete upstream runtime set is intentional: GStreamer selects plug-ins
# from media content at runtime, so a build-time dependency walk is incomplete.
relocated=0
image_count=0
for root in MacOS Frameworks Helpers PlugIns; do
  [[ -d "$contents/$root" ]] || continue
  while IFS= read -r -d '' image; do
    is_macho "$image" || continue
    image_count=$((image_count + 1))
    if ! dependencies="$(macho_dependency_paths otool "$image")"; then
      echo "failed to inspect Mach-O load commands: $image" >&2
      exit 1
    fi
    while IFS= read -r dependency; do
      [[ -n "$dependency" ]] || continue
      case "$dependency" in
        /usr/lib/*|/System/Library/*)
          continue
          ;;
      esac

      library_name="$(basename "$dependency")"
      bundled_library="$frameworks_dir/$library_name"
      if [[ ! -f "$bundled_library" ]]; then
        echo "the pinned runtime does not provide $dependency" >&2
        echo "  required by: $image" >&2
        exit 1
      fi
      bundled_identity="@executable_path/../Frameworks/$library_name"
      if [[ "$dependency" != "$bundled_identity" ]]; then
        install_name_tool -change "$dependency" "$bundled_identity" "$image"
        relocated=$((relocated + 1))
      fi
    done <<<"$dependencies"
  done < <(find "$contents/$root" -type f -print0)
done

if ((image_count == 0)); then
  echo "no Mach-O images found to vendor in $app" >&2
  exit 1
fi

while IFS= read -r -d '' library; do
  is_macho "$library" || continue
  current_id="$(otool -D "$library" 2>/dev/null | sed -n '2p')"
  [[ -n "$current_id" ]] || continue
  install_name_tool -id \
    "@executable_path/../Frameworks/$(basename "$library")" \
    "$library"
done < <(find "$frameworks_dir" -type f -print0)

echo "relocated $relocated non-system Mach-O load commands"

"$(dirname "$0")/audit-macos-app.sh" "$app"

mkdir -p "$contents/Resources/vendor/gstreamer"
gst_version="$(pkg-config --modversion gstreamer-1.0)"
printf '%s\n' \
  'This application contains a private GStreamer runtime.' \
  "Version: $gst_version" \
  'Project: https://gstreamer.freedesktop.org/' \
  'License information: https://gstreamer.freedesktop.org/documentation/frequently-asked-questions/licensing.html' \
  >"$contents/Resources/vendor/gstreamer/README.txt"

echo "vendored relocatable macOS runtime into $app"
