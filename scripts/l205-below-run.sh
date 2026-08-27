#!/usr/bin/env bash
# Ledger 205: run a below-content audit .el in one editor under a pty.
#
#   scripts/l205-below-run.sh EDITOR AUDIT.el REDISPLAYS OUTFILE [COLS] [ROWS]
#
# Same as scripts/below-content-run.sh but with the audit file as an argument,
# so the 14-case script this ledger's RED commit shipped can be re-run against
# the post-fix binary on exactly the basis the pre-fix numbers were taken on.
set -u
editor="$1"
audit="$2"
redisplays="$3"
out="$4"
cols="${5:-80}"
rows="${6:-24}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root" || exit 1
mkdir -p "$(dirname "$out")"
rm -f "$out"
export RUST_LOG=error
export L195_COLS="$cols"
export L195_ROWS="$rows"
export L195_TIMEOUT="${L195_TIMEOUT:-300}"
export L205_REDISPLAY="$redisplays"
export L205_OUT="$out"
python3 scripts/motion-parity-pty.py "$editor" -nw -Q -l "$audit" > "$out.pty.log" 2>&1
status=$?
echo "pty exit=$status out=$out lines=$( [ -f "$out" ] && wc -l < "$out" || echo MISSING )"
exit "$status"
