#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/generate-release-notes.sh --repo OWNER/REPO --tag TAG
       --dist-dir DIR --generated-notes FILE --output FILE

Generate a release body containing the download guide and GitHub's generated
changelog. Package links are derived from TAG and checked against DIR. The
What's Changed section from FILE is collapsed by default.
USAGE
}

repository=""
tag=""
dist_dir=""
generated_notes=""
output=""

while (($#)); do
  case "$1" in
    --repo)
      repository="${2:?--repo requires a value}"
      shift 2
      ;;
    --tag)
      tag="${2:?--tag requires a value}"
      shift 2
      ;;
    --dist-dir)
      dist_dir="${2:?--dist-dir requires a value}"
      shift 2
      ;;
    --generated-notes)
      generated_notes="${2:?--generated-notes requires a value}"
      shift 2
      ;;
    --output)
      output="${2:?--output requires a value}"
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

if [[ -z "$repository" || -z "$tag" || -z "$dist_dir" || -z "$generated_notes" || -z "$output" ]]; then
  usage >&2
  exit 2
fi
if [[ ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "invalid GitHub repository: $repository" >&2
  exit 2
fi
if [[ ! "$tag" =~ ^v[0-9][A-Za-z0-9._-]*$ ]]; then
  echo "release tag must start with v followed by a version: $tag" >&2
  exit 2
fi
if [[ ! -d "$dist_dir" ]]; then
  echo "release artifact directory not found: $dist_dir" >&2
  exit 1
fi
if [[ ! -f "$generated_notes" ]]; then
  echo "generated GitHub release notes not found: $generated_notes" >&2
  exit 1
fi
if ! grep -Fxq "## What's Changed" "$generated_notes"; then
  echo "generated GitHub release notes have no What's Changed section: $generated_notes" >&2
  exit 1
fi

version="${tag#v}"
release_base="https://github.com/$repository/releases/download/$tag"

required_assets=(
  install.sh
  SHA256SUMS
  "neomacs-$version-x86_64-unknown-linux-gnu.AppImage"
  "neomacs-$version-aarch64-unknown-linux-gnu.AppImage"
  "neomacs_${version}_amd64.deb"
  "neomacs_${version}_arm64.deb"
  "neomacs-$version-1.x86_64.rpm"
  "neomacs-$version-1.aarch64.rpm"
  "neomacs-$version-x86_64-unknown-linux-gnu.tar.gz"
  "neomacs-$version-aarch64-unknown-linux-gnu.tar.gz"
  "neomacs-$version-aarch64-apple-darwin.dmg"
  "neomacs-$version-aarch64-apple-darwin.zip"
  "neomacs-$version-aarch64-apple-darwin.tar.gz"
  "neomacs-$version-x86_64-pc-windows-msvc-user-setup.exe"
  "neomacs-$version-x86_64-pc-windows-msvc.zip"
  "neomacs-$version-aarch64-pc-windows-msvc-user-setup.exe"
  "neomacs-$version-aarch64-pc-windows-msvc.zip"
)

missing_assets=0
for asset in "${required_assets[@]}"; do
  if [[ ! -f "$dist_dir/$asset" ]]; then
    echo "missing release asset: $asset" >&2
    missing_assets=$((missing_assets + 1))
  fi
done
if ((missing_assets > 0)); then
  exit 1
fi

cat >"$output" <<HTML
## Download Guide — Pick the Right Build

<table>
  <thead>
    <tr>
      <th>Platform</th>
      <th>Distribution / package</th>
      <th>Architecture</th>
      <th>Download</th>
      <th>Notes</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td rowspan="8"><img src="https://cdn.jsdelivr.net/gh/devicons/devicon@v2.17.0/icons/linux/linux-original.svg" width="36" height="36" alt="Linux logo"><br><strong>Linux</strong></td>
      <td rowspan="2"><img src="https://cdn.simpleicons.org/appimage" width="28" height="28" alt="AppImage logo"> <strong>AppImage</strong><br>Any distribution</td>
      <td><code>x86_64</code></td>
      <td><a href="$release_base/neomacs-$version-x86_64-unknown-linux-gnu.AppImage"><code>neomacs-$version-x86_64-unknown-linux-gnu.AppImage</code></a></td>
      <td>⭐ Recommended for most Intel/AMD Linux computers</td>
    </tr>
    <tr>
      <td><code>aarch64</code></td>
      <td><a href="$release_base/neomacs-$version-aarch64-unknown-linux-gnu.AppImage"><code>neomacs-$version-aarch64-unknown-linux-gnu.AppImage</code></a></td>
      <td>⭐ Recommended portable build for ARM64 Linux computers</td>
    </tr>
    <tr>
      <td rowspan="2"><img src="https://cdn.jsdelivr.net/gh/devicons/devicon@v2.17.0/icons/debian/debian-original.svg" width="24" height="24" alt="Debian logo"> <strong>Debian</strong><br><img src="https://cdn.jsdelivr.net/gh/devicons/devicon@v2.17.0/icons/ubuntu/ubuntu-original.svg" width="24" height="24" alt="Ubuntu logo"> <strong>Ubuntu</strong><br><code>.deb</code></td>
      <td><code>x86_64</code></td>
      <td><a href="$release_base/neomacs_${version}_amd64.deb"><code>neomacs_${version}_amd64.deb</code></a></td>
      <td>Native package for Intel/AMD Debian-based distributions</td>
    </tr>
    <tr>
      <td><code>aarch64</code></td>
      <td><a href="$release_base/neomacs_${version}_arm64.deb"><code>neomacs_${version}_arm64.deb</code></a></td>
      <td>Native package for ARM64 Debian-based distributions</td>
    </tr>
    <tr>
      <td rowspan="2"><img src="https://cdn.jsdelivr.net/gh/devicons/devicon@v2.17.0/icons/fedora/fedora-original.svg" width="22" height="22" alt="Fedora logo"> <strong>Fedora</strong><br><img src="https://cdn.jsdelivr.net/gh/devicons/devicon@v2.17.0/icons/redhat/redhat-original.svg" width="22" height="22" alt="Red Hat logo"> <strong>RHEL</strong><br><img src="https://cdn.jsdelivr.net/gh/devicons/devicon@v2.17.0/icons/opensuse/opensuse-original.svg" width="22" height="22" alt="openSUSE logo"> <strong>openSUSE</strong><br><code>.rpm</code></td>
      <td><code>x86_64</code></td>
      <td><a href="$release_base/neomacs-$version-1.x86_64.rpm"><code>neomacs-$version-1.x86_64.rpm</code></a></td>
      <td>Native RPM package for Intel/AMD computers</td>
    </tr>
    <tr>
      <td><code>aarch64</code></td>
      <td><a href="$release_base/neomacs-$version-1.aarch64.rpm"><code>neomacs-$version-1.aarch64.rpm</code></a></td>
      <td>Native RPM package for ARM64 computers</td>
    </tr>
    <tr>
      <td rowspan="2"><img src="https://cdn.jsdelivr.net/gh/vscode-icons/vscode-icons@v12.19.0/icons/file_type_zip.svg" width="28" height="28" alt="Archive file icon"> <strong>Portable archive</strong><br><code>.tar.gz</code></td>
      <td><code>x86_64</code></td>
      <td><a href="$release_base/neomacs-$version-x86_64-unknown-linux-gnu.tar.gz"><code>neomacs-$version-x86_64-unknown-linux-gnu.tar.gz</code></a></td>
      <td>For manual installation on Intel/AMD Linux</td>
    </tr>
    <tr>
      <td><code>aarch64</code></td>
      <td><a href="$release_base/neomacs-$version-aarch64-unknown-linux-gnu.tar.gz"><code>neomacs-$version-aarch64-unknown-linux-gnu.tar.gz</code></a></td>
      <td>For manual installation on ARM64 Linux</td>
    </tr>
    <tr>
      <td rowspan="3"><img src="https://cdn.simpleicons.org/apple/808080" width="32" height="32" alt="Apple logo"><br><strong>macOS</strong></td>
      <td rowspan="3" colspan="2">Apple Silicon<br><code>aarch64</code></td>
      <td><a href="$release_base/neomacs-$version-aarch64-apple-darwin.dmg"><code>neomacs-$version-aarch64-apple-darwin.dmg</code></a></td>
      <td>⭐ Recommended DMG installer</td>
    </tr>
    <tr>
      <td><a href="$release_base/neomacs-$version-aarch64-apple-darwin.zip"><code>neomacs-$version-aarch64-apple-darwin.zip</code></a></td>
      <td>Application bundle in a ZIP archive</td>
    </tr>
    <tr>
      <td><a href="$release_base/neomacs-$version-aarch64-apple-darwin.tar.gz"><code>neomacs-$version-aarch64-apple-darwin.tar.gz</code></a></td>
      <td>Application bundle in a tar archive</td>
    </tr>
    <tr>
      <td rowspan="4"><img src="https://cdn.jsdelivr.net/gh/devicons/devicon@v2.17.0/icons/windows11/windows11-original.svg" width="32" height="32" alt="Windows logo"><br><strong>Windows</strong></td>
      <td rowspan="2" colspan="2"><code>x86_64</code></td>
      <td><a href="$release_base/neomacs-$version-x86_64-pc-windows-msvc-user-setup.exe"><code>neomacs-$version-x86_64-pc-windows-msvc-user-setup.exe</code></a></td>
      <td>⭐ Recommended installer for most Windows computers</td>
    </tr>
    <tr>
      <td><a href="$release_base/neomacs-$version-x86_64-pc-windows-msvc.zip"><code>neomacs-$version-x86_64-pc-windows-msvc.zip</code></a></td>
      <td>Portable ZIP for manual installation</td>
    </tr>
    <tr>
      <td rowspan="2" colspan="2"><code>aarch64</code></td>
      <td><a href="$release_base/neomacs-$version-aarch64-pc-windows-msvc-user-setup.exe"><code>neomacs-$version-aarch64-pc-windows-msvc-user-setup.exe</code></a></td>
      <td>⭐ Recommended installer for Windows on ARM</td>
    </tr>
    <tr>
      <td><a href="$release_base/neomacs-$version-aarch64-pc-windows-msvc.zip"><code>neomacs-$version-aarch64-pc-windows-msvc.zip</code></a></td>
      <td>Portable ZIP for Windows on ARM</td>
    </tr>
  </tbody>
</table>

### Verify your download

SHA-256 checksums for every release asset are available in [SHA256SUMS]($release_base/SHA256SUMS).
HTML

printf '\n' >>"$output"
awk -v summary="<summary><strong>What's Changed</strong></summary>" '
  $0 == "## What\047s Changed" {
    print "<details>"
    print summary
    print ""
    in_changes = 1
    next
  }
  in_changes && ($0 == "## New Contributors" || index($0, "**Full Changelog**:") == 1) {
    print "</details>"
    print ""
    in_changes = 0
  }
  { print }
  END {
    if (in_changes) {
      print "</details>"
    }
  }
' "$generated_notes" >>"$output"

echo "wrote $output"
