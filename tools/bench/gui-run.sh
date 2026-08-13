#!/usr/bin/env bash
# Deterministic GUI-session runner for benchmarks and reproducers.
#
# Runs a neomacs GUI session under headless Weston and waits for a sentinel
# FILE, the same discipline as tools/bench/pty-run.py: completion is proven by
# an artifact the fixture writes, never inferred from output or exit codes,
# and not finishing is a loud non-zero exit.
#
# The one GUI-specific hazard is a startup race: the Wayland surface handshake
# intermittently fails with ERROR_SURFACE_LOST_KHR (~50% of launches,
# documented in the frame-scheduling notes). That is a LAUNCH flake, not a
# measurement flake, so the runner retries the whole attempt -- fresh Weston,
# fresh socket -- and reports how many attempts it took. A measurement is only
# valid if the fixture's own completion counter says the workload finished.
#
#   SENTINEL=path GUI_TIMEOUT=secs ATTEMPTS=n \
#     tools/bench/gui-run.sh ./target/release/neomacs -Q -l fixture.el
set -u
mkdir -p ./tmp/gui-logs
SENTINEL=${SENTINEL:?set SENTINEL to the fixture completion file}
GUI_TIMEOUT=${GUI_TIMEOUT:-180}
ATTEMPTS=${ATTEMPTS:-5}
GUI_WIDTH=${GUI_WIDTH:-1200}
GUI_HEIGHT=${GUI_HEIGHT:-800}
GUI_WESTON_LOG=${GUI_WESTON_LOG:-./tmp/gui-logs/weston-$$.log}
GUI_APP_LOG=${GUI_APP_LOG:-./tmp/gui-logs/neomacs-gui-$$.log}
BIN=$1; shift

for attempt in $(seq 1 "$ATTEMPTS"); do
  rm -f "$SENTINEL"
  SOCKET="nm-bench-$$-$attempt"
  weston --backend=headless --renderer=${WESTON_RENDERER:-pixman} \
    --width="$GUI_WIDTH" --height="$GUI_HEIGHT" --socket="$SOCKET" \
    >"$GUI_WESTON_LOG" 2>&1 &
  WESTON=$!
  # Wait for the compositor socket rather than sleeping a guess.
  for _ in $(seq 1 50); do
    [ -S "$XDG_RUNTIME_DIR/$SOCKET" ] && break
    sleep 0.1
  done
  if [ ! -S "$XDG_RUNTIME_DIR/$SOCKET" ]; then
    kill "$WESTON" 2>/dev/null; wait "$WESTON" 2>/dev/null
    echo "gui-run: weston socket never appeared (attempt $attempt)" >&2
    continue
  fi

  # GUI_PERF_OUT wraps ONLY the app in perf stat (cycles+instructions,
  # all app threads: Lisp, render, wpe). Wrapping the whole script would
  # also count Weston's compositing, muddying attribution.
  if [ -n "${GUI_PERF_RECORD:-}" ]; then
    WAYLAND_DISPLAY="$SOCKET" timeout "$GUI_TIMEOUT"       taskset -c 0-15 perf record -F 999 --call-graph=lbr -o "$GUI_PERF_RECORD"       "$BIN" "$@" >"$GUI_APP_LOG" 2>&1
  elif [ -n "${GUI_PERF_OUT:-}" ]; then
    WAYLAND_DISPLAY="$SOCKET" timeout "$GUI_TIMEOUT"       taskset -c 0-15 perf stat -o "$GUI_PERF_OUT" -e cycles:u,instructions:u       "$BIN" "$@" >"$GUI_APP_LOG" 2>&1
  else
    WAYLAND_DISPLAY="$SOCKET" timeout "$GUI_TIMEOUT" "$BIN" "$@"       >"$GUI_APP_LOG" 2>&1
  fi
  APP_RC=$?
  kill "$WESTON" 2>/dev/null; wait "$WESTON" 2>/dev/null

  if [ -f "$SENTINEL" ]; then
    echo "SENTINEL_WRITTEN attempts=$attempt"
    exit 0
  fi
  echo "gui-run: attempt $attempt failed (app rc=$APP_RC); app log tail:" >&2
  tail -3 "$GUI_APP_LOG" >&2
done
echo "GUI-RUN-INCOMPLETE: sentinel $SENTINEL never appeared in $ATTEMPTS attempts" >&2
exit 2
