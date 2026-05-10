#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package-release.sh [--target NAME] [--skip-build] [--no-smoke]

Build and package a Neomacs binary release archive.

Options:
  --target NAME   Artifact target suffix, e.g. linux-x86_64.
                  Defaults to "$(uname -s | tr A-Z a-z)-$(uname -m)".
  --skip-build    Package existing target/release artifacts without running
                  cargo xtask fresh-build --release.
  --no-smoke      Do not smoke-test the extracted archive.

Output:
  dist/neomacs-NAME.tar.gz
  dist/SHA256SUMS
USAGE
}

target_name="$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
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

if ((skip_build == 0)); then
  cargo xtask fresh-build --release
fi

release_dir="$repo_root/target/release"
dist_dir="$repo_root/dist"
package_name="neomacs-${target_name}"
package_dir="$dist_dir/$package_name"
archive="$dist_dir/$package_name.tar.gz"

for required in "$release_dir/neomacs" "$release_dir/neomacs.pdump"; do
  if [[ ! -f "$required" ]]; then
    echo "missing required release artifact: $required" >&2
    echo "run cargo xtask fresh-build --release first, or omit --skip-build" >&2
    exit 1
  fi
done

rm -rf "$package_dir" "$archive"
mkdir -p "$package_dir/bin" "$package_dir/share/neomacs"

for binary in neomacs neomacs-temacs bootstrap-neomacs mock-display; do
  if [[ -f "$release_dir/$binary" ]]; then
    install -m 0755 "$release_dir/$binary" "$package_dir/bin/$binary"
  fi
done

shopt -s nullglob
for image in "$release_dir"/*.pdump; do
  install -m 0644 "$image" "$package_dir/bin/$(basename "$image")"
done
shopt -u nullglob

cp -a lisp "$package_dir/share/neomacs/"
cp -a etc "$package_dir/share/neomacs/"
cp -a leim "$package_dir/share/neomacs/"
cp -a info "$package_dir/share/neomacs/" 2>/dev/null || true

install -m 0644 README.md "$package_dir/README.md"
install -m 0644 COPYING "$package_dir/COPYING"

cat >"$package_dir/VERSION" <<VERSION
name: neomacs
target: $target_name
git: $(git rev-parse --short=12 HEAD 2>/dev/null || echo unknown)
built: $(date -u +%Y-%m-%dT%H:%M:%SZ)
VERSION

if ((smoke)); then
  smoke_dir="$(mktemp -d "${TMPDIR:-/tmp}/neomacs-release-smoke.XXXXXX")"
  trap 'rm -rf "$smoke_dir"' EXIT
  tar -C "$dist_dir" -czf "$archive" "$package_name"
  tar -C "$smoke_dir" -xzf "$archive"
  NEOMACS_RUNTIME_ROOT="$smoke_dir/$package_name/share/neomacs" \
    timeout 30s "$smoke_dir/$package_name/bin/neomacs" \
      --batch --eval "(kill-emacs 0)"
else
  tar -C "$dist_dir" -czf "$archive" "$package_name"
fi

(
  cd "$dist_dir"
  sha256sum "$(basename "$archive")" > SHA256SUMS
)

echo "wrote $archive"
echo "wrote $dist_dir/SHA256SUMS"
