# Task 05 — Background (async) JIT compilation

Status: EVIDENCE-GATED DEFER — do NOT build until the trigger fires. This doc
records the complete adjudicated design (one recon + two independent
critiques) so that when the trigger fires, implementation can start without
re-deriving anything. Risk: HIGH (two confirmed UAF classes in the naive
design — both understood and fenced below). Effort: v1 ~1-2 sessions; v2 a
multi-session refactor.

## 1. The trigger (why this is deferred)

Measured (always-on metering at the compile seam, `jit/stats.rs`, landed):
per-function synchronous compile stall is **mean ~0.8ms, max 2.9ms** (57-op
body), ~8ms per 10-compile cold burst. Stalls happen once per function per
session. The truly big bodies (call-dominated) are mostly rejected by
`body_is_jit_profitable` and never compile at all. Sub-frame, rare, bounded.

**Revive when:** `NEOVM_JIT_COMPILE_STATS=1` on a real workload shows compile
stalls >5ms, or threshold-crossing bursts correlate with dropped frames, or
intrinsics round 2 (task 02) loosens the profitability gate enough that big
bodies start compiling. The metering prints a running summary every 64
compiles and exposes `compile_stats_snapshot()` for tests — the detector is
permanent.

## 2. Ground truth from the recon (verified facts, do not re-derive)

- The compile path (`compile_bytecode_function_with` -> `_inner`) makes NO
  lisp-heap allocations and never reads `TAGGED_HEAP` — EXCEPT the two deref
  sites the critics found (see §4 H-A). All `func` inputs (`ops: Vec<Op>`,
  `constants: Vec<Value>`=bit-copyable, params, gnu_byte_offset_map) snapshot
  into Send values.
- Cranelift: `JITModule` IS Send (upstream compile-time assertion in
  cranelift-jit); `cranelift_codegen::Context` Send+Sync. The finished
  artifact can travel.
- `CompiledLeaf` is !Send only via `entry: *const u8` (+ AOT-only sidecar,
  None for JIT leaves). The entry points INTO the Send JITModule -> an
  explicit unsafe-Send wrapper is sound via Box-address-stability +
  process-global code (NOT via the GC HeapPtr's exclusivity argument — write
  the correct justification).
- The obarray is the ONLY thread-affine input: `resolve_inline_callee`
  (MIR inliner), `find_spec_sites` (+ its `leaders` from `analyze_cfg`), and
  `function_epoch` snapshots. All outputs are Send (callee ops/constants
  clones, SpecSite maps, u64 epoch).
- Compile failure is `Result<_, CompileError>` -> `CacheEntry::NotCompilable`
  (never retried) — async mirrors it trivially.
- Inlining is strictly single-level (callee MIR never re-inlined) -> a
  pre-resolved candidate pack is COMPLETE, no transitive under-approximation.
  BUT candidate discovery keys off build_mir's model stack (Const symbol in
  arg position, possibly non-adjacent push) — the enqueue-side pack must run
  the REAL build_mir + discovery, not a naive "Constant-then-Call" scan.

## 3. The adjudicated architecture (v1 = MIR-TIER-ONLY)

The two critiques converged: the BASELINE tier's lowering derefs the live
heap (below), the MIR tier provably does not (`lower_mir_pure` takes
`&MirFunction`, bakes Const bits, never derefs). Therefore:

**v1 scope: only bodies the MIR tier accepts compile in the background;
baseline bodies keep compiling synchronously.** (Honest caveat: MIR bodies
are the FAST compiles — <250us bucket — so v1's stall reduction is small;
its value is proving the machinery. If the trigger that revived this task is
big-baseline-body stalls, go straight to v2's prerequisite work.)

Mechanism (all mandates from the critiques):
1. Worker: lazy singleton thread a la the GC thread BUT with the poison fixes
   — clone the Sender under the OnceLock Mutex and send OUTSIDE the lock
   (the GC pattern's `.lock().expect()` + send-under-guard poisons the mutex
   permanently if the thread dies — fine for a fatal GC, wrong for a
   degradable JIT); `catch_unwind` around each compile; send-failure =>
   compile synchronously inline (graceful degradation).
2. Enqueue (eval thread, at the `try_run_compiled` miss, AFTER the AOT
   consult): run build_mir + inliner eval-side (obarray access stays here);
   if the MIR qualifies for the MIR tier, snapshot the finished MirFunction
   (+ gnu_byte_offset_map + armed epoch) into a Send CompileRequest; ROOT all
   contained Values in an eval-side mirror (PENDING_COMPILE_ROOTS keyed by
   compiled_id) that the GC root collection walks (pattern:
   collect_jit_reloc_gc_roots) and that heap-identity changes CLEAR (a pdump
   reload with a pending compile: stale mirror Values = the R1a-class
   tag-vs-header crash; in-flight requests must be abandoned via an identity
   token checked at install).
3. Pending state: `CacheEntry::Pending` variant (without it: duplicate-enqueue
   storms — every hot call re-runs the AOT consult + re-enqueues — and racing
   installs are undetectable). Plus an `AtomicBool compile_in_flight` on
   `Runtime` checked in `dispatch()` BEFORE the root-save/cache-borrow so the
   in-flight window costs ~1 load per call instead of a RefCell+HashMap
   detour that would be slower than interpreting.
4. Intercept BOTH sync-compile entry points: `try_run_compiled` AND
   `resolve_compiled_leaf_ptr` (the VM's V3 spec fast path compiles
   independently; if it fills the slot while a background compile is pending,
   the install must not clobber — see 5).
5. Install (eval thread, drained at the top of try_run_compiled, strictly
   OUTSIDE any COMPILED/INLINE_DEPS borrow): heap-identity check; the SAME
   staleness rule sync uses (`inline_epoch.is_some() && != live epoch` =>
   discard + clear pending); assemble the CompiledLeaf eval-side from the
   shipped JITModule+entry+metadata; `register_inline_deps` eval-side;
   **insert-if-absent (Entry::Vacant)** — never overwrite (spec slots may
   hold `Rc::as_ptr` of an existing leaf: overwrite = the documented
   prepopulate UAF class); remove the root-mirror entry on EVERY exit path
   (install/discard/NotCompilable/worker-death-timeout — enumerate them; the
   worker dying after accepting a request otherwise leaks that mirror entry
   forever).
6. Off by default: `NEOVM_JIT_ASYNC=1` opt-in. The THRESHOLD=1 differential
   gate stays on the SYNC path (its compile-coverage meaning depends on
   sync); an async=1 gate run is a separate, additional check (results-only
   equality — engagement-COUNT tests are timing-dependent under async and
   must pin async=0 or use a drain_for_test seam; inventory: ~29 eval_test +
   17 aot + 4 compile tests assert sync-compile effects).

## 4. The two UAF classes that killed the naive design (v2's prerequisite)

- **H-A (the headline): the BASELINE compile path derefs the live heap.**
  `analyze_cfg` -> `switch_static_targets` calls `table.as_hash_table()` and
  iterates `ht.data.values()` — a live FxHashMap inside a heap object — for
  every `Op::Switch`, UNCONDITIONALLY (before any obarray gate). Also
  `const_sym_id`'s symbol-with-pos fallback derefs a heap object. `Value` is
  auto-Send (usize newtype), so the type system will NOT catch a worker-side
  deref: (a) heap freed while worker compiles (test heaps drop constantly) =
  UAF; (b) mutator `puthash` on the switch table while the worker iterates =
  data race/UB. **v2 = make the baseline path provably deref-free**: pack
  Cfg + resolved switch targets + SpecSite map + armed epoch + offset map
  eval-side; refactor lower_leaf_full/analyze_cfg to consume the pack; add a
  debug guard (panic-on-veclike-deref while a worker-compile flag is set) so
  the differential gate PROVES deref-freedom. This is a real multi-session
  refactor of the compile core's signatures — budget it as such.
- **H-B: obarray=None divergence.** If the worker just passed None (no pack),
  leaves silently lose inlining + spec sites — strictly less optimized than
  the sync tier, and the async gate would not be validating the same
  artifacts. The pack is mandatory, not optional.

## 5. Refuted worries (do not re-litigate)

Transitive inlining under-approximation (inlining is single-level);
spec-slot dangling at install (SpecSlot.leaf is filled at RUNTIME eval-side,
never baked by the worker); compiled_id lifecycle (monotonic fetch_add,
never reused; clone resets to 0; a reply landing after clone installs under
the dead old id = today's bounded dead-id story); install-vs-heap-swap
interleaving (both eval-thread).

## 6. Tests & gates when built

drain_for_test seam; two-thread stress (enqueue storm + redefinitions + heap
drops); the root-mirror lifecycle unit tests (every exit path); heap-identity
abandonment test (pdump reload with pending compiles); the full differential
matrix sync + the async=1 run; the metering before/after showing the stall
distribution collapsing. Worker-death chaos test (kill the thread, assert
sync fallback + no poison + no mirror leak).
