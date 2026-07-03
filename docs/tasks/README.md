# neovm-core performance roadmap — task documents

This directory holds one design + implementation document per open roadmap task
for neovm-core's VM/GC/JIT/AOT/regex modernization. They were written at commit
`013f94d03` (2026-07-03), immediately after the three-phase modernization run
that delivered: the AOT tier, direct-subr JIT intrinsics (7.9x predicates), the
GC pacer + dump-less concurrent collection (7x less blocking), parity mark bits
(clear walk -> 0), concurrent string marking (termination drain 14.9x), the
regex fastmap + failure-stack overhaul (font-lock 10.1x), and the size-class
object arena for floats/strings/vectors (ownership hashset eliminated).

**Line-number caveat:** file:line citations were accurate at `013f94d03`. Code
drifts; navigate by SYMBOL first, use line numbers as hints. Every doc pins the
commits it builds on by subject line.

## The house method (follow it — it caught real bugs every single time)

Every task here follows the same discipline, and the documents are structured
around it:

1. **Evidence first.** No implementation without a measurement showing the cost
   is real. The instrumentation for almost everything already exists
   (`NEOVM_GC_TRACE` handshake/drain lines, `SweepStats`/`HandshakeStats`/
   `DrainKinds`, the `vm-profile` SUBR-MIX builtin ranking, `NEOVM_JIT_COMPILE_STATS`,
   the `#[ignore]`d release profilers `gc_drain_kinds_profile_*`,
   `regex_bench_*`, `jit_bench_*`, `alloc_roundtrip_cost_probe`).
2. **Design, then adversarial critique.** For anything touching concurrency,
   GC invariants, or GNU parity: write the design, then have at least two
   INDEPENDENT reviewers attack it through different lenses (correctness/UB
   lens + mechanics/semantics lens) with a mandate to REFUTE, not approve.
   In the runs that produced these documents, every single critique round
   found at least one real, would-have-shipped bug — including three bugs in
   the coordinator's own mandates that implementation agents then caught.
3. **Implement with the converged mandates as hard constraints.** The docs
   list them; they are load-bearing. "Simplifying" one away has a named
   failure mode (UAF/leak/corruption/parity divergence) in every case.
4. **Gate per commit.** The reliable bar on this codebase:
   `cargo nextest run -p neovm-core -E 'test(/<targeted-regex>/)'` in FOUR
   variants — plain, `NEOVM_GC_VERIFY_PARTITION=1`, `NEOVM_GC_STRESS=1`, and
   combined — plus `cargo clippy -p neovm-core --lib` (your diff adds zero
   warnings), plus the relevant `#[ignore]`d release profilers for
   before/after numbers. Full-suite (`--no-fail-fast`, with VERIFY) at
   integration checkpoints. Do NOT run the oracle/tui suites for VM-internal
   work.
5. **Report honestly.** Numbers that refute the plan are the deliverable, not
   a failure. Two of the tasks below exist BECAUSE a measurement said "don't
   build the obvious thing".

## Known environment gotchas (all hit during the runs; will bite you too)

- Fresh git worktrees LACK ~1727 git-ignored generated lisp files
  (`lisp/international/*.el`, `.elc`s) — bootstrap-loading tests fail with
  confusing load errors. Fix: `rsync -a <main-checkout>/lisp/ ./lisp/`.
- nextest on this box SILENTLY DROPS `or`-joined filterset clauses:
  `-E 'test(a) or test(b)'` runs only `a`. Use ONE regex: `-E 'test(/a|b/)'`.
- Worktree target dirs are 20-50GB; the box died at 98% disk with silent
  link-step failures. `df -h` before diagnosing mystery build errors; remove
  integrated worktrees promptly; prune `target/debug/incremental`.
- A dead sccache server produces compat_module test failures/timeouts with no
  useful diagnostics. `sccache --start-server` and retry before debugging.
- Benchmarks on this shared box: another session may push loadavg to 500-800.
  Interleave A/B runs (before,after,before,after... min-of-N per side) so both
  sides see the same load; report loadavg alongside numbers; in-process
  median-over-many-cycles probes tolerate load, short wall-clock boots do not.
- The bench-style tests report via `panic!` BY DESIGN (`--run-ignored
  ignored-only --no-capture`); a FAILED bench test is the report, not a bug.

## Task index (ranked)

| # | file | area | state |
|---|------|------|-------|
| 01 | `01-concurrent-veclike-claiming.md` | GC | design ready; UNBLOCKED by exact page ownership |
| 02 | `02-intrinsics-round-2.md` | JIT | profile-first protocol ready |
| 03 | `03-box-type-migrations.md` | GC/alloc | bytecode -> markers -> hash tables; hazards mapped |
| 04 | `04-display-latency-frontier.md` | display | pointer doc; the true keystroke frontier |
| 05 | `05-async-jit-compilation.md` | JIT | EVIDENCE-GATED; full adjudicated design recorded |
| 06 | `06-survival-based-tenuring.md` | GC | prerequisite design (page evacuation/retirement) |
| 07 | `07-regex-lazy-dfa.md` | regex | EVIDENCE-GATED (deferred); prerequisite bug + re-scoped increment recorded |
| 08 | `08-aot-speculation-pgo.md` | AOT | design question; unlocks PGO + package cache |
| 09 | `09-jit-standing-defers.md` | JIT | consolidated go-criteria (OSR, ICs, float, feedback, gate) |
| 10 | `10-gc-dirty-owners-aba.md` | GC | small; MUST land before dirty_owners gets a consumer |
| 11 | `11-tsan-verification-debt.md` | GC | small; nightly TSan pass over concurrent tests |
| 12 | `12-small-parity-items.md` | misc | memory-limit, finalizer inert-dump, AOT residual levers |

Cross-cutting background lives in the session memory notes referenced by each
document, and in `neovm-core/src/tagged/CONCURRENT_GC.md` (update it as GC
stages land — it lags the code today).
