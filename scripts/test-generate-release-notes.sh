#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/neomacs-release-notes-test.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

dist_dir="$work_dir/dist"
output="$work_dir/release-notes.md"
generated_notes="$work_dir/generated-notes.md"
mkdir -p "$dist_dir"

cat >"$generated_notes" <<'NOTES'
## What's Changed

* Add the release download guide by @eval-exec in https://github.com/eval-exec/neomacs/pull/999

## New Contributors
* @someone made their first contribution in https://github.com/eval-exec/neomacs/pull/998

**Full Changelog**: https://github.com/eval-exec/neomacs/compare/v9.8.6...v9.8.7
NOTES

assets=(
  neomacs-9.8.7-x86_64-unknown-linux-gnu.AppImage
  neomacs-9.8.7-aarch64-unknown-linux-gnu.AppImage
  neomacs_9.8.7_amd64.deb
  neomacs_9.8.7_arm64.deb
  neomacs-9.8.7-1.x86_64.rpm
  neomacs-9.8.7-1.aarch64.rpm
  neomacs-9.8.7-x86_64-unknown-linux-gnu.tar.gz
  neomacs-9.8.7-aarch64-unknown-linux-gnu.tar.gz
  neomacs-9.8.7-aarch64-apple-darwin.dmg
  neomacs-9.8.7-aarch64-apple-darwin.zip
  neomacs-9.8.7-aarch64-apple-darwin.tar.gz
  neomacs-9.8.7-x86_64-pc-windows-msvc-user-setup.exe
  neomacs-9.8.7-x86_64-pc-windows-msvc.zip
  neomacs-9.8.7-aarch64-pc-windows-msvc-user-setup.exe
  neomacs-9.8.7-aarch64-pc-windows-msvc.zip
)

touch "$dist_dir/install.sh" "$dist_dir/SHA256SUMS"
for asset in "${assets[@]}"; do
  touch "$dist_dir/$asset"
done

"$repo_root/scripts/generate-release-notes.sh" \
  --repo eval-exec/neomacs \
  --tag v9.8.7 \
  --dist-dir "$dist_dir" \
  --generated-notes "$generated_notes" \
  --output "$output"

assert_contains() {
  local expected="$1"
  if ! grep -Fq "$expected" "$output"; then
    echo "generated release notes are missing: $expected" >&2
    exit 1
  fi
}

assert_contains '## Download Guide — Pick the Right Build'
assert_contains '<th>Distribution / package</th>'
assert_contains '<th>Architecture</th>'
assert_contains '<th>Download</th>'
assert_contains '<td rowspan="8"><img'
assert_contains '<td rowspan="3" colspan="2">Apple Silicon<br><code>aarch64</code></td>'
assert_contains '<td rowspan="2" colspan="2"><code>x86_64</code></td>'
assert_contains 'alt="Debian logo"> <strong>Debian</strong><br><img'
assert_contains 'alt="Ubuntu logo"> <strong>Ubuntu</strong><br><code>.deb</code>'
assert_contains 'alt="Fedora logo"> <strong>Fedora</strong><br><img'
assert_contains 'alt="Red Hat logo"> <strong>RHEL</strong><br><img'
assert_contains 'alt="openSUSE logo"> <strong>openSUSE</strong><br><code>.rpm</code>'
assert_contains 'https://github.com/eval-exec/neomacs/releases/download/v9.8.7/SHA256SUMS'
assert_contains '<details>'
assert_contains "<summary><strong>What's Changed</strong></summary>"
assert_contains '* Add the release download guide by @eval-exec'
assert_contains '</details>'
assert_contains '## New Contributors'
assert_contains '**Full Changelog**: https://github.com/eval-exec/neomacs/compare/v9.8.6...v9.8.7'

for asset in "${assets[@]}"; do
  assert_contains "href=\"https://github.com/eval-exec/neomacs/releases/download/v9.8.7/$asset\"><code>$asset</code></a>"
done

download_count="$(grep -o 'href="https://github.com/eval-exec/neomacs/releases/download/v9.8.7/[^\"]*"><code>[^<]*</code></a>' "$output" | wc -l | tr -d ' ')"
if [[ "$download_count" != "15" ]]; then
  echo "expected 15 package download links, found $download_count" >&2
  exit 1
fi

if grep -Fq '⬇️' "$output"; then
  echo "generated release notes contain a download emoji" >&2
  exit 1
fi

details_close_line="$(grep -n '^</details>$' "$output" | cut -d: -f1)"
contributors_line="$(grep -n '^## New Contributors$' "$output" | cut -d: -f1)"
if ((details_close_line >= contributors_line)); then
  echo "New Contributors should remain outside the collapsed changelog" >&2
  exit 1
fi

missing_asset="neomacs-9.8.7-aarch64-pc-windows-msvc.zip"
rm "$dist_dir/$missing_asset"
if "$repo_root/scripts/generate-release-notes.sh" \
  --repo eval-exec/neomacs \
  --tag v9.8.7 \
  --dist-dir "$dist_dir" \
  --generated-notes "$generated_notes" \
  --output "$work_dir/incomplete-release-notes.md" \
  >"$work_dir/missing-asset.log" 2>&1; then
  echo "release-note generation accepted a missing asset: $missing_asset" >&2
  exit 1
fi
if ! grep -Fq "missing release asset: $missing_asset" "$work_dir/missing-asset.log"; then
  echo "missing-asset failure did not identify: $missing_asset" >&2
  exit 1
fi

echo "generated release notes match the download-guide contract"
