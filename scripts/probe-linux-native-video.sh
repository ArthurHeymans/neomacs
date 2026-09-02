#!/usr/bin/env bash
# Verify Linux's observed decoder -> DMA-BUF -> wgpu presentation path.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
neomacs_bin=${NEOMACS_BIN:-"$repo_root/target/release/neomacs"}
probe_el="$repo_root/test/neomacs/native-video-path-probe.el"
video_file=${1:-${NEOMACS_VIDEO_PROBE_FILE:-}}
probe_dir="$repo_root/target/neomacs-video-probe"

if [[ $(uname -s) != Linux ]]; then
    echo "error: this acceptance probe verifies Linux DMA-BUF import" >&2
    exit 2
fi
if [[ -z $video_file || ! -r $video_file ]]; then
    echo "usage: $0 VIDEO_FILE" >&2
    echo "       NEOMACS_VIDEO_PROBE_FILE=VIDEO_FILE $0" >&2
    exit 2
fi
if [[ ! -x $neomacs_bin ]]; then
    echo "error: release binary not found: $neomacs_bin" >&2
    echo "run: cargo xtask fresh-build --release" >&2
    exit 2
fi
if ! "$neomacs_bin" --version | grep -Eq '^Build: release(/bench)? '; then
    echo "error: native video probes must use a release Neomacs build" >&2
    exit 2
fi
if [[ -z ${DISPLAY:-} && -z ${WAYLAND_DISPLAY:-} ]]; then
    echo "error: a graphical session is required to submit the video frame" >&2
    exit 2
fi

mkdir -p "$probe_dir"
log_file="$probe_dir/linux-native-video.log"
result_file="$probe_dir/linux-native-video.result"

export NEOMACS_VIDEO_PROBE_FILE
NEOMACS_VIDEO_PROBE_FILE=$(realpath "$video_file")
export NEOMACS_VIDEO_PROBE_RESULT_FILE=$result_file
export RUST_LOG=${RUST_LOG:-info}

rm -f "$result_file"

echo "release binary: $neomacs_bin"
echo "video input: $NEOMACS_VIDEO_PROBE_FILE"
echo "probe log: $log_file"

set +e
timeout "${NEOMACS_VIDEO_PROBE_PROCESS_TIMEOUT:-30}" \
    "$neomacs_bin" -Q -l "$probe_el" >"$log_file" 2>&1
process_status=$?
set -e

if [[ -s $log_file ]]; then
    cat "$log_file"
fi
if [[ -s $result_file ]]; then
    cat "$result_file"
else
    echo "NEOMACS_VIDEO_PROBE_RESULT FAIL process-exited-without-result status=$process_status"
fi

[[ $process_status -eq 0 ]] &&
    grep -q '^NEOMACS_VIDEO_PROBE_RESULT PASS dma-buf-zero-copy$' "$result_file"
