# Task 11 — ThreadSanitizer pass over the concurrent GC (verification debt)

Status: SMALL, one session. Risk: none (verification only). Value: the
concurrent machinery grew a LOT in 2026-07 (parity claims, string claiming
with the AtomicPtr intervals field, page-base snapshots, immediate-join
wake) and TSan has NOT been run over the new surface — the toolchain is
pinned to stable (`rust-toolchain.toml`, 1.95.0) and `-Zsanitizer=thread`
needs nightly + build-std. The strmark work explicitly recorded "TSan not
run" as a deviation.

## 1. What to run

1. A nightly toolchain SIDE INSTALL (do not change the repo pin):
   `rustup toolchain install nightly --component rust-src`, then
   `RUSTFLAGS="-Zsanitizer=thread" cargo +nightly nextest run -p neovm-core
   -Zbuild-std --target x86_64-unknown-linux-gnu -E 'test(/tagged::/)'`
   (expect to need `--no-default-features`/feature juggling if any dep fights
   build-std; document what worked).
2. Target the CONCURRENT tests specifically: the tagged:: module's
   concurrent-mark overlap tests, the seqlock race test (the designed
   negative-control pattern — it proves the harness has teeth), the string
   interval-flip race test, the float/vector page tests that run cycles, the
   two-cycle parity tests, and `finalizer_*` concurrent variants.
3. Add ONE new adversarial test while in there: a mutator thread hammering
   `put-text-property`/`remove-text-properties` (intervals AtomicPtr flips)
   + vector `aset` + `ensure_intervals` churn while a concurrent mark runs
   with claiming enabled — the widest write/claim overlap in one test.

## 2. What TSan can and cannot catch here (set expectations)

- CAN: real data races on the atomics' surrounding fields, missed
  atomic conversions (e.g. a plain write to a field the GC thread reads),
  ordering bugs that manifest as racy accesses.
- CANNOT: the noalias/Box-aliasing UB class (that is Miri territory — the
  intervals field was RETYPED to AtomicPtr precisely to avoid it, per the
  string critique; if a Miri run over the non-threaded parts is cheap, note
  it as a bonus, but Miri cannot run the threaded GC meaningfully).
- Known-benign TSan noise to pre-triage: the Relaxed mark-bit operations are
  intentional (matching the audited cons-path discipline); document any
  suppression with the reasoning rather than silencing wholesale.

## 3. Deliverable

A short report (docs/ or the GC md): command lines that worked, suite results,
any races found (each becomes its own fix commit with the standard gate
matrix), suppressions with justification, and a repeatable script under
`neovm-core/scripts/` (e.g. `run-gc-tsan.sh`) so this becomes a periodic
gate rather than a one-off. If races ARE found in the 2026-07 surface, run
the fix through the usual critique discipline — concurrency fixes under time
pressure are how regressions happen.
