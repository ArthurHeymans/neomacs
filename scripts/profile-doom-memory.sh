#!/usr/bin/env bash
# Compare Neomacs' Linux mimalloc policy against mimalloc's eager-commit
# default while loading the user's real Doom configuration.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
neomacs_bin="${NEOMACS_BIN:-$repo_root/target/release/neomacs}"
runs="${RUNS:-5}"
settle_seconds="${SETTLE_SECONDS:-30}"
startup_timeout_seconds="${STARTUP_TIMEOUT_SECONDS:-180}"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "this profiler reads Linux /proc memory metrics" >&2
  exit 1
fi
if [[ ! -x "$neomacs_bin" ]]; then
  echo "neomacs executable not found: $neomacs_bin" >&2
  exit 1
fi
if ! command -v script >/dev/null 2>&1; then
  echo "the util-linux 'script' command is required to provide a TTY" >&2
  exit 1
fi
if [[ ! "$runs" =~ ^[1-9][0-9]*$ ]]; then
  echo "RUNS must be a positive integer" >&2
  exit 1
fi

neomacs_bin="$(cd "$(dirname "$neomacs_bin")" && pwd)/$(basename "$neomacs_bin")"
mkdir -p "$repo_root/target/profiling"
work_dir="$(mktemp -d "$repo_root/target/profiling/doom-memory.XXXXXX")"
report="${REPORT:-$repo_root/target/profiling/doom-memory.tsv}"
active_pid=""
active_script_pid=""

cleanup_process() {
  if [[ -n "$active_pid" ]] && kill -0 "$active_pid" 2>/dev/null; then
    kill "$active_pid" 2>/dev/null || true
    for _ in {1..20}; do
      kill -0 "$active_pid" 2>/dev/null || break
      sleep 0.1
    done
    if kill -0 "$active_pid" 2>/dev/null; then
      kill -KILL "$active_pid" 2>/dev/null || true
    fi
  fi
  if [[ -n "$active_script_pid" ]]; then
    wait "$active_script_pid" 2>/dev/null || true
  fi
  active_pid=""
  active_script_pid=""
}

cleanup() {
  cleanup_process
  rm -rf -- "$work_dir"
}
trap cleanup EXIT INT TERM

metric() {
  local name="$1"
  local file="$2"
  awk -v name="$name" '$1 == name ":" { print $2; exit }' "$file"
}

run_sample() {
  local mode="$1"
  local pair="$2"
  local marker="$work_dir/$mode-$pair.ready"
  local log="$work_dir/$mode-$pair.log"
  local marker_elisp="$marker"
  local form command quoted_bin quoted_form
  local start_ns deadline now_ns startup_ms smaps snapshot

  marker_elisp="${marker_elisp//\\/\\\\}"
  marker_elisp="${marker_elisp//\"/\\\"}"
  form="(progn (garbage-collect) (with-temp-file \"$marker_elisp\" (insert \"ready\")))"
  printf -v quoted_bin '%q' "$neomacs_bin"
  printf -v quoted_form '%q' "$form"

  # Keep ambient tracing configuration from becoming part of the workload.
  command="exec env -u MIMALLOC_ARENA_EAGER_COMMIT RUST_LOG=warn"
  if [[ "$mode" == "eager-commit" ]]; then
    command+=" MIMALLOC_ARENA_EAGER_COMMIT=2"
  fi
  command+=" $quoted_bin -nw --eval $quoted_form"

  start_ns="$(date +%s%N)"
  script --quiet --return --command "$command" /dev/null >"$log" 2>&1 &
  active_script_pid=$!
  deadline=$((SECONDS + startup_timeout_seconds))

  while [[ ! -f "$marker" ]]; do
    if ! kill -0 "$active_script_pid" 2>/dev/null; then
      echo "$mode run $pair exited before Doom startup completed" >&2
      sed -n '1,160p' "$log" >&2
      return 1
    fi
    if (( SECONDS >= deadline )); then
      echo "$mode run $pair timed out after ${startup_timeout_seconds}s" >&2
      sed -n '1,160p' "$log" >&2
      return 1
    fi
    sleep 0.1
  done

  active_pid="$(pgrep -P "$active_script_pid" | head -n 1 || true)"
  if [[ -z "$active_pid" ]] || [[ ! -r "/proc/$active_pid/smaps_rollup" ]]; then
    echo "could not resolve the Neomacs child process for $mode run $pair" >&2
    return 1
  fi

  now_ns="$(date +%s%N)"
  startup_ms=$(((now_ns - start_ns) / 1000000))
  sleep "$settle_seconds"
  smaps="/proc/$active_pid/smaps_rollup"
  if [[ ! -r "$smaps" ]]; then
    echo "$mode run $pair exited during the settle interval" >&2
    return 1
  fi
  snapshot="$work_dir/$mode-$pair.smaps_rollup"
  cp "$smaps" "$snapshot"

  local rss_kib pss_kib private_clean_kib private_dirty_kib private_kib anon_kib swap_kib
  rss_kib="$(metric Rss "$snapshot")"
  pss_kib="$(metric Pss "$snapshot")"
  private_clean_kib="$(metric Private_Clean "$snapshot")"
  private_dirty_kib="$(metric Private_Dirty "$snapshot")"
  private_kib=$((private_clean_kib + private_dirty_kib))
  anon_kib="$(metric Anonymous "$snapshot")"
  swap_kib="$(metric Swap "$snapshot")"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$mode" "$pair" "$startup_ms" "$rss_kib" "$pss_kib" \
    "$private_kib" "$anon_kib" "$swap_kib" | tee -a "$report"

  cleanup_process
}

median_for() {
  local mode="$1"
  local column="$2"
  awk -F '\t' -v mode="$mode" -v column="$column" '$1 == mode { print $column }' "$report" \
    | sort -n \
    | awk '{ values[NR] = $1 } END {
        if (NR % 2) print values[(NR + 1) / 2];
        else print (values[NR / 2] + values[NR / 2 + 1]) / 2;
      }'
}

printf 'mode\tpair\tstartup_ms\trss_kib\tpss_kib\tprivate_kib\tanonymous_kib\tswap_kib\n' >"$report"
echo "Writing samples to $report"
echo "Each mode gets $runs runs; samples are interleaved to reduce ordering bias."

for ((pair = 1; pair <= runs; pair++)); do
  if (( pair % 2 )); then
    run_sample commit-on-demand "$pair"
    run_sample eager-commit "$pair"
  else
    run_sample eager-commit "$pair"
    run_sample commit-on-demand "$pair"
  fi
done

echo
echo "Median results (MiB are derived from Linux KiB counters):"
printf '%-18s %12s %12s %12s %12s\n' mode startup_ms rss_mib private_mib anonymous_mib
for mode in commit-on-demand eager-commit; do
  startup_ms="$(median_for "$mode" 3)"
  rss_kib="$(median_for "$mode" 4)"
  private_kib="$(median_for "$mode" 6)"
  anon_kib="$(median_for "$mode" 7)"
  awk -v mode="$mode" -v startup="$startup_ms" -v rss="$rss_kib" \
    -v private="$private_kib" -v anon="$anon_kib" \
    'BEGIN { printf "%-18s %12.0f %12.1f %12.1f %12.1f\n", mode, startup, rss / 1024, private / 1024, anon / 1024 }'
done
