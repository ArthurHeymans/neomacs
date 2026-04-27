# neovm-gc Modernization Plan

**Date**: 2026-04-27
**Status**: Plan
**Branch**: `wire-neovm-gc`

## Goal

Move neomacs from the current modern-ish hybrid collector to a genuinely
modern, very low-pause, high-throughput collector suitable for an interactive
editor.

The target is not "zero stop-the-world ever". The target is:

- precise, generational GC
- TLAB bump allocation on the fast path
- mostly-concurrent old-generation marking
- pause-bounded minor, remark, reclaim, and compaction phases
- no global safepoint lock as a throughput bottleneck
- adaptive pacing that starts work early enough to avoid emergency full pauses
- telemetry good enough to debug P99 regressions

## Current State

Implemented today:

- precise tracing and relocation through `Trace` descriptors
- nursery, old, pinned, large, and immortal spaces
- per-mutator TLAB nursery allocation
- card-table remembered sets for old-to-young edges
- SATB-style support for active major marking
- persistent major/full collection sessions
- background service and dedicated concurrent-marker scaffold
- bounded major reclaim-prep and reclaim-commit slices
- pause histogram, pacer, barrier stats, compaction stats, and background status
- broad `cargo nextest` coverage in `neovm-gc` and GC integration tests in `neovm-core`

Important limitations:

- minor GC remains stop-the-world
- major/full marking can be background/concurrent, but the implementation is
  lock-alternating rather than lock-free
- reclaim and compaction still require stop-the-world slices
- shared heap mutation still depends on a global safepoint `RwLock<()>` plus
  heap storage locks
- old-generation relocation is not concurrent; moving work happens inside
  bounded stop-the-world windows
- the pacer is useful but not yet the single authority for all GC scheduling
- real Neomacs bootstrap/editor workloads need stronger pause and throughput
  gates than unit tests alone

## Non-Goals

- Do not switch to conservative stack scanning.
- Do not make old-gen objects permanently pinned to avoid solving relocation.
- Do not add a load barrier until old-gen concurrent relocation is actually
  being implemented.
- Do not optimize by weakening rooting or relocation correctness.
- Do not keep env toggles as the normal performance story. Toggles are for
  diagnostics only.

## Milestone 0: Baseline And Regression Gates

Purpose: make performance and pause claims measurable before deep refactors.

Tasks:

- Capture current `neovm-gc` Criterion baselines for allocation throughput,
  barrier throughput, collection latency, and multi-mutator scaling.
- Add a small scripted benchmark wrapper that records wall time, pause histogram,
  allocation rate, major mark throughput, reclaim throughput, dirty-card counts,
  and background-session counts.
- Add a GC stress profile for `neovm-core` bootstrap-like workloads: bytecode
  eval, macroexpand/pcase, load/eager compile, and pdump-adjacent flows.
- Add a checked-in baseline file with machine/date/compiler metadata.
- Define "red line" regressions: P99 pause, total bootstrap time, allocation
  throughput, and live-byte blowup.

Done when:

- `cargo nextest run -p neovm-gc --no-fail-fast` is green.
- GC-related `neovm-core` nextest filters are green.
- Bench scripts can compare current branch against a named baseline.
- A change that collapses bounded major slices into one large pause is caught by
  tests or benchmarks.

## Milestone 1: Handshake Safepoints

Purpose: remove the final global safepoint `RwLock<()>` as the main stop-the-world
coordination primitive.

Design:

- Add a `MutatorRegistry` owned by the heap.
- Each mutator owns a `MutatorState` containing:
  - mutator id
  - current epoch
  - last acknowledged safepoint epoch
  - active/parked/running state
  - root-stack publication state
  - local allocation/barrier buffer flush state
- Collector requests a safepoint by incrementing a global request epoch.
- Mutators poll and acknowledge at allocation slow paths, VM dispatch/backedge
  boundaries, callback boundaries, blocking I/O boundaries, and explicit
  `yield_safepoint` points.
- Collector waits for all registered mutators to acknowledge the requested epoch
  or prove they are parked with roots published.

Implementation steps:

1. Introduce registry data structures and tests without removing the `RwLock`.
2. Register/unregister mutators and expose snapshots for tests.
3. Thread safepoint polling through allocation, barriers, and public mutator APIs.
4. Add VM/evaluator polling hooks in `neovm-core`.
5. Teach collector entry points to request and wait on handshake acknowledgements.
6. Keep the existing safepoint lock as a debug fallback until parity tests pass.
7. Remove the lock from normal safepoint coordination.

Done when:

- no normal mutator operation needs to hold a safepoint read lock
- collector can stop all mutators through epoch handshakes
- parked mutators do not block collection
- stale/unregistered mutators cannot hide roots
- tests cover mutator enters, exits, parks, wakes, panics, and nested runtime calls

## Milestone 2: Split Heap Storage Contention

Purpose: make mutator allocation and barriers scale while concurrent mark is
active.

Tasks:

- Keep TLAB hit allocation entirely local.
- Move TLAB refill to a narrow nursery allocator lock or atomic reservation path.
- Split old/pinned/large allocation metadata from global `HeapCore` mutation.
- Keep collector state behind its own lock-free mirrors or narrow mutex.
- Remove heap write-lock dependency from inactive barrier fast paths.
- Ensure SATB and remembered-set barriers remain exact during active major mark.
- Replace global dirty-card scans with dirty-region/card queues where profitable.

Done when:

- multi-mutator nursery allocation scales substantially better than the current
  shared-lock shape
- inactive write barrier path does not acquire collector or heap locks
- active SATB barrier correctness tests still pass under concurrent mutation
- no benchmark shows a new O(heap) allocation or barrier path

## Milestone 3: Background Major By Default

Purpose: make "modern GC" the default path, not an optional mode.

Tasks:

- Make concurrent/background major marking the normal policy whenever workers are
  available.
- Remove or demote environment toggles that disable background/incremental major
  collection from normal operation.
- Keep explicit synchronous APIs, but make them drain the same active/background
  pipeline with bounded slices.
- Wire editor idle time to background service ticks.
- Teach the pacer to start major work before the old-generation allocation debt
  becomes urgent.

Done when:

- major/full plans auto-start background sessions under pressure
- synchronous `collect(Major|Full)` records multiple bounded pause samples for
  large heaps
- idle-time service can make progress without user-visible work
- disabling background GC is a diagnostic override, not a default configuration

## Milestone 4: Mostly-Concurrent Reclaim

Purpose: stop paying old-generation reclaim cost in stop-the-world commit slices
except where object graph mutation requires it.

Tasks:

- Represent reclaim candidates as region/block work items.
- Build reclaim snapshots incrementally with bounded object budgets.
- Retire fully-dead regions through epoch-protected free lists.
- Make allocator reuse retired regions only after every active mutator has passed
  a safe epoch.
- Move finalizer queueing out of broad heap mutation windows.
- Keep weak and ephemeron semantics exact; do not reclaim until fixpoint and
  weakness processing are complete.

Done when:

- large dead old-gen regions can be returned to allocation without one broad STW
  pass
- reclaim work has explicit throughput and pause metrics
- finalizer, weak, and ephemeron tests pass under concurrent allocation pressure
- bounded reclaim tests cover multi-slice progress and completion

## Milestone 5: Incremental Region Compaction

Purpose: defragment old generation without long pauses, before attempting fully
concurrent relocation.

Tasks:

- Keep old-gen region/block liveness and fragmentation scores current.
- Select evacuation candidates by garbage density, live-byte budget, and pause
  budget.
- Move selected regions in bounded stop-the-world slices.
- Record and fix all root/object slots affected by moved objects.
- Make compaction resumable across slices.
- Integrate compaction debt with the pacer.

Done when:

- compaction pause is budgeted and visible in pause histograms
- fragmented workloads recover space without a large full-heap pause
- root/object slot relocation tests cover VM objects, conses, strings, vectors,
  bytecode, weak refs, ephemerons, and finalizers
- long-running allocation/fragmentation benchmarks stay bounded

## Milestone 6: Optional Concurrent Relocation

Purpose: reach the next tier of low-pause old-gen compaction. This is the most
risky milestone and should only start after incremental compaction is stable.

Two viable shapes:

- V8-style mostly-concurrent marking with stop-the-world evacuation/fixup kept
  tiny through region selection.
- ZGC/Shenandoah-style concurrent relocation with load barriers and forwarding
  metadata.

Recommended path for Neomacs:

- Stay V8-style first. It preserves zero-cost reads, which matters for Lisp value
  access.
- Only add a load barrier if measured old-gen compaction pauses remain too high
  after incremental region evacuation.

If load barriers become necessary:

- add forwarding metadata readable without a heap write lock
- add a read/load barrier at every GC pointer dereference point
- prove tagged pointer reads remain correct through forwarding
- update benchmarks to account for permanent read-barrier overhead

Done when:

- old-gen relocation no longer creates user-visible pauses in realistic editor
  workloads
- the read/write barrier cost is measured and accepted
- relocation stress tests run with concurrent mutators and background collector

## Milestone 7: Pacer As Scheduler

Purpose: make GC work proactive and adaptive, not threshold-reactive.

Tasks:

- Feed the pacer allocation rate, live-set growth, mark throughput, reclaim
  throughput, compaction throughput, dirty-card pressure, and finalizer backlog.
- Give each GC work category a budget: mark, reclaim prep, reclaim commit,
  finalizer drain, compaction, idle service.
- Add mutator assists when background GC falls behind.
- Optimize for P99 pause and bounded live-byte growth, not just average time.

Done when:

- high allocation rate starts background major early
- mutator assists prevent runaway heap growth
- benchmark output explains why GC chose each plan
- pause target overshoot is visible and actionable

## Milestone 8: neovm-core Integration Hardening

Purpose: make the modern collector the real Neomacs collector, not just a crate
success.

Tasks:

- Audit every VM callback boundary for rooted arguments and return values.
- Keep bytecode, eval, load, macroexpansion, and dynamic binding paths
  relocation-safe.
- Add root-slot tests for every Lisp heap object family.
- Run bootstrap and pdump workloads with forced tiny nursery/old thresholds.
- Run TUI/editor smoke tests with background GC enabled.
- Keep GNU-compat oracle tests focused on behavior, not allocation shape.

Done when:

- forced-GC bootstrap and pdump flows pass
- TUI tests pass with background/incremental major enabled
- `garbage-collect` compatibility output remains valid
- no raw `Value` survives a GC-capable callback unrooted

## Validation Commands

Use `cargo nextest`, not `cargo test`.

Core commands:

```sh
env RUSTC_WRAPPER= cargo nextest run -p neovm-gc --no-fail-fast
env RUSTC_WRAPPER= cargo nextest run -p neovm-core gc --no-fail-fast
env RUSTC_WRAPPER= cargo nextest run -p neovm-core bootstrap_macroexpand_all_pcase_and_pred_survives_1mib_minor_gc
cargo fmt --all
git diff --check
```

Benchmark commands should be captured through a wrapper so results include git
commit, rustc version, host CPU, and GC configuration.

## Risk Register

Highest-risk areas:

- missing root publication during handshake safepoints
- stale roots or raw `Value`s across callbacks
- weak/ephemeron ordering regressions during concurrent reclaim
- load-barrier overhead if concurrent relocation is introduced too early
- pacer instability causing either latency spikes or heap growth
- old-gen compaction moving objects while external/runtime slots are not fixed

Mitigation:

- add stress tests before changing each phase
- keep every new concurrent path paired with a deterministic single-threaded test
- gate large refactors behind pause/throughput baselines
- avoid deleting the old debug fallback until the new handshake path is proven

## Suggested Commit Sequence

1. Add benchmark wrapper and checked-in baseline.
2. Add `MutatorRegistry` skeleton and tests.
3. Register mutators and publish root states.
4. Add handshake safepoint request/ack protocol.
5. Thread safepoint polls through neovm-gc allocation/barrier/runtime APIs.
6. Thread safepoint polls through neovm-core eval/VM/runtime boundaries.
7. Switch collector STW entry points from global lock to handshakes.
8. Remove normal-path safepoint read lock.
9. Split allocator/barrier heap-lock bottlenecks.
10. Make background major default and remove normal env-mode split.
11. Implement epoch-retired concurrent region reclaim.
12. Implement incremental region compaction.
13. Reassess whether true concurrent relocation/load barriers are necessary.

## Definition Of Done

Neomacs GC can be called "very modern and fast" when:

- normal allocation is TLAB-local and scales across mutators
- inactive barriers are lock-free or near-lock-free
- major marking runs concurrently by default
- all remaining stop-the-world work is bounded by explicit pause budgets
- no global safepoint lock gates normal mutator progress
- old-gen reclaim and compaction are incremental or concurrent enough that P99
  editor pauses stay below target
- bootstrap, pdump, and TUI workloads pass with forced frequent GC
- performance regressions are caught by automated nextest and benchmark gates
