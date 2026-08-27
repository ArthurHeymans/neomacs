#!/usr/bin/env bash
# Ledger 205: run one parity audit .el in one editor under a pty.
#
#   scripts/l205-audit-run.sh EDITOR AUDIT.el OUT_ENV OUTFILE REDISPLAY_ENV N COLS ROWS
#
# e.g.  scripts/l205-audit-run.sh emacs scripts/posn-parity-audit.el \
#         L201_OUT ./tmp/l205/p201-gnu-warm.txt L201_REDISPLAY 1 160 50
set -u
editor="$1"
audit="$2"
out_env="$3"
out="$4"
red_env="$5"
red="$6"
cols="${7:-160}"
rows="${8:-50}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root" || exit 1
mkdir -p "$(dirname "$out")"
rm -f "$out"
export RUST_LOG=error
export L195_COLS="$cols"
export L195_ROWS="$rows"
export L195_TIMEOUT="${L195_TIMEOUT:-300}"
export "$out_env=$out"
export "$red_env=$red"
python3 scripts/motion-parity-pty.py "$editor" -nw -Q -l "$audit" > "$out.pty.log" 2>&1
status=$?
echo "pty exit=$status out=$out lines=$( [ -f "$out" ] && wc -l < "$out" || echo MISSING )"
exit "$status"
