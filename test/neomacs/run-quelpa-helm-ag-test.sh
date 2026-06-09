#!/usr/bin/env bash
# Test use-package + quelpa installing helm-ag from GitHub over HTTPS
# Two-phase load: bootstrap quelpa-use-package first, then use-package :quelpa
# Usage: ./test/neomacs/run-quelpa-helm-ag-test.sh
set -e
cd "$(git rev-parse --show-toplevel)"
TMPHOME="$(mktemp -d "${TMPDIR:-/tmp}/neomacs-quelpa-test.XXXXXX")"
export HOME="$TMPHOME"
cleanup() { rm -rf "$TMPHOME"; }
trap cleanup EXIT
exec ./target/release/neomacs -Q -l test/neomacs/quelpa-helm-ag-bootstrap.el -l test/neomacs/quelpa-helm-ag-test.el
