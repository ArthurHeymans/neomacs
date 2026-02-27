#!/usr/bin/env bash
# Reproduce CJK cursor alignment behavior and save a screenshot.
# Usage: ./test/neomacs/repro-cjk-cursor.sh

set -euo pipefail

cd "$(dirname "$0")/../.."

LOG="/tmp/cjk-cursor-repro.log"
SCREENSHOT_CJK="/tmp/cjk-cursor-repro-cjk.png"
SCREENSHOT_MOVED="/tmp/cjk-cursor-repro-moved.png"
XVFB_PID=""
EMACS_PID=""

setup_lib_path() {
    local xcursor_so=""
    local xkb_so=""
    xcursor_so=$(find /nix/store -maxdepth 3 -name 'libXcursor.so.1' 2>/dev/null | head -1 || true)
    xkb_so=$(find /nix/store -maxdepth 3 -name 'libxkbcommon-x11.so' 2>/dev/null | head -1 || true)

    local extra_path=""
    if [ -n "$xcursor_so" ]; then
        extra_path="$(dirname "$xcursor_so")"
    fi
    if [ -n "$xkb_so" ]; then
        extra_path="${extra_path:+$extra_path:}$(dirname "$xkb_so")"
    fi

    if [ -n "$extra_path" ]; then
        export LD_LIBRARY_PATH="${extra_path}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    fi
}

setup_display() {
    local current_display="${DISPLAY:-}"
    if [ -z "$current_display" ] || ! DISPLAY="$current_display" xdotool getactivewindow >/dev/null 2>&1; then
        rm -f /tmp/.X99-lock
        kill $(pgrep -f "Xvfb :99") 2>/dev/null || true
        sleep 1
        Xvfb :99 -screen 0 1920x1080x24 -ac >/tmp/cjk-cursor-repro-xvfb.log 2>&1 &
        XVFB_PID=$!
        sleep 2
        if ! kill -0 "$XVFB_PID" 2>/dev/null; then
            echo "ERROR: failed to start Xvfb on :99"
            exit 1
        fi
        export DISPLAY=:99
    else
        export DISPLAY="$current_display"
    fi
}

cleanup() {
    if [ -n "$EMACS_PID" ]; then
        kill "$EMACS_PID" 2>/dev/null || true
        wait "$EMACS_PID" 2>/dev/null || true
    fi
    if [ -n "$XVFB_PID" ]; then
        kill "$XVFB_PID" 2>/dev/null || true
        wait "$XVFB_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

echo "=== CJK Cursor Repro ==="

if [ ! -x ./src/emacs ]; then
    echo "ERROR: ./src/emacs not found. Build neomacs first."
    exit 1
fi

for bin in xdotool import Xvfb; do
    if ! command -v "$bin" >/dev/null 2>&1; then
        echo "ERROR: missing dependency: $bin"
        exit 1
    fi
done

rm -f "$LOG" "$SCREENSHOT_CJK" "$SCREENSHOT_MOVED"
setup_lib_path
setup_display

RUST_LOG=neomacs_display=debug ./src/emacs -Q \
    -l test/neomacs/cjk-cursor-repro.el 2>"$LOG" &
EMACS_PID=$!

sleep 6

if ! kill -0 "$EMACS_PID" 2>/dev/null; then
    echo "ERROR: emacs exited early"
    tail -n 80 "$LOG" || true
    exit 1
fi

WIN_ID=$(xdotool search --name "CJK Cursor Repro|emacs" 2>/dev/null | head -1 || true)
if [ -z "${WIN_ID:-}" ]; then
    echo "ERROR: failed to find Emacs window"
    tail -n 80 "$LOG" || true
    exit 1
fi

xdotool windowactivate --sync "$WIN_ID" 2>/dev/null || true
xdotool windowfocus --sync "$WIN_ID" 2>/dev/null || true
sleep 1

import -window "$WIN_ID" "$SCREENSHOT_CJK" 2>/dev/null || true

# Drive cursor through mixed-width content before capture.
xdotool key --window "$WIN_ID" Home
sleep 0.1
for _ in $(seq 1 14); do
    xdotool key --window "$WIN_ID" Right
    sleep 0.06
done
sleep 1

import -window "$WIN_ID" "$SCREENSHOT_MOVED" 2>/dev/null || true

if [ ! -f "$SCREENSHOT_CJK" ]; then
    echo "ERROR: initial screenshot capture failed"
    tail -n 80 "$LOG" || true
    exit 1
fi

echo "Screenshot (cursor-on-CJK): $SCREENSHOT_CJK"
if [ -f "$SCREENSHOT_MOVED" ]; then
    echo "Screenshot (cursor-moved): $SCREENSHOT_MOVED"
fi
echo "Log: $LOG"
