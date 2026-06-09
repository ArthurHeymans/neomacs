#!/usr/bin/env bash
# Test HTTPS package archive access via rustls TLS
# Usage: ./test/neomacs/run-package-archive-test.sh
set -e
cd "$(git rev-parse --show-toplevel)"
TMPHOME="$(mktemp -d "${TMPDIR:-/tmp}/neomacs-pkgtest.XXXXXX")"
export HOME="$TMPHOME"
cleanup() { rm -rf "$TMPHOME"; }
trap cleanup EXIT
exec ./target/release/neomacs -Q -l test/neomacs/package-archive-test.el
