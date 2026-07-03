# Task 12 — Small parity + polish items (consolidated)

Status: each is hours-scale, independent, good first tasks for someone
learning the codebase. Gate bar for all: the targeted nextest matrix +
clippy-clean diff; none needs a critique round (they are additive/parity
items with no concurrency surface) EXCEPT where noted.

## 12a. `memory-limit` builtin (unimplemented)

Found by the GC API audit: GNU's `memory-limit` returns an integer (KiB of
process memory, historically sbrk-based; modern GNU returns 0 when unknown).
neovm-core has `memory-info` (symbols.rs) but no `memory-limit`. Implement as
a thin builtin (sysinfo is already a dependency; or return 0 for strict-GNU
minimalism — check what GNU 31 actually returns on Linux and match it),
register with the right arity, add the compat-shaped unit test. Also grep the
stubs list (`builtin_garbage_collect_maybe`, `garbage_collect_heapsize` in
stubs.rs) and consider upgrading `garbage-collect-maybe` to a real
conditional collect (`should_collect()` is exposed) — trivial now that the
pacer exists.

## 12b. Finalizer inert-dump GNU parity (optional; current behavior is
deliberately stricter)

Current: `dump-emacs-portable` with a live finalizer signals
`(error "Cannot dump Emacs with a finalizer object")` after a
collect-until-quiescent loop (registry-emptiness precondition; writer-arm
panics kept as unreachable backstops). VERIFIED GNU behavior (against the
oracle binary): GNU *dumps finalizers inertly* — reload yields a type-correct
`#<finalizer>` that never runs. If parity is ever demanded: add a Finalizer
variant to the dump format (convert.rs DumpValue graph + object_starts +
load-side reconstruction as an inert object that is NOT registered in
`finalizer_registry`), delete the pre-scan error, keep a test that a reloaded
finalizer never fires. NOTE: needs the pdump round-trip test patterns; touch
`collect_veclike_children`⊇`trace_veclike` consistency for the variant.
Low value — GNU's own semantics here are arguably a wart; the strict error
is defensible. Do it only if a real .pdmp workflow hits the error.

## 12c. AOT prepopulate residual levers (~6.4ms floor)

Post fast-reject + manifest-v2 pre-keys, the remaining prepopulate median
decomposes as: obarray function-cell scan (the `interned_function_cells()`
walk), the 706 MEMBERS' content hashes (kept BY DESIGN — the live hash feeds
the dlsym ground truth), 706 leaf loads (dlsym + descriptor decode +
`live_reloc_for_emit_tier` build_mir), manifest parse, cache insert + heap
sync. Levers if startup ever matters more: (i) persist per-member
`(name -> ops_len, arity, hash)` AND trust it when the live fn's
(ops_len, arity) match, skipping the member hash too — this weakens the
fail-closed chain from hash-verified to shape-verified + dlsym; adjudicate
that tradeoff explicitly before doing it; (ii) lazy prepopulate (arm a
first-miss hook instead of the boot-time walk) — changes native-from-call-1
to native-from-call-2 for loadup fns; measure whether anyone can tell;
(iii) parallelize the walk (it is pure reads until insert) — thread-safety
audit needed on the obarray iterator. Only bother if AOT graduates from
opt-in.

## 12d. `CONCURRENT_GC.md` + module-doc refresh

The GC's architecture doc lags the code by the whole 2026-07 arc (parity
bits, string claiming, arenas, pacer, dump-less concurrent, per-group
handshake stats). One session: rewrite it against `tagged/gc.rs` as-built,
keeping the invariants sections (collect⊇trace; tenured-before-parity read
order; no-free-during-mark; page ownership incl. retired; born-at-parity;
the claim-ordering rules) — these are the sentences future critics grep for.
Also fix the stale module docs the audits flagged (compile.rs header claims
about &optional/&rest bails; the ABI_TAG u128 comment in aot.rs; "6 MIR
shims" references).

## 12e. Bench/profiler hygiene

- `run-jit-bench.sh` / `run-aot-bench.sh` / the `gc_drain_kinds_profile_*`
  + `alloc_roundtrip_cost_probe` + `vm_subr_mix_*` inventory: add a single
  `neovm-core/scripts/run-perf-suite.sh` that runs the whole panel and emits
  one comparable report (the interleaved-A/B discipline built in), so
  before/after numbers across future work stop being hand-assembled.
- Consider promoting the handshake/drain medians into a tiny CI-able
  smoke-check (assert start-pause < 1ms pdump in a release probe) to catch
  pause regressions mechanically.
