# Concurrent GC for neovm-core — design & phased plan

Goal: a background GC thread that marks (and eventually sweeps) concurrently
with the mutator, so the mutator only stops for short safe-point handshakes
(root snapshot + mark termination), each sub-millisecond. This is the Go-style
design: concurrent tri-color mark, SATB write barrier, cooperative safe points.

Built behind `NEOVM_GC_CONCURRENT` (default OFF) until proven; the existing
incremental collector (default) stays untouched and is the fallback.

## Where we start from

Already in place (the incremental collector):
- Tri-color mark with an explicit gray queue, sliced across safe points.
- Dump partition (mapped pdump image is permanent-black) + tenured old gen +
  remembered set, so a cycle only traces the young heap + dirty owners.
- Incremental-update (Steele) write barrier: `record_heap_write` logs dirty
  owners; each slice re-traces them.
- Deferred (incremental) sweep with allocate-black during the sweep window.
- Safe points: `gc_safe_point` at eval boundaries (GNU `maybe_gc` model).
- The whole VM is otherwise single-threaded; the heap is reached through a
  `thread_local!` raw pointer and every Lisp slot is a plain (non-atomic) field.

## The hard problem (Rust-specific)

A GC thread reading the object graph while the mutator writes it is a data
race = UB in Rust, not just a logic bug. So every GC-visible mutable word that
the two threads touch must be accessed atomically (or via `UnsafeCell` + the
right `Ordering`). To bound the shared surface, ONLY MARKING goes concurrent;
allocation and sweep stay mutator-side (sweep already incremental; a short
handshake hands the sweep work over or runs it mutator-side). Shared surface:

- Mark state: cons block mark bitmaps, `GcHeader.marked`, mapped mark vectors.
- Object slots the GC thread reads while the mutator writes: `ConsCell.car/cdr`,
  vector/record/closure/bytecode/char-table slots, string interval roots, etc.
- The gray queue (mutator pushes via barrier/alloc; GC thread pops).
- The SATB log buffers (per-mutator; GC thread drains).

x86 relaxed atomic loads/stores compile to plain mov, so the perf cost of
atomic slots is essentially zero; the cost is the pervasive code change.

## Barrier: switch to SATB (snapshot-at-the-beginning / Yuasa)

Concurrent marking wants SATB, not incremental-update: before the mutator
overwrites a slot, it logs the OLD value to a thread-local buffer; the GC
thread drains the buffers and marks those values. This keeps the start-of-cycle
snapshot live regardless of concurrent mutation, with a wait-free fast path
(append to a local buffer). New objects allocate-black. The current
`record_heap_write` already runs at every mutation site (18 in tagged/mutate.rs
+ value.rs char-table/obarray) — it gains an "append old value to SATB buffer"
path when a concurrent mark is active.

## Handshakes (the only STW)

1. Initial mark / root snapshot: stop the mutator briefly, scan roots (push to
   the gray queue), enable the SATB barrier + allocate-black, start the GC
   thread. (The obarray root scan is the ~0.5ms floor here — see Stage 0 note.)
2. Mark termination: stop the mutator briefly, drain residual SATB buffers,
   re-scan the small stack/specpdl roots, confirm gray empty. Then sweep.

Both are bounded root scans, sub-ms. Everything between runs concurrently.

## Phases (each flag-gated, verified with the gc_stress test + fresh-build
##         verify + full suite before the next)

- Phase 1 — Atomic mark state. Convert cons bitmaps / `GcHeader.marked` / mapped
  mark vectors to atomic access. No concurrency yet (single-threaded, behavior
  identical) — de-risks the representation change in isolation.
- Phase 2 — Atomic object slots. Convert the slots the GC reads to atomic
  access. Still single-threaded. Verify no regression.
  - 2a DONE (`f116d798b`): cons car/cdr (a single set_car/set_cdr chokepoint).
  - 2b/2c the hard part — see "Resizable structures" below. Fixed-size arrays
    (Emacs vector / record / closure: set via `aset`, never resized) and the
    individual TaggedValue fields are mechanical (atomic element store at the
    write + atomic load in `trace_veclike`). GROWABLE structures need real care.

### Resizable structures (the Phase 2b/2c design decision)

Unlike cons, veclike slot writes are NOT a single chokepoint: `with_*_data_mut`
hand a `&mut Vec<TaggedValue>` to arbitrary closures (push/extend/index/sort),
and hash tables + obarray buckets GROW — a `Vec::push` can REALLOCATE, moving
the backing buffer. A concurrent GC thread holding a pointer into the old
buffer would then read freed memory (UAF). So plain "atomic element store"
only suffices for fixed-size arrays.

Approach for growable structures (to implement when the GC thread lands):
1. Element writes go through the atomic-store path + the SATB barrier (records
   the overwritten value) — keeps the start-of-cycle snapshot live.
2. The backing-buffer POINTER is published atomically (Release) on realloc and
   loaded atomically (Acquire) by the GC; the OLD buffer is RETIRED onto a
   "retired buffers" list kept alive until mark termination, so the GC thread's
   in-flight read of either buffer is always valid (no UAF). Freed at the
   termination handshake.
3. Alternatively (simpler first cut): snapshot growable structures' slot
   pointers into the gray queue at the STW root-snapshot handshake, so the GC
   thread never reads a growable backing buffer concurrently — only fixed-size
   arrays and individual fields are read concurrently. Element mutations are
   still covered by SATB. This trades a slightly larger snapshot handshake for
   not needing retired-buffer bookkeeping; do this first, optimize later.

Recommended: start with (3) — fixed-size arrays + fields read concurrently
(mechanical atomic conversion), growable structures captured at the snapshot
handshake. Move to (2) only if the snapshot handshake proves too long.
- Phase 3 — SATB barrier + per-mutator log buffers, drained on the mark path.
  Still single-threaded (the buffer just replaces/augments dirty-owner re-trace).
- Phase 4 — GC thread + handshake protocol + shareable heap (the `Send`/sharing
  model). GC thread initially idle except handshake validation — proves the
  threading/sharing model with no marking moved yet.
- Phase 5 — Move marking onto the GC thread: it drains the gray queue + SATB
  buffers concurrently; mutator only does the two handshakes. Allocate-black.
- Phase 6 — Concurrent / handed-off sweep; tighten the handshakes.

## Risks / invariants

- UB from a missed atomic / wrong `Ordering` → nondeterministic corruption,
  worse than the UAFs already fixed. Gate every phase with `gc_stress` (GC at
  every safe point) + `NEOVM_GC_VERIFY_PARTITION` + the full suite; add a
  concurrent stress test and run under ThreadSanitizer where possible.
- Any new root source must be seeded at BOTH the snapshot and the termination
  handshake (the lesson from the incremental-termination UAF).
- Keep allocation + free list mutator-only; never let the GC thread touch them.
- The default (incremental) collector must stay byte-for-byte unchanged when
  `NEOVM_GC_CONCURRENT` is off.
