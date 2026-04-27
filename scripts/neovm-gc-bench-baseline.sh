#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/neovm-gc-bench-baseline.sh [OPTIONS]

Run the neovm-gc Criterion benchmark suite with reproducibility metadata.

Options:
  --quick                    Use short Criterion timings (default).
  --full                     Use Criterion's production-style timings.
  --bench NAME               Run one benchmark target. May be repeated.
  --sample-size N            Override Criterion sample size.
  --warm-up-time SECONDS     Override Criterion warm-up time.
  --measurement-time SECONDS Override Criterion measurement time.
  --save-baseline NAME       Pass --save-baseline NAME to Criterion.
  --baseline NAME            Pass --baseline NAME to Criterion.
  --output-dir DIR           Write logs and metadata to DIR.
  --dry-run                  Print commands without executing them.
  -h, --help                 Show this help text.

Default benchmark targets:
  alloc_throughput barrier_cost collection_latency multi_mutator_scaling workloads
EOF
}

profile="quick"
sample_size="10"
warm_up_time="1"
measurement_time="3"
save_baseline=""
compare_baseline=""
output_dir=""
dry_run=0
benches=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      profile="quick"
      sample_size="10"
      warm_up_time="1"
      measurement_time="3"
      shift
      ;;
    --full)
      profile="full"
      sample_size="100"
      warm_up_time="3"
      measurement_time="5"
      shift
      ;;
    --bench)
      [[ $# -ge 2 ]] || { echo "error: --bench requires a value" >&2; exit 2; }
      benches+=("$2")
      shift 2
      ;;
    --sample-size)
      [[ $# -ge 2 ]] || { echo "error: --sample-size requires a value" >&2; exit 2; }
      sample_size="$2"
      shift 2
      ;;
    --warm-up-time)
      [[ $# -ge 2 ]] || { echo "error: --warm-up-time requires a value" >&2; exit 2; }
      warm_up_time="$2"
      shift 2
      ;;
    --measurement-time)
      [[ $# -ge 2 ]] || { echo "error: --measurement-time requires a value" >&2; exit 2; }
      measurement_time="$2"
      shift 2
      ;;
    --save-baseline)
      [[ $# -ge 2 ]] || { echo "error: --save-baseline requires a value" >&2; exit 2; }
      save_baseline="$2"
      shift 2
      ;;
    --baseline)
      [[ $# -ge 2 ]] || { echo "error: --baseline requires a value" >&2; exit 2; }
      compare_baseline="$2"
      shift 2
      ;;
    --output-dir)
      [[ $# -ge 2 ]] || { echo "error: --output-dir requires a value" >&2; exit 2; }
      output_dir="$2"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -n "$save_baseline" && -n "$compare_baseline" ]]; then
  echo "error: --save-baseline and --baseline are mutually exclusive" >&2
  exit 2
fi

if [[ ${#benches[@]} -eq 0 ]]; then
  benches=(alloc_throughput barrier_cost collection_latency multi_mutator_scaling workloads)
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

commit="$(git rev-parse --short=12 HEAD)"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
if [[ -z "$output_dir" ]]; then
  output_dir="target/neovm-gc-bench-runs/${timestamp}-${commit}-${profile}"
fi

mkdir -p "$output_dir/logs"

metadata_env="$output_dir/metadata.env"
metadata_md="$output_dir/summary.md"
estimate_list="$output_dir/criterion-estimate-files.txt"
estimate_tsv="$output_dir/criterion-estimates.tsv"

{
  printf 'timestamp_utc=%q\n' "$timestamp"
  printf 'git_commit=%q\n' "$(git rev-parse HEAD)"
  printf 'git_branch=%q\n' "$(git rev-parse --abbrev-ref HEAD)"
  printf 'profile=%q\n' "$profile"
  printf 'sample_size=%q\n' "$sample_size"
  printf 'warm_up_time_seconds=%q\n' "$warm_up_time"
  printf 'measurement_time_seconds=%q\n' "$measurement_time"
  printf 'save_baseline=%q\n' "$save_baseline"
  printf 'compare_baseline=%q\n' "$compare_baseline"
  printf 'benches=%q\n' "${benches[*]}"
  printf 'host=%q\n' "$(hostname 2>/dev/null || true)"
  printf 'uname=%q\n' "$(uname -a)"
  printf 'rustc=%q\n' "$(rustc -Vv | tr '\n' ';')"
  printf 'cargo=%q\n' "$(cargo -V)"
} > "$metadata_env"

{
  echo "# neovm-gc benchmark run"
  echo
  echo "- Timestamp UTC: \`$timestamp\`"
  echo "- Commit: \`$(git rev-parse HEAD)\`"
  echo "- Branch: \`$(git rev-parse --abbrev-ref HEAD)\`"
  echo "- Profile: \`$profile\`"
  echo "- Sample size: \`$sample_size\`"
  echo "- Warm-up: \`${warm_up_time}s\`"
  echo "- Measurement: \`${measurement_time}s\`"
  echo "- Benches: \`${benches[*]}\`"
  if [[ -n "$save_baseline" ]]; then
    echo "- Criterion save baseline: \`$save_baseline\`"
  fi
  if [[ -n "$compare_baseline" ]]; then
    echo "- Criterion compare baseline: \`$compare_baseline\`"
  fi
  echo
  echo "Logs are under \`$output_dir/logs\`."
  echo "Machine metadata is in \`$metadata_env\`."
  echo "Criterion estimate paths are in \`$estimate_list\`."
  echo "Extracted estimates are in \`$estimate_tsv\` when Python is available."
} > "$metadata_md"

echo "writing benchmark run metadata to $output_dir"

run_bench() {
  local bench="$1"
  local log="$output_dir/logs/${bench}.log"
  local cmd=(
    cargo bench
    -p neovm-gc
    --bench "$bench"
    --
    --sample-size "$sample_size"
    --warm-up-time "$warm_up_time"
    --measurement-time "$measurement_time"
  )
  if [[ -n "$save_baseline" ]]; then
    cmd+=(--save-baseline "$save_baseline")
  fi
  if [[ -n "$compare_baseline" ]]; then
    cmd+=(--baseline "$compare_baseline")
  fi

  printf 'running:'
  printf ' %q' "${cmd[@]}"
  printf '\n'

  if [[ "$dry_run" -eq 1 ]]; then
    printf 'dry-run:'
    printf ' %q' "${cmd[@]}"
    printf '\n' | tee "$log"
    return 0
  fi

  "${cmd[@]}" 2>&1 | tee "$log"
}

for bench in "${benches[@]}"; do
  run_bench "$bench"
done

if [[ -d target/criterion ]]; then
  find target/criterion -path '*/new/estimates.json' -print | sort > "$estimate_list"
else
  : > "$estimate_list"
fi

if command -v python3 >/dev/null 2>&1; then
  python3 - "$repo_root" "$estimate_tsv" <<'PY'
import json
import pathlib
import sys

repo = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
criterion = repo / "target" / "criterion"

rows = ["bench\tmetric\tpoint_estimate\tlower_bound\tupper_bound"]
for path in sorted(criterion.glob("**/new/estimates.json")):
    rel = path.relative_to(criterion)
    bench = "/".join(rel.parts[:-2])
    try:
        data = json.loads(path.read_text())
    except Exception:
        continue
    for metric in ("mean", "median", "slope"):
        value = data.get(metric)
        if not isinstance(value, dict):
            continue
        interval = value.get("confidence_interval") or {}
        rows.append(
            "\t".join(
                [
                    bench,
                    metric,
                    str(value.get("point_estimate", "")),
                    str(interval.get("lower_bound", "")),
                    str(interval.get("upper_bound", "")),
                ]
            )
        )

out.write_text("\n".join(rows) + "\n")
PY
else
  echo "warning: python3 not found; skipping Criterion estimate extraction" >&2
fi

echo "benchmark run artifacts written to $output_dir"
