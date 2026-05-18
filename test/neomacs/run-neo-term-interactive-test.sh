#!/usr/bin/env bash
# Interactive neo-term test using xdotool
# Usage: ./test/neomacs/run-neo-term-interactive-test.sh
#
# Tests:
#   1. Terminal creation (window mode)
#   2. Keyboard input and text output
#   3. ANSI color rendering
#   4. Terminal resize
#   5. Floating terminal overlay
#   6. Cleanup and destruction
#
# Requires: xdotool, Xvfb (or existing DISPLAY)

set -e

cd "$(dirname "$0")/../.."

LOG=/tmp/neo-term-interactive.log
RESULTS=/tmp/neo-term-interactive-results.txt
SCREENSHOT_DIR=/tmp/neo-term-screenshots
TEST_NAME="Neo-Term Interactive"
XVFB_PID=""

# Cleanup from previous runs
rm -f /tmp/neo-term-phase{2,3,4,5,6}
rm -f "$RESULTS"
mkdir -p "$SCREENSHOT_DIR"

# Setup LD_LIBRARY_PATH for nix-based systems
setup_lib_path() {
    local xcursor_so=""
    local xkb_so=""
    xcursor_so=$(find /nix/store -maxdepth 3 -name 'libXcursor.so.1' 2>/dev/null | head -1)
    xkb_so=$(find /nix/store -maxdepth 3 -name 'libxkbcommon-x11.so' 2>/dev/null | head -1)

    local extra_path=""
    if [ -n "$xcursor_so" ]; then
        extra_path="$(dirname "$xcursor_so")"
    fi
    if [ -n "$xkb_so" ]; then
        extra_path="${extra_path:+$extra_path:}$(dirname "$xkb_so")"
    fi

    if [ -n "$extra_path" ]; then
        export LD_LIBRARY_PATH="${extra_path}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        echo "LD_LIBRARY_PATH set: $LD_LIBRARY_PATH"
    fi
}

# Setup display (start Xvfb if needed)
setup_display() {
    local test_display="${DISPLAY:-}"

    # If no DISPLAY or the display isn't accessible, start Xvfb
    if [ -z "$test_display" ] || ! DISPLAY="$test_display" xdotool getactivewindow >/dev/null 2>&1; then
        echo "Starting Xvfb on :99..."
        rm -f /tmp/.X99-lock
        kill $(pgrep -f "Xvfb :99") 2>/dev/null || true
        sleep 1
        Xvfb :99 -screen 0 1920x1080x24 -ac 2>/dev/null &
        XVFB_PID=$!
        sleep 2
        if ! kill -0 $XVFB_PID 2>/dev/null; then
            echo "ERROR: Failed to start Xvfb"
            exit 1
        fi
        export DISPLAY=:99
        echo "Xvfb started (PID $XVFB_PID) on $DISPLAY"
    else
        echo "Using existing DISPLAY=$test_display"
        export DISPLAY="$test_display"
    fi
}

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up..."
    kill $EMACS_PID 2>/dev/null || true
    wait $EMACS_PID 2>/dev/null || true
    if [ -n "$XVFB_PID" ]; then
        kill $XVFB_PID 2>/dev/null || true
    fi
    rm -f /tmp/neo-term-phase{2,3,4,5,6}
}
trap cleanup EXIT

echo "=== $TEST_NAME Test ==="

setup_lib_path
setup_display

echo "Starting Neomacs with neo-term interactive test..."

RUST_LOG=neomacs_display=debug ./target/release/neomacs -Q \
    -l test/neomacs/neo-term-interactive-test.el 2>"$LOG" &
EMACS_PID=$!

echo "Neomacs PID: $EMACS_PID"

# Helper: wait for Emacs window
wait_for_window() {
    local attempts=0
    local max_attempts=30
    while [ $attempts -lt $max_attempts ]; do
        if ! kill -0 $EMACS_PID 2>/dev/null; then
            echo "ERROR: Emacs process died"
            echo "--- Last 20 lines of log ---"
            tail -20 "$LOG" 2>/dev/null || true
            return 1
        fi
        WIN_ID=$(xdotool search --class "neomacs" 2>/dev/null | head -1)
        if [ -n "$WIN_ID" ]; then
            echo "Found Emacs window: $WIN_ID"
            return 0
        fi
        sleep 0.5
        attempts=$((attempts + 1))
    done
    echo "ERROR: Emacs window not found after ${max_attempts} attempts"
    echo "--- Last 20 lines of log ---"
    tail -20 "$LOG" 2>/dev/null || true
    return 1
}

# Helper: take a screenshot with label
take_screenshot() {
    local label=$1
    local file="$SCREENSHOT_DIR/neo-term-${label}.png"
    if command -v import &>/dev/null && [ -n "$WIN_ID" ]; then
        import -window "$WIN_ID" "$file" 2>/dev/null && \
            echo "  Screenshot: $file" || echo "  Screenshot failed"
    fi
}

# Helper: wait for results file to contain a marker
wait_for_marker() {
    local marker=$1
    local timeout=${2:-15}
    local elapsed=0
    while [ $elapsed -lt $timeout ]; do
        if ! kill -0 $EMACS_PID 2>/dev/null; then
            echo "  WARNING: Emacs process died while waiting for '$marker'"
            return 1
        fi
        if [ -f "$RESULTS" ] && grep -q "$marker" "$RESULTS" 2>/dev/null; then
            return 0
        fi
        sleep 0.5
        elapsed=$((elapsed + 1))
    done
    echo "  WARNING: Timed out waiting for '$marker' (${timeout}s)"
    return 1
}

# Helper: send keystrokes with delay
type_text() {
    local text=$1
    local delay=${2:-50}
    xdotool type --window "$WIN_ID" --delay "$delay" "$text"
}

send_key() {
    local key=$1
    xdotool key --window "$WIN_ID" "$key"
}

# Wait for window
echo ""
echo "--- Waiting for Emacs window ---"
if ! wait_for_window; then
    exit 1
fi

# Activate and focus (may fail on Xvfb without a window manager, that's ok)
xdotool windowactivate --sync "$WIN_ID" 2>/dev/null || true
xdotool windowfocus --sync "$WIN_ID" 2>/dev/null || true
sleep 1

# Wait for terminal to be ready
echo ""
echo "--- Phase 1: Waiting for terminal creation ---"
if ! wait_for_marker "READY_FOR_INPUT" 15; then
    echo "ERROR: Terminal not ready"
    cat "$RESULTS" 2>/dev/null || true
    exit 1
fi
echo "  Terminal is ready"
take_screenshot "01-initial"
sleep 2

# Phase 2: Type a command
echo ""
echo "--- Phase 2: Typing 'echo hello' ---"
type_text "echo hello"
sleep 0.5
send_key "Return"
sleep 2
touch /tmp/neo-term-phase2
wait_for_marker "Phase 2" 10
take_screenshot "02-echo-hello"

# Phase 3: ANSI color output
echo ""
echo "--- Phase 3: ANSI color test ---"
sleep 1
type_text "printf '\\033[31mRED\\033[32mGREEN\\033[34mBLUE\\033[0m COLOR_TEST\\n'"
sleep 0.5
send_key "Return"
sleep 2
touch /tmp/neo-term-phase3
wait_for_marker "Phase 3" 10
take_screenshot "03-ansi-colors"

# Phase 4: Resize test (driven by Elisp)
echo ""
echo "--- Phase 4: Resize test ---"
sleep 1
touch /tmp/neo-term-phase4
wait_for_marker "Phase 4" 10
take_screenshot "04-after-resize"

# Phase 5: Floating terminal
echo ""
echo "--- Phase 5: Floating terminal ---"
sleep 1
touch /tmp/neo-term-phase5
wait_for_marker "Phase 5" 10
take_screenshot "05-floating"

# Phase 6: Cleanup
echo ""
echo "--- Phase 6: Cleanup ---"
touch /tmp/neo-term-phase6
wait_for_marker "DONE" 10
take_screenshot "06-final"

sleep 1

# Read and display results
echo ""
echo "=== Test Results ==="
if [ -f "$RESULTS" ]; then
    cat "$RESULTS"
else
    echo "WARNING: Results file not found"
fi

# Check for panics in Rust log
echo ""
echo "=== Log Analysis ==="
PANIC_COUNT=0
ERROR_COUNT=0
if [ -f "$LOG" ]; then
    PANIC_COUNT=$(grep -ci "panic" "$LOG" 2>/dev/null | head -1 || true)
    PANIC_COUNT=${PANIC_COUNT:-0}
    # Filter out known non-fatal errors
    ERROR_COUNT=$(grep -ci "error" "$LOG" 2>/dev/null | head -1 || true)
    ERROR_COUNT=${ERROR_COUNT:-0}
    echo "PANIC entries: $PANIC_COUNT"
    echo "ERROR entries: $ERROR_COUNT (includes non-fatal warnings)"

    if [ "$PANIC_COUNT" -gt 0 ]; then
        echo ""
        echo "--- PANIC log entries ---"
        grep -i "panic" "$LOG" | tail -10
    fi
else
    echo "Log file not found"
fi

# Final summary
echo ""
echo "=== Summary ==="
if [ -f "$RESULTS" ]; then
    PASS_COUNT=$(grep -c "^PASS:" "$RESULTS" 2>/dev/null || true)
    FAIL_COUNT=$(grep -c "^FAIL:" "$RESULTS" 2>/dev/null || true)
    echo "Tests passed: $PASS_COUNT"
    echo "Tests failed: $FAIL_COUNT"
    echo "Panics: $PANIC_COUNT"

    if [ "$FAIL_COUNT" -gt 0 ]; then
        echo ""
        echo "Failed tests:"
        grep "^FAIL:" "$RESULTS"
    fi

    if [ "$FAIL_COUNT" -gt 0 ] || [ "$PANIC_COUNT" -gt 0 ]; then
        echo ""
        echo "RESULT: FAIL"
        exit 1
    else
        echo ""
        echo "RESULT: PASS"
    fi
else
    echo "RESULT: UNKNOWN (no results file)"
    exit 1
fi

echo ""
echo "Screenshots: $SCREENSHOT_DIR/"
echo "Full log: $LOG"
echo "Test results: $RESULTS"
