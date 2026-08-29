#!/usr/bin/env bash
# Run the in-tree neovm-core VM-JIT micro-benchmarks and print the interp-vs-JIT
# ratios. These are the CORRECT pipeline to measure: elisp -> bytecode -> the
# Tier-0 interpreter vs the tiering method-JIT (jit/compile.rs: lower_leaf_full +
# lower_mir_pure). NOT neovm-executor/benches/jit_benchmarks.rs, which drives a
# SEPARATE pipeline (neovm_compiler SSA -> neovm-executor/jit_rt) and does not
# exercise neovm-core's JIT.
#
# Each bench (jit_bench_* in neovm-core/src/emacs_core/runtime/eval/tests/mod.rs) is an
# #[ignore] test that pins a Hot copy (set_hot_for_test) against a force-Cold copy
# (set_cold_for_test) in ONE process to cancel CPU-frequency variance, warms once
# (compile is outside the timed loop), then reports min-of-N steady-state run time
# via a panic! whose message carries the BENCH line — so the tests "FAIL" by
# design; the numbers are the deliverable.
#
# Usage: neovm-core/scripts/run-jit-bench.sh [extra nextest filter args]
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

cargo nextest run -p neovm-core --features jit --release \
    --run-ignored ignored-only -E 'test(/jit_bench_/)' --no-fail-fast --no-capture \
    "$@" 2>&1 | grep 'BENCH '
