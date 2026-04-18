# Moving-Nursery GC Design Spec

## Goal

Reduce neomacs GC pause times from 300-500 ms per full-heap mark-sweep to
**<2 ms for typical minor cycles**, matching the pause profile of modern
production VMs (V8 Orinoco, HotSpot G1, .NET GC).

Success metric: on a 12 MB live heap under `NEOVM_GC_ENABLE_COLLECTION=1`,
> 95% of GC pauses during an editing session (scripted insert/delete loop
> for 5 minutes) finish in < 2 ms. Major collections still allowed to be
> slower but must be rare (< 1 per minute of sustained editing).

Non-goals: beating V8 on micro-benchmarks, sub-ms major cycles, changing
`Value` representation to handle indirection.

## Context: why current design caps at ~300 ms

`wire-neovm-gc` (Phases 1-4) wired a full major mark-sweep through
`neovm-gc`. Every Lisp heap type ships with `MovePolicy::Pinned` and
no-op `relocate()`. The collector correctly traces the live graph,
sweeps dead records from `ObjectStore`, and drops payloads. But:

- **No nursery.** Every allocation goes to `SpaceKind::Pinned`. The
  collector's nursery semispace is unused.
- **No generational split.** Every cycle is a major cycle: visits every
  reachable object, sweeps every `ObjectRecord`. Cost is O(N) where N is
  total heap size, not O(live-young) like V8 minor cycles.
- **No minor GC.** `Mutator::collect(Major)` only path exercised from
  neomacs. `CollectionKind::Minor` runs but finds the nursery empty.
- **Sweep rebuilds the full `objects` Vec every cycle** (via
  `commit_prepared_reclaim_objects`), proportional to total heap.
- **`finish_major_collection` is the pause bottleneck**, not mark.

The `TaggedValue` is a raw pointer into the payload. *This is the same
design V8 uses*. The bottleneck isn't pointer representation — it's that
we never let the collector move anything and therefore can't use any
modern GC technique. When nothing moves, every GC is a full trace.

## Design

### Conceptual model (V8 Orinoco analog)

```
Allocation path:
   fresh alloc ---> nursery (bump-allocate)
                    |
                    v
   survives minor --> old gen (pinned span pool) -- or compactable blocks

Minor GC (STW, target <2ms):
   1. Scan roots + remembered set (dirty-card scan for old->young edges)
   2. Copy live nursery objects into old gen / to-space
   3. Rewrite every live TaggedValue that points at a moved object
   4. Reset nursery bump pointer

Major GC (STW, accept ~300-500ms, runs rarely):
   1. Full mark from roots
   2. Sweep old gen (remove dead from ObjectStore)
   3. Optionally compact sparse old blocks (neovm-gc already has this path)
```

Key observation: most allocations die young. A `dolist` body that conses
temporaries, a `format` that builds a string, a `mapcar` lambda return
value — all short-lived. Under GNU Emacs's generational model the young
gen collects these in tens of microseconds. We want the same shape.

### Heap layout after Path 3

| Space | Policy | Used for | Size | Collection |
|---|---|---|---|---|
| Nursery (semispace) | `MovePolicy::Movable` | Fresh `GcCons`, `GcFloat`, small `GcVector`, small `GcLispString` | 16 MB bump arena | Minor (copy-evacuate) |
| Pinned span pool | `MovePolicy::Pinned` | `GcBuffer`, `GcWindow`, `GcFrame`, `GcMarker`, `GcBignum`, promoted survivors | Grows on demand | Major (mark-sweep) |
| Large | `MovePolicy::LargeObject` | Single allocations > 2 KB | Direct alloc | Major |

Current state: everything goes to Pinned. Path 3 routes short-lived
types to Nursery and promotes survivors to Pinned.

### What we already have

- [x] `alloc_pinned_raw` in `Mutator` — routes to `SpaceKind::Pinned`
- [x] Persistent safepoint (`pin_safepoint`) — avoids chunk-leak
- [x] External root scanner delivering `gc_root_buffer` to collector
- [x] Post-write SATB barriers (`gc_post_write_barrier`,
      `gc_post_write_barrier_bulk`)
- [x] Major cycle end-to-end (begin → mark → reclaim)
- [x] `ObjectStore` publication for every allocation
- [x] `trace()` implementations for all 17 `Gc*` types
- [x] Concurrent mark worker support in `neovm-gc` (unused by neomacs)

### What's missing

- [ ] **Real `relocate()` for every `Gc*` type.** Currently all are
      `fn relocate(&self, _relocator: &mut dyn Relocator) {}`. Need to
      mirror `trace()` but calling `relocate_tagged` on each edge so the
      collector can rewrite moved pointers. ~17 trivial impls, one per
      type.
- [ ] **Flip `MovePolicy` to `Movable`** for short-lived types.
      Candidates: `GcCons`, `GcFloat`, `GcLispString` (text-props
      make this harder), `GcVector` (payload is Vec<TaggedValue>,
      variable-size), bytecode frame temporaries. Keep `Pinned` for
      types with external invariants: `GcBuffer`, `GcWindow`, `GcFrame`,
      `GcMarker`, `GcBignum`, `GcSubr`, `GcSymbolWithPos`.
- [ ] **Nursery allocation path in `alloc_pinned_raw`.** Split into
      `alloc_nursery_raw` (for `Movable` types) and `alloc_pinned_raw`
      (for `Pinned` types), or pass `space` from the caller. Routes
      through `Mutator::alloc` with the right `SpaceKind`.
- [ ] **Root enumeration must expose `&mut TaggedValue`.** Today
      `trace_roots` hands `Value` by copy via a `FnMut(Value)` closure;
      the collector can see the pointer but cannot rewrite it. Need
      `FnMut(&mut Value)` so post-evacuation fix-up rewrites each root
      in place. Touches every `trace_roots` impl (20+ sites across
      eval.rs, obarray, buffers, frames, windows, threads, faces,
      overlays, timers, processes, etc.).
- [ ] **Remembered set for old→young edges.** When an old-gen object's
      slot is written to a young-gen value, minor GC must treat the old
      object as a root. `neovm-gc` has a remembered-edge table; the
      SATB barrier already records writes. Need to route the barrier's
      new-value check through `needs_remembered_edge` (already
      implemented — but only fires for `target_space == Nursery` and
      requires our owner to be non-nursery, non-immortal).
- [ ] **Minor-cycle driver in `flush_roots_and_collect`.** Currently
      hard-codes `CollectionKind::Major`. Needs a policy:
        - threshold-driven: try Minor first; if nursery fills
          repeatedly without much promotion, upgrade to Major.
        - explicit `(garbage-collect)`: always Major.
      `neovm-gc`'s pacer can drive this automatically once
      `prepare_typed_allocation` is wired.
- [ ] **Interior relocation for Vec-backed types.** `GcVector`'s items
      live in a heap `Vec<TaggedValue>`, not inline. Minor GC that
      copies a Vec's backing buffer requires either (a) allocating the
      new Vec in the nursery too (but Vec::grow is Rust's own
      allocator), or (b) keeping the Vec pinned and only moving the
      `GcVector` wrapper. Simplest first cut: keep `Vec`-backed types
      Pinned, move only leaf types (Cons, Float). Revisit later.

### Phase plan

Each phase lands independently, test suite must stay green.

**Phase α — Relocate impls (low risk, foundation).**
- Implement real `relocate()` for every `Gc*` type, mirroring `trace()`
- Keep `MovePolicy::Pinned` for everything (relocate never fires yet)
- All 638 `neovm-gc` tests still pass, all neovm-core tests still pass
- Single PR, ~2 days
- **STATUS: done** (commit 3b7f1b071)

**Phase δ — Root rewriting. (REORDERED, must come before β)**
- Change `GcTrace::trace_roots` signature from
  `fn trace_roots(&self, roots: &mut Vec<Value>)` to
  `fn trace_roots_mut(&mut self, visit: &mut dyn FnMut(&mut Value))`.
- Migrate all 20+ call sites in neovm-core.
- Wire the VM's `Context::trace_roots` to drive both the existing
  immutable path (for read-only analysis) and the new mutable path
  (for evacuation fix-up).
- ~3-5 days; highest risk phase because Rust's borrow checker will
  fight every `&mut` that crosses a trait object.
- **Order rationale:** Without root rewriting, any type flipped to
  `Movable` breaks on its first minor-GC evacuation (tagged values
  in live roots dangle). Originally listed 4th in the roadmap;
  moved to 2nd after discovering this blocker during Phase α.
- **STATUS: done** (commits 3b7f1b071 through febc96cec). All 23
  sub-system impl sites have `trace_roots_mut`;
  `Context::trace_roots_mut` drives them. neovm-gc side
  `ExternalRootRelocator` is plumbed through
  `with_flat_store_for_collection` and invoked after
  `relocate_forwarded_roots_and_edges` in both Minor and Full
  branches of `execute_collection_plan`. 638/638 neovm-gc tests,
  43/43 tagged tests green.

**Phase β — GcFloat as the canary.**
- Flip `GcFloat::move_policy` to `Movable`
- Route `alloc_float` through the nursery path (policy drives routing
  automatically via `select_allocation_space`)
- Verify nursery minor cycles run, floats get copied, tagged values
  get rewritten correctly via the Phase δ root-rewrite API
- Keep all other types Pinned to keep the blast radius small
- ~2-3 days
- **STATUS: done** (commits f95cc2f78, bd0ac29da). End-to-end:
  1. `Mutator::alloc_external_raw` respects `MovePolicy`;
     `TaggedHeap::alloc_float` routes through it.
  2. `TaggedHeap::new` registers an `ExternalRootRelocator` that
     walks `Context::trace_roots_mut` and rewrites each heap-tagged
     `TaggedValue` via a new `relocate_tagged_slot` helper. The
     closure finds the live Context through the
     `GC_RELOCATOR_CONTEXT` thread-local that
     `gc_collect_from_current_roots` installs around
     `complete_collection`.
  3. VM's synchronous collect switched from
     `CollectionKind::Major` to `CollectionKind::Full`, so every
     cycle evacuates the nursery and exercises the relocator.
  4. `GcFloat::move_policy()` flipped to `MovePolicy::Movable`.
  5. All 43 tagged tests pass; pre-existing failure count in
     neovm-core unchanged at 190 (same before and after the flip).

**Phase γ — GcCons (the big one).**
- Repeat Phase β for `GcCons`. Cons cells dominate allocation volume
  in Lisp workloads, so this is the bulk of the pause-time win.
- Add remembered-set probe points so cross-generation edges work
- Stress-test: `gc_stress` mode + heavy cons workload
- ~3-5 days
- **STATUS: done with caveats** (commits d7eb9fe5e, 9084d72cc,
  ca6cb2ef3, ba1eb8624, 9beb178bf, ca0116a66, 06e37a51b, c067f3ef6,
  0d715ba16). Full end-to-end wiring:
  `TaggedHeap::complete_collection_minor`, 8 thread-local
  `relocate_*_gc_roots` helpers,
  `FrameManager::trace_roots_mut` rebuilding HashMap<Value,Value>
  and HashMap<Value,RuntimeFace> keys via drain-and-reinsert,
  `TextPropertyTable::trace_roots_mut` same pattern,
  `trace_window_mut` covering Window enum variants, `alloc_cons`
  policy-aware, `GcCons::move_policy()` Movable,
  `gc_minor_threshold` bisected to 2 MiB.
  Measured impact (bootstrap_macroexpand_all_pcase_and_pred):
  - Before: 11.9 s, Major pauses accumulate because the
    span-allocator sweep on Pinned cons dominates.
  - After: 3.0 s; Major pauses 68-227 ms progressing across 4
    cycles. Improvement is ~4x wall-clock.
  Residual issues to address in future work:
  1. Minor cycles almost never fire in practice because Lisp sets
     `gc-cons-threshold` to 16 MiB and file loads allocate ~5 MiB
     per safe-point -- the 16 MiB Major threshold hits first every
     cycle. The 4x speedup is therefore entirely from the Movable
     flip making Major cheaper, not from Minor itself. To extract
     Minor's pause win, raise `gc-cons-threshold` far above typical
     burst size (64 MiB+) or restructure the pacer to trigger
     Minor on Nursery pressure rather than safe-point polling.
  2. At `gc_minor_threshold = 1 MiB`, bootstrap still signals
     partway through ldefs-boot -- a rare Value slot goes
     un-rewritten under high Minor frequency. Reproduces reliably;
     bisect by instrumenting `relocate_tagged_slot` or by
     comparing trace_roots vs trace_roots_mut slot-by-slot.
  3. Major pauses still 100-200 ms per cycle, which exceeds the
     p50 < 2 ms / p99 < 20 ms goal. Pause reduction needs either
     concurrent marking (SharedHeap + BackgroundWorker) or
     incremental mark/reclaim -- both are multi-week projects.

**Phase ε — Remaining short-lived types.**
- `GcLispString` (nursery survival depends on text_props being
  empty — fresh strings usually are).
- `GcVector` small cases.
- Bytecode constant arrays.
- ~3-5 days.

**Phase ζ — Pacer + policy.**
- Threshold-driven minor-vs-major selection.
- Tune nursery size, promotion age, major frequency.
- Measure against the success metric.
- ~3-5 days.

**Phase η — Incremental Major mark (partial) + concurrent Major (full).**
- *Incremental*: landed (commit 79aad7a17). Opt-in via
  `NEOVM_GC_INCREMENTAL_MAJOR=1`. Wires neovm-gc's
  `begin_major_mark` + `assist_major_mark` +
  `finish_major_collection` into the safe-point path so Major
  cycles slice mark work across small assist calls (~1 ms each)
  instead of blocking the mutator for the whole cycle.
  Default-off today because the bootstrap test calls
  `gc_collect_exact` per form; real editor paths that drive GC
  through safe-point thresholds should flip the default on.
- *Concurrent benchmark (500k cons churn, release)*:
  | Mode | Wall time | vs baseline |
  | --- | --- | --- |
  | default (sync Major) | 753 ms | 1.0x |
  | NEOVM_GC_BACKGROUND | 1159 ms | 1.5x slower |
  | NEOVM_GC_INCREMENTAL_MAJOR | 7108 ms | 9.4x slower |
  | both | 6962 ms | 9.2x slower |

  All modes ran the same 4 Major cycles; the deltas are pure
  overhead. INCREMENTAL_MAJOR regresses heavily because
  `assist_major_mark` takes the safepoint write lock every safe
  point (thousands of lock acquires per loop iteration);
  BACKGROUND alone adds modest cache/CPU contention. The sync
  path wins on single-threaded tight allocation loops because
  Major mark is already fast (cons are Movable, span sweep
  avoided) and the main thread is 100% CPU-bound on allocs, so
  there is no idle time for a background thread to exploit.
  Concurrent marking is the right shape for interactive /
  multi-threaded workloads where the mutator has substantial
  CPU work beyond allocation (user input wait, I/O, heavy
  non-Lisp computation). Defaults stay off;
  `neovm-core/examples/gc_bench.rs` is the repro harness.
- *Concurrent (background thread)*: **infrastructure landed,
  opt-in** (commits cb466566e, c15473979). neovm-gc gained three
  new APIs: `MutatorLocal::new_registered` /
  `MutatorLocal::release`, `Mutator::from_local` /
  `Mutator::into_local` (ManuallyDrop-suppressed release), and
  `SharedHeap::with_persistent_mutator`. These let a host reuse
  one `MutatorLocal` across calls, avoiding the ~40 GiB
  ObjectPublishLocal churn that plain `with_mutator` would
  cause.
  TaggedHeap now holds both `&static Heap` (direct, for the
  main mutator's persistent `Mutator<static>`) AND `&static
  SharedHeap` (for spawning a `BackgroundWorker`). Both views
  back the same `HeapState` via `Heap::clone` (Arc bump), so
  neither path costs the other anything.
  `SharedHeap::with_persistent_mutator` switched from outer
  write-lock to outer read-lock so alloc-heavy callers don't
  serialize against the background collector thread.
  Spawn the worker with `NEOVM_GC_BACKGROUND=1`. In the current
  bootstrap test the worker does not reduce pause time because
  `gc_collect_exact` is invoked per form (the caller blocks for
  the whole sync cycle). Concurrent-marking wins materialize
  only on the safe-point-driven path; set
  `NEOVM_GC_INCREMENTAL_MAJOR=1` alongside
  `NEOVM_GC_BACKGROUND=1` to exercise it in workloads that do
  not force per-form sync GC.

**Total estimate: 2-4 weeks focused work.** Phase δ is the risk
centerpiece; everything else is mechanical.

### Risk analysis

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `relocate_tagged` miswrites tag bits | Medium | Silent corruption | Exhaustive unit tests per `Gc*` type: allocate, force minor GC, verify all accessors still return the pre-GC value. |
| Root sites miss a `Value` | High | Silent corruption, crashes hours into a session | Convert one sub-manager at a time; keep old scanner path alongside for 1-week bakeout; diff root sets. |
| `Vec<TaggedValue>` interior pointers go stale | High | GcVector corruption after minor GC | Keep Vec-backed types Pinned for the first cut; revisit only after measuring whether they're a hot allocation path. |
| Write barrier misses cross-gen edge | Medium | Minor GC frees still-reachable old-gen-referenced object | Gate by `NEOVM_GC_ENABLE_NURSERY=1` during rollout; fall back to pinned-only if barriers misbehave. |
| `Rc`/`Arc` inside GcByteCode's `ByteCodeFunction` | Low | Bytecode frame corruption | Audit `ByteCodeFunction` fields; no non-Copy fields holding `TaggedValue` directly. |
| Borrow checker fights `trace_roots_mut` | High | Phase δ stalls for a week | Start with a raw-pointer-based visitor API that avoids the `&mut` lifetime issue; migrate to safe API once shape is clear. |
| Pdump-restored objects hit nursery on the first minor | Medium | Invalid assumption about age | Pdump path continues to go through `alloc_pinned_raw`; bootstrap-restored state always lands in old gen. |
| Performance regression in steady-state allocation | Low | Slower than current Pinned-only path | Benchmark `alloc_cons` + `alloc_float` hot paths before/after. Nursery bump should be strictly faster than span-pool alloc. |

### Testing strategy

- Unit: per-type `relocate` correctness (allocate, run minor, check).
- Integration: existing 43 tagged tests + 638 neovm-gc tests (must stay
  green across every phase).
- Stress: `gc_stress = true` mode with tight threshold (< 64 KB) to
  force GC on every allocation and catch any missed root / barrier.
- End-to-end: buffer_mark + call_interactively tests with
  `NEOVM_GC_ENABLE_COLLECTION=1` at every phase.
- Pause-time bench: scripted edit loop for 5 min, capture GC pause
  histogram. Assert p50 < 2 ms, p99 < 20 ms after Phase ζ.
- RSS bench: `neomacs -nw` 30-min idle, RSS should stay < 200 MB.

### Open questions

- **How aggressively to promote?** V8's tenure threshold is ~2 minor
  cycles. neovm-gc's `age` field in `ObjectHeader` already tracks this.
  Default from config: promote after how many cycles? Needs
  experimentation.
- **Concurrent marking.** Phase ζ doesn't need concurrent marking;
  major cycles are rare enough that a 300ms pause once a minute is
  tolerable. If unacceptable, Phase η can wire `SharedHeap` +
  `BackgroundWorker` (separate 1-week project).
- **How does this interact with pdump?** Pdump restores objects into
  `SpaceKind::Pinned` via `alloc_*`. With nursery enabled, that's still
  correct — bootstrap objects go directly to old gen, which is what we
  want. Verify no code path restores via a path that would hit the
  nursery.
- **Relocation of `UnsafeCell<Vec<Value>>` contents.** GcVector,
  GcLambda, GcMacro, GcRecord, GcByteCode, GcHashTable all have
  `UnsafeCell<Vec<...>>`. Interior relocation requires mutable access
  during a GC — possible in practice because GC holds the only
  reference at that moment, but needs careful write-up.

### Implementation order inside each phase

1. Add the code change behind a feature flag or env var (same pattern
   as `NEOVM_GC_ENABLE_COLLECTION`).
2. Run `cargo nextest -p neovm-gc` — must be 638/638.
3. Run `cargo nextest -p neovm-core --lib tagged` — must be 43/43.
4. Run `cargo nextest -p neovm-core --no-fail-fast` and diff pass/fail
   count against main. Any regression blocks the phase.
5. Run `buffer_mark` 3× with env flag on; stable.
6. Commit behind the flag.
7. Next phase flips the flag default or adds another.

### What not to do

- **Don't change `TaggedValue` to use `ObjId` / handle indirection.**
  V8 doesn't. HotSpot doesn't. JavaScriptCore doesn't. This was the
  mistake in my first analysis; our representation is fine.
- **Don't enable concurrent marking yet.** Orthogonal to nursery work.
  Concurrent marking on top of nursery is a separate `SharedHeap`
  refactor.
- **Don't touch pdump format.** Pdump-restored state goes to old gen.
  The nursery is only for runtime allocations.
- **Don't try Phase γ (GcCons) before Phase β (GcFloat) and Phase α
  (relocate impls).** Each phase builds testing confidence for the
  next.

## Success criteria recap

- Pause p50 < 2 ms during scripted editing
- Pause p99 < 20 ms during scripted editing
- Major cycles < 1 per minute of sustained editing
- Test suite ≥ current 5676 pass count with collection on
- No RSS growth over 30 min idle (`neomacs -nw`)

If we don't hit these, the design was wrong and we iterate. The shape
of the win — moving nursery + pinned old — is what every production VM
does, so the path is well-trodden; what varies is the tuning.
