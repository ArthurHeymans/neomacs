#!/usr/bin/env bash
# Test inline video rendering in Neomacs
#
# This script:
# 1. Launches neomacs with video test
# 2. Captures GPU rendering logs
# 3. Takes a screenshot

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NEOMACS_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
EMACS_BIN="$NEOMACS_ROOT/target/release/neomacs"
TEST_EL="$SCRIPT_DIR/neomacs-video-test.el"
ARTIFACT_DIR="$NEOMACS_ROOT/target/neomacs-video-test"
LOG_FILE="$ARTIFACT_DIR/neomacs-video-test-$$.log"
SCREENSHOT_FILE="$ARTIFACT_DIR/neomacs-video-test-screenshot-$$.png"
mkdir -p "$ARTIFACT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Neomacs Video Rendering Test ==="
echo ""

# Check emacs binary
if [[ ! -x "$EMACS_BIN" ]]; then
    echo -e "${RED}ERROR: Neomacs binary not found at $EMACS_BIN${NC}"
    echo "Run: cargo xtask fresh-build --release"
    exit 1
fi

# Check test file
if [[ ! -f "$TEST_EL" ]]; then
    echo -e "${RED}ERROR: Test elisp file not found at $TEST_EL${NC}"
    exit 1
fi

# Check for test video
VIDEO_FILE="${NEOMACS_VIDEO_TEST_FILE:-/home/exec/Videos/4k_f1.mp4}"
if [[ ! -f "$VIDEO_FILE" ]]; then
    echo -e "${RED}ERROR: Test video not found at $VIDEO_FILE${NC}"
    exit 1
fi

echo "Neomacs binary: $EMACS_BIN"
echo "Test file: $TEST_EL"
echo "Log file: $LOG_FILE"
echo ""

# Enable video debug logs
export RUST_LOG="neomacs_renderer_wgpu::video_cache=debug,neomacs_video=debug,info"

echo "Running test with RUST_LOG=$RUST_LOG"
echo ""

# Run emacs and capture logs
NEOMACS_VIDEO_TEST_FILE="$VIDEO_FILE" timeout 15 "$EMACS_BIN" -Q \
    --eval "(setq inhibit-startup-screen t)" \
    -l "$TEST_EL" \
    2>&1 | tee "$LOG_FILE" &

EMACS_PID=$!

# Wait for window to appear
sleep 3

# Take screenshot
if command -v import &> /dev/null && [[ -n "$DISPLAY" ]]; then
    echo "Taking screenshot..."
    WINDOW_ID=$(xdotool search --class "neomacs" 2>/dev/null | head -1 || true)
    if [[ -n "$WINDOW_ID" ]]; then
        import -window "$WINDOW_ID" "$SCREENSHOT_FILE" 2>/dev/null || true
        if [[ -f "$SCREENSHOT_FILE" ]]; then
            echo -e "${GREEN}Screenshot saved: $SCREENSHOT_FILE${NC}"
        fi
    fi
fi

# Wait for Neomacs and preserve failures from Lisp, the native backend, or the
# timeout.  The old smoke test discarded this status and could report success
# when every video native function was absent.
set +e
wait "$EMACS_PID"
NEOMACS_STATUS=$?
set -e
if [[ $NEOMACS_STATUS -ne 0 ]]; then
    echo -e "${RED}VIDEO TEST: Neomacs exited with status $NEOMACS_STATUS${NC}"
    exit "$NEOMACS_STATUS"
fi

echo ""
echo "=== Analyzing logs ==="
echo ""

# Check for video-related logs
VIDEO_SUCCESS=false

if grep -q "Video cache initialized" "$LOG_FILE" 2>/dev/null; then
    echo -e "${GREEN}[INFO] VideoCache activity detected${NC}"
    VIDEO_SUCCESS=true
fi

if grep -q "GStreamer" "$LOG_FILE" 2>/dev/null; then
    echo -e "${GREEN}[INFO] GStreamer pipeline created${NC}"
    VIDEO_SUCCESS=true
fi

if grep -q "opening video\|video open queued" "$LOG_FILE" 2>/dev/null; then
    echo -e "${GREEN}[INFO] Video loading detected${NC}"
    VIDEO_SUCCESS=true
fi

if grep -q "DMA-BUF\|dmabuf\|DmaBuf" "$LOG_FILE" 2>/dev/null; then
    echo -e "${GREEN}[INFO] DMA-BUF activity detected${NC}"
fi

# Backend/session failures are fatal.  General Neomacs diagnostics remain
# visible below without making unrelated warnings fail this focused smoke test.
if grep -Eqi "native video subsystem is unavailable|video playback failed|panic" "$LOG_FILE"; then
    echo -e "${RED}VIDEO TEST: Native video backend failed${NC}"
    grep -Ei "native video subsystem is unavailable|video playback failed|panic" "$LOG_FILE" | head -10
    exit 1
fi

if grep -qi "error\|failed\|panic" "$LOG_FILE" 2>/dev/null; then
    echo -e "${YELLOW}[WARN] Some errors in log:${NC}"
    grep -i "error\|failed\|panic" "$LOG_FILE" | head -10
fi

echo ""
echo "=== Summary ==="

if [[ "$VIDEO_SUCCESS" == "true" ]]; then
    echo -e "${GREEN}VIDEO TEST: Activity detected${NC}"
else
    echo -e "${RED}VIDEO TEST: No video session activity in logs${NC}"
    exit 1
fi

echo ""
echo "Full log saved to: $LOG_FILE"
if [[ -f "$SCREENSHOT_FILE" ]]; then
    echo "Screenshot saved to: $SCREENSHOT_FILE"
fi

# Show relevant log lines
echo ""
echo "=== Relevant log lines ==="
grep -E "(video|Video|gstreamer|GStreamer|DMA|dmabuf)" "$LOG_FILE" 2>/dev/null | head -20 || echo "(no video-related output)"

exit 0
