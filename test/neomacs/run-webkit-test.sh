#!/usr/bin/env bash
# Test script for Neomacs inline WebKit rendering
#
# This tests that WPE WebKit views render inline in Emacs buffers
# using GPU acceleration.
#
# Usage: ./test/neomacs/run-webkit-test.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EMACS_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$EMACS_DIR"

if [ ! -x "./target/release/neomacs" ]; then
    echo "Error: ./target/release/neomacs not found. Run: cargo xtask fresh-build --release."
    exit 1
fi

# Check for display
if [ -z "$DISPLAY" ]; then
    export DISPLAY=:0
fi

echo "=== Neomacs Inline WebKit Test ==="
echo "Emacs: $EMACS_DIR/target/release/neomacs"
echo "Display: $DISPLAY"
echo ""

# Check for bubblewrap - if not available, disable sandbox
if ! command -v bwrap &> /dev/null; then
    echo "Warning: bubblewrap (bwrap) not found - disabling WebKit sandbox"
    export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
fi

# Check for xdg-dbus-proxy
if ! command -v xdg-dbus-proxy &> /dev/null; then
    echo "Warning: xdg-dbus-proxy not found - disabling WebKit sandbox"
    export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
fi

# Run the test
exec ./target/release/neomacs -Q -l test/neomacs/neomacs-webkit-test.el "$@"
