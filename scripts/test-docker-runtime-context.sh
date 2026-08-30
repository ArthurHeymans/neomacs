#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
prepare_script="$repo_root/scripts/prepare-docker-runtime-context.sh"
runtime_dockerfile="$repo_root/docker/Dockerfile.runtime"
target_triple="x86_64-unknown-linux-gnu"
package_name="neomacs-9.8.7-$target_triple"

mkdir -p "$repo_root/tmp"
work_dir="$(mktemp -d "$repo_root/tmp/docker-runtime-test.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

make_release_fixture() {
  local fixture_root="$1"
  local fixture_target="$2"
  local root="$fixture_root/$package_name"

  mkdir -p "$root/bin" "$root/share/neomacs/lisp" "$root/share/neomacs/etc"
  printf '#!/bin/sh\nprintf "Neomacs fixture\\n"\n' >"$root/bin/neomacs"
  printf '#!/bin/sh\nexit 0\n' >"$root/bin/neomacsclient"
  chmod 0755 "$root/bin/neomacs" "$root/bin/neomacsclient"
  printf 'portable dump fixture\n' >"$root/bin/neomacs.pdump"
  printf 'lisp fixture\n' >"$root/share/neomacs/lisp/loadup.el"
  printf 'etc fixture\n' >"$root/share/neomacs/etc/NEWS"
  printf 'name: neomacs\ntarget: %s\ngit: abcdef123456\nbuilt: 2026-08-30T00:00:00Z\n' \
    "$fixture_target" >"$root/VERSION"
}

fixture="$work_dir/fixture"
make_release_fixture "$fixture" "$target_triple"
archive="$work_dir/$package_name.tar.gz"
tar -C "$fixture" -czf "$archive" "$package_name"

context="$work_dir/context"
"$prepare_script" \
  --archive "$archive" \
  --target "$target_triple" \
  --output "$context"

test -x "$context/rootfs/bin/neomacs"
test -x "$context/rootfs/bin/neomacsclient"
test -f "$context/rootfs/bin/neomacs.pdump"
test -d "$context/rootfs/share/neomacs/lisp"
test -d "$context/rootfs/share/neomacs/etc"
test -f "$context/rootfs/VERSION"
test ! -e "$context/rootfs/$package_name"

if "$prepare_script" \
  --archive "$archive" \
  --target "$target_triple" \
  --output "$context" 2>"$work_dir/existing-output.err"
then
  echo "preparation unexpectedly overwrote an existing context" >&2
  exit 1
fi
grep -Fq 'output already exists' "$work_dir/existing-output.err"

wrong_target_fixture="$work_dir/wrong-target-fixture"
make_release_fixture "$wrong_target_fixture" "aarch64-unknown-linux-gnu"
wrong_target_archive="$work_dir/wrong-target/$package_name.tar.gz"
mkdir -p "$(dirname "$wrong_target_archive")"
tar -C "$wrong_target_fixture" -czf "$wrong_target_archive" "$package_name"
if "$prepare_script" \
  --archive "$wrong_target_archive" \
  --target "$target_triple" \
  --output "$work_dir/wrong-target-context" 2>"$work_dir/wrong-target.err"
then
  echo "preparation accepted mismatched VERSION target metadata" >&2
  exit 1
fi
grep -Fq 'VERSION target does not match' "$work_dir/wrong-target.err"

extra_root="$work_dir/extra-root"
mkdir -p "$extra_root"
printf 'not part of the release\n' >"$extra_root/unexpected"
cp -a "$fixture/$package_name" "$extra_root/"
extra_archive="$work_dir/extra/$package_name.tar.gz"
mkdir -p "$(dirname "$extra_archive")"
tar -C "$extra_root" -czf "$extra_archive" "$package_name" unexpected
if "$prepare_script" \
  --archive "$extra_archive" \
  --target "$target_triple" \
  --output "$work_dir/extra-context" 2>"$work_dir/extra-root.err"
then
  echo "preparation accepted an archive with an extra top-level entry" >&2
  exit 1
fi
grep -Fq 'outside the expected release root' "$work_dir/extra-root.err"

grep -Fq 'FROM ubuntu:22.04@sha256:' "$runtime_dockerfile"
grep -Fq 'COPY --chown=root:root rootfs/ /opt/neomacs/' "$runtime_dockerfile"
grep -Fq '/home/neomacs/.emacs.d' "$runtime_dockerfile"
grep -Fq 'USER neomacs' "$runtime_dockerfile"
grep -Fq 'ENTRYPOINT ["/opt/neomacs/bin/neomacs"]' "$runtime_dockerfile"
grep -Fq 'CMD ["-nw"]' "$runtime_dockerfile"

echo "Docker runtime context contract passed"
