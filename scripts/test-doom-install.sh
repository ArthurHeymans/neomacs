#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
neomacs_bin="${NEOMACS_BIN:-$repo_root/target/release/neomacs}"
runtime_root="${NEOMACS_RUNTIME_ROOT:-$repo_root}"
doom_repository="${DOOM_REPOSITORY:-https://github.com/doomemacs/doomemacs.git}"
doom_revision="${DOOM_REVISION:-}"

if [[ ! -x "$neomacs_bin" ]]; then
  echo "neomacs executable not found: $neomacs_bin" >&2
  exit 1
fi
neomacs_bin="$(cd "$(dirname "$neomacs_bin")" && pwd)/$(basename "$neomacs_bin")"
runtime_root="$(cd "$runtime_root" && pwd)"

mkdir -p "$repo_root/tmp"
work_dir="$(mktemp -d "$repo_root/tmp/doom-install.XXXXXX")"
trap 'rm -rf -- "$work_dir"' EXIT

home_dir="$work_dir/home"
xdg_config_home="$home_dir/.config"
doom_emacs_dir="$xdg_config_home/emacs"
doom_user_dir="$xdg_config_home/doom"
mkdir -p \
  "$home_dir/.cache" \
  "$home_dir/.local/share" \
  "$home_dir/.local/state"

if [[ -n "$doom_revision" ]]; then
  git init --quiet "$doom_emacs_dir"
  git -C "$doom_emacs_dir" remote add origin "$doom_repository"
  git -C "$doom_emacs_dir" fetch --quiet --depth 1 origin "$doom_revision"
  git -C "$doom_emacs_dir" -c advice.detachedHead=false checkout --quiet --detach FETCH_HEAD
else
  git clone --quiet --depth 1 "$doom_repository" "$doom_emacs_dir"
fi

resolved_doom_revision="$(git -C "$doom_emacs_dir" rev-parse HEAD)"
echo "Testing Doom $resolved_doom_revision with $neomacs_bin"

env \
  -u DOOMDIR \
  -u EMACSLOADPATH \
  -u EMACSNATIVELOADPATH \
  -u EMACSDIR \
  HOME="$home_dir" \
  XDG_CONFIG_HOME="$xdg_config_home" \
  XDG_CACHE_HOME="$home_dir/.cache" \
  XDG_DATA_HOME="$home_dir/.local/share" \
  XDG_STATE_HOME="$home_dir/.local/state" \
  EMACS="$neomacs_bin" \
  EMACSDIR="$doom_emacs_dir" \
  DOOMDIR="$doom_user_dir" \
  NEOMACS_RUNTIME_ROOT="$runtime_root" \
  "$doom_emacs_dir/bin/doom" --force install

for config_file in init.el config.el packages.el; do
  if [[ ! -f "$doom_user_dir/$config_file" ]]; then
    echo "doom install did not create $doom_user_dir/$config_file" >&2
    exit 1
  fi
done

echo "Doom install compatibility passed for $resolved_doom_revision"
