# Task 03 — Remaining Box-type arena migrations (ByteCode -> Markers -> HashTables/rest)

Status: HAZARDS FULLY MAPPED (two critique rounds covered these types while
reviewing stages 1-3). Risk: MEDIUM (bytecode) to HIGH (markers, hash tables).
Effort: bytecode ~1 session; markers ~1 session with its own critique; hash
tables last and only with a fresh critique. Prerequisite reading: the arena
commits ("gc: float arena pages...", "gc: generalize float pages to
size-class ObjectArena...", string/vector/promotion commits) and the
`ObjectArena`/`ObjectPage` code in `tagged/gc.rs`.

## 1. Where we are

Floats (32B slots), Strings (64B), Vectors (64B) live on 64KB size-class
pages with: page-span ownership (base-registry + stride + ALLOC-BIT — all
three, always), full-header `ptr::write` on slot reuse, born-at-parity,
allocated-bit-first page sweeps in BOTH sweep entry points with incremental
cursors, `drop_in_place` before bit-clear for payload-bearing types, page-walk
promotion + full-page retirement (retired pages STAY in the ownership oracle
— C1, the true UAF otherwise), and rewritten verifiers
(`assert_object_arenas_coherent`, tricolor/partition page walks).

Measured after stage 3: float alloc/free 22.3/1.3ns, vector 20.9/5.8ns,
string 76/16.6ns; the ownership FxHashSet is gone for ~86% of objects by
count; zero pause regression; drain deferred median unchanged at ~7.3k
(bytecode ~4.2k + subr ~1.6k of it — see task 01, which shares this
population).

Remaining Box-allocated kinds (still in `non_cons_object_addrs`, still on the
intrusive `all_objects`/`tenured_objects` lists, still freed via
`free_gc_object`'s `Box::from_raw`): ByteCode (360B fixed), HashTable (264B),
CharTable (592B) / SubCharTable, Lambda/Macro (112B), Record (48B), Marker
(88B), Overlay (80B), Obarray, Bignum, SymbolWithPos (40B), Buffer/Window/
Frame/Timer/Process/Sqlite/UserPtr/ModuleFunction/Xwidget*, Subr (leaked,
special), Finalizer.

## 2. Why continue (and why NOT to rush all of it)

- Alloc-rate evidence (the `alloc_probe` histograms): after strings+vectors,
  the remaining high-RATE class is **ByteCodeObj** — interpreted-lambda
  instantiation legitimately allocates 2 x 360B per lambda via upstream
  cconv (`cconv-make-interpreted-closure`; investigated and exonerated as
  GNU-parity — see task notes #10), reaching ~40K/20K-iterations in churn
  workloads and 17K at startup. Lambda/Macro (112B) is the companion
  (replay-nadvice measured 15.9K Lambda allocs).
- Sweep/clear costs for these types are already mostly O(young population)
  via the intrusive-list walks that the parity work stopped CLEARING but
  sweeps still traverse; page-walking them is the same 8-57x visit win
  measured for the first three classes.
- BUT: each type carries its own termination/lifecycle couplings. The stage-3
  critics explicitly flagged the ones below. The value declines steeply after
  ByteCode+Lambda — records are only 380 allocs at startup; hash tables 567.
  **Recommended scope: ByteCode + Lambda/Macro + Record (+SymbolWithPos) in
  one arc; markers only if their alloc rate on real sessions justifies the
  chain complexity; hash tables and the exotic types probably NEVER (their
  value is ~nil and their coupling is the worst).**

## 3. Per-type dossier

### 3a. ByteCodeObj (360B -> 384B class) — DO FIRST
- Payloads: `ops: Vec<Op>` (Rust-only), `constants: LispValueVec`, misc.
  REAL `Drop` -> page sweep must `drop_in_place::<ByteCodeObj>()`; page Drop
  walks allocated slots (the stage-3 `needs_drop`-gated walk generalizes).
- Children: constants (Lisp values) — traced by `trace_veclike`'s ByteCode
  arm; the collect⊇trace invariant must keep its arm consistent (check
  `collect_veclike_children`).
- JIT coupling: `ByteCodeFunction.runtime` (heat/compiled_id) lives WHERE?
  Verify whether the Runtime struct is inside the heap object or the chunk —
  the JIT cache keys by `compiled_id` and never dereferences dead objects
  (monotonic ids), but confirm nothing holds a raw `*const ByteCodeObj`
  across GC (the `&'static` borrow in `run_loop` is rooted by the executing
  frame — an EXECUTING bytecode object is always reachable; state it).
- Immutability: bytecode constants should be immutable post-publish; task 01
  (4c) wants this proven anyway — proving it here once serves both.
- Tenuring: bytecode objects ARE loadup-heavy -> most tenure at the one-time
  promotion -> page retirement will fire for real here (unlike floats).
  The stage-3 machinery (promotion page walk + full-page retirement +
  mixed-page tenured-skip) generalizes without new design.
- Slot layout: 360B fixed -> 384B stride, link in bytes 360..368 — add the
  const-asserts (`size_of::<ByteCodeObj>() <= 360` etc.). If the struct is
  bigger than believed, bump the class and say so — do not squeeze.

### 3b. Lambda/Macro (112B -> 128B class) + Record (48B -> 64B) + SymbolWithPos (40B -> 64B or 48B)
- Same shape as vectors: `LispValueVec` payloads (Lambda/Macro/Record),
  `drop_in_place` required; Record/closure slot WRITES are non-atomic plain
  stores — IRRELEVANT for paging (paging changes allocation, not tracing;
  they remain deferred-at-termination for marking — see task 01 §4d).
- The stage-3 oracle/verifier/sweep machinery generalizes directly; these are
  "add a class" commits. Group them after 3a in one gated commit each.

### 3c. Markers (88B) — ONLY WITH ITS OWN CRITIQUE ROUND
Two couplings the stage-3 mechanics critic flagged as disqualifying-for-now:
- `MarkerObj.data.next_marker: *mut MarkerObj` — an INTRUSIVE cross-marker
  chain threaded through the owning buffer. Slot reuse must not corrupt a
  chain that other markers still link through; the full-header rewrite is
  NOT sufficient — the chain must be unlinked BEFORE the slot is freed.
- ORDERED sweep dependency: `unchain_dead_markers` must splice dead markers
  out of buffer chains BEFORE they are freed (it runs at termination, before
  sweep). A page sweep freeing a marker whose chain-splice hasn't happened
  breaks the invariant. Today the ordering holds because markers are freed by
  the LIST sweep which runs strictly after termination — a page sweep is also
  post-termination, so the ordering may in fact hold trivially; but the
  critic's point stands: PROVE it, per path (eager + incremental + Drop),
  including a marker freed by Drop mid-chain.
- Value check first: measure marker alloc rates on a real session (profilers
  + the alloc_probe histogram). Startup measured only 333 markers — if real
  sessions don't allocate markers in bulk, skip this type entirely.

### 3d. HashTables (264B -> 320B) — LAST OR NEVER
- `permanent_weak_hash_tables` holds raw pointers to tenured hash tables
  forever (append-only, never removed). A tenured hash table on a page that
  could ever be recycled would dangle them — retirement's freed-at-Drop-only
  rule covers it, but the coupling makes hash tables the type with the least
  margin. The weak-table termination protocol (registration during trace,
  fixpoint at termination) reads them at exactly the wrong times for
  mistakes.
- 567 allocations at startup. The win is ~nil. Recommendation: leave on Box
  permanently; document.

### 3e. Everything else (Buffer/Window/Frame/Process/UserPtr/ModuleFunction/...)
- C-finalizer obligations (UserPtr/ModuleFunction run finalizers inside
  `free_gc_object` BEFORE drop), singleton-ish populations, no rate.
  Leave on Box; the residual addr-set exists precisely for them.

## 4. Implementation plan

Commit 1: ByteCode pages (3a) — class + const-asserts, alloc chokepoint
switch (find the single alloc site; keep the addr-set OUT, lists OUT per the
standing traps), drop_in_place sweep arms, page-Drop walk, verifier class
addition, tenure/retire tests (bytecode is the first class that MEANINGFULLY
retires — assert retired pages stay owned + skipped), alloc_probe before/after.
Commit 2: Lambda/Macro. Commit 3: Record + SymbolWithPos.
Commit 4 (conditional): Markers, ONLY after (i) a real-session rate
measurement justifies it and (ii) a dedicated two-critic round on the chain +
unchain ordering.
Explicit non-goal commit: a doc note in gc.rs stating hash tables + id-ref
types stay Box-resident and why.

## 5. Standing traps (from the stage-1..3 critiques — all still armed)

- TRAP A: never remove a type from the addr-set without its page-span oracle
  arm landing IN THE SAME COMMIT (the mapped-fallback in `mark_value` turns
  ownership misses into "mapped" -> dropped marks -> UAF).
- TRAP B: page objects must NEVER enter `all_objects`/`tenured_objects`/
  `sweep_noncons_pending` (`free_gc_object` is Box-only; `Box::from_raw` on
  page memory = heap corruption).
- Tenured-skip BEFORE parity in every page sweep arm (frozen tenured bits
  read "unmarked" on alternate parities).
- Full-header `ptr::write` on slot reuse (stale kind = type-confused Drop).
- Variable-byte accounting into BOTH live-bytes recompute sites (live_bytes
  feeds `effective_gc_threshold_bytes` — the pacer).
- Retired pages STAY in the ownership oracle (C1).
- The `dirty_owners` ABA (task 10) becomes more urgent with every migrated
  type — land task 10 before or with this work.

## 6. Gates & measurement

Per commit: tagged/finalizer matrix x4 + clippy + the alloc_probe class
histograms + `gc_drain_kinds_profile_pdump` (no pause regression; for
bytecode, expect the drain unchanged — marking is task 01's job, not this
one's) + full suite at arc end. Report per-class alloc/free ns and the
addr-set peak-size drop (was ~95K before stage 1; measure what remains).
