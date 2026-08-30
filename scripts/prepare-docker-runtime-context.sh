#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/prepare-docker-runtime-context.sh \
  --archive FILE --target TRIPLE --output DIR

Validate and extract one canonical Linux release tarball into a Docker build
context. The output directory must not already exist and will contain rootfs/.

All scratch data is created below ./tmp, never /tmp.
USAGE
}

archive=""
target_triple=""
output_dir=""

while (($#)); do
  case "$1" in
    --archive)
      archive="${2:?--archive requires a value}"
      shift 2
      ;;
    --target)
      target_triple="${2:?--target requires a value}"
      shift 2
      ;;
    --output)
      output_dir="${2:?--output requires a value}"
      shift 2
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

if [[ -z "$archive" || -z "$target_triple" || -z "$output_dir" ]]; then
  usage >&2
  exit 2
fi
if [[ ! -f "$archive" ]]; then
  echo "release archive not found: $archive" >&2
  exit 1
fi
case "$target_triple" in
  x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu) ;;
  *)
    echo "unsupported Docker release target: $target_triple" >&2
    exit 1
    ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
archive="$(cd "$(dirname "$archive")" && pwd)/$(basename "$archive")"
archive_name="$(basename "$archive")"
case "$archive_name" in
  neomacs-*-${target_triple}.tar.gz) ;;
  *)
    echo "archive name does not identify target $target_triple: $archive_name" >&2
    exit 1
    ;;
esac
package_name="${archive_name%.tar.gz}"

if [[ -e "$output_dir" || -L "$output_dir" ]]; then
  echo "output already exists: $output_dir" >&2
  exit 1
fi
output_parent="$(dirname "$output_dir")"
output_name="$(basename "$output_dir")"
mkdir -p "$output_parent"
output_parent="$(cd "$output_parent" && pwd)"
output_dir="$output_parent/$output_name"
if [[ -e "$output_dir" || -L "$output_dir" ]]; then
  echo "output already exists: $output_dir" >&2
  exit 1
fi

mkdir -p "$repo_root/tmp"
staging="$(mktemp -d "$repo_root/tmp/docker-runtime-context.XXXXXX")"
cleanup() {
  rm -rf "$staging"
}
trap cleanup EXIT

member_count=0
while IFS= read -r member; do
  normalized="${member#./}"
  [[ -n "$normalized" ]] || continue
  member_count=$((member_count + 1))
  if [[ "$normalized" == /* || "/$normalized/" == *"/../"* ]]; then
    echo "unsafe archive member: $member" >&2
    exit 1
  fi
  member_root="${normalized%%/*}"
  if [[ "$member_root" != "$package_name" ]]; then
    echo "archive member is outside the expected release root $package_name: $member" >&2
    exit 1
  fi
done < <(tar -tzf "$archive")

if ((member_count == 0)); then
  echo "release archive is empty: $archive" >&2
  exit 1
fi

mkdir -p "$staging/rootfs"
tar -C "$staging/rootfs" \
  --extract --gzip --file "$archive" \
  --strip-components=1 --no-same-owner --delay-directory-restore

rootfs="$staging/rootfs"
for required_file in \
  "$rootfs/bin/neomacs" \
  "$rootfs/bin/neomacsclient" \
  "$rootfs/bin/neomacs.pdump" \
  "$rootfs/VERSION"
do
  if [[ ! -f "$required_file" ]]; then
    echo "release archive is missing required file: ${required_file#"$rootfs/"}" >&2
    exit 1
  fi
done
for required_dir in "$rootfs/share/neomacs/lisp" "$rootfs/share/neomacs/etc"; do
  if [[ ! -d "$required_dir" ]]; then
    echo "release archive is missing required directory: ${required_dir#"$rootfs/"}" >&2
    exit 1
  fi
done
for required_executable in "$rootfs/bin/neomacs" "$rootfs/bin/neomacsclient"; do
  if [[ ! -x "$required_executable" ]]; then
    echo "release binary is not executable: ${required_executable#"$rootfs/"}" >&2
    exit 1
  fi
done
if ! grep -Fxq "target: $target_triple" "$rootfs/VERSION"; then
  echo "VERSION target does not match requested target $target_triple" >&2
  exit 1
fi

unexpected_type="$(find "$rootfs" \
  ! -type f ! -type d ! -type l -print -quit)"
if [[ -n "$unexpected_type" ]]; then
  echo "release archive contains an unsupported file type: ${unexpected_type#"$rootfs/"}" >&2
  exit 1
fi

while IFS= read -r -d '' link; do
  resolved="$(realpath -m -- "$link")"
  case "$resolved" in
    "$rootfs"|"$rootfs"/*) ;;
    *)
      echo "release symlink escapes the runtime root: ${link#"$rootfs/"}" >&2
      exit 1
      ;;
  esac
done < <(find "$rootfs" -type l -print0)

mv "$staging" "$output_dir"
trap - EXIT
echo "prepared Docker runtime context at $output_dir"
