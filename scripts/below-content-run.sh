#!/usr/bin/env bash
# Ledger 205: run scripts/below-content-audit.el in one editor under a pty.
#
#   scripts/below-content-run.sh EDITOR REDISPLAYS OUTFILE [COLS] [ROWS]
#
# EDITOR is an absolute path or a name on PATH.  REDISPLAYS is L205_REDISPLAY
# (0 = COLD, 1 = WARM).  The pty geometry defaults to 80x24, which is the
# geometry ledger 204's residual 1 was measured in.
set -u
editor="$1"
redisplays="$2"
out="$3"
cols="${4:-80}"
rows="${5:-24}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root" || exit 1
mkdir -p "$(dirname "$out")"
rm -f "$out"
unset RUST_LOG
export RUST_LOG=error
export L195_COLS="$cols"
export L195_ROWS="$rows"
export L195_TIMEOUT="${L195_TIMEOUT:-240}"
export L205_REDISPLAY="$redisplays"
export L205_OUT="$out"
python3 scripts/motion-parity-pty.py "$editor" -nw -Q -l scripts/below-content-audit.el \
  > "$out.pty.log" 2>&1
status=$?
echo "pty exit=$status out=$out lines=$( [ -f "$out" ] && wc -l < "$out" || echo MISSING )"
exit "$status"
