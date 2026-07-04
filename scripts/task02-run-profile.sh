#!/usr/bin/env bash
# task02-run-profile.sh --- build + run the JIT-intrinsics-round-2 profiling
# session (task 02 §3(a)). Produces the SUBR-MIX of a REAL interactive editing
# session (buffer/point/search/match/edit/indent/replace over a large .el
# buffer, with font-lock), with the Op::Call-vs-CallBuiltinSym entry split.
#
# Usage:
#   scripts/task02-run-profile.sh [--skip-build] [OUTFILE]
#
# NEOVM_JIT=0 is set automatically (a tiered body bypasses the profiler bump
# sites). The build threads the `vm-profile` feature into neomacs-bin so the
# binary carries the profiler + the neovm--vm-profile-{reset,dump} subrs.
set -euo pipefail
cd "$(dirname "$0")/.."

SKIP_BUILD=0
OUT="tmp/task02/session-subr-mix.log"
for a in "$@"; do
  case "$a" in
    --skip-build) SKIP_BUILD=1 ;;
    *) OUT="$a" ;;
  esac
done
mkdir -p "$(dirname "$OUT")"

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  # taskset bounds parallelism on a loaded shared box (avoids OOM); drop it if
  # you have the machine to yourself. fresh-build threads the feature via
  # `--features vm-profile` (see initial_cargo_build_args).
  taskset -c 0-7 cargo xtask fresh-build --release --features vm-profile
fi

BIN="${NEOMACS_BIN:-target/release/neomacs}"
echo "== running profiling session with $BIN (NEOVM_JIT=0) =="
NEOVM_JIT=0 "$BIN" --batch -l scripts/task02-profile-session.el 2>&1 | tee "$OUT"
echo "== dump written to $OUT =="
