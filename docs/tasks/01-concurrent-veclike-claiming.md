# Task 01 — Concurrent veclike claiming (move the remaining deferred drain off the STW termination)

Status: DESIGN READY (newly unblocked). Risk: HIGH (concurrent GC surgery — this
is the class of change where every prior critique round found real UAFs).
Effort: 1-2 focused sessions incl. a mandatory two-critic design review.
Prerequisite reading: `neovm-core/src/tagged/gc.rs` (the whole concurrent
machinery), `CONCURRENT_GC.md`, and doc 03 (shares the drain-kind evidence).

## 1. Why (measured evidence)

The concurrent collector defers every non-cons object the GC thread discovers
to the stop-the-world termination drain, EXCEPT the kinds that have been
individually made concurrent-safe: conses (atomic block bitmaps), obarray
symbol cells (per-chunk seqlock), Vector BACKINGS (Tier-B snapshot +
clone-on-write), and — since the string work — interval-free strings (atomic
claim + null-check). Everything else parks in the shared `deferred` buffer and
is traced at the pause.

Measured (release, `gc_drain_kinds_profile_pdump`, the per-kind classifier on
the `concurrent_termination` trace line):

- Before string claiming: deferred median ~30,485 objects/cycle; drain median
  3,153-3,297us — the DOMINANT term of the termination pause.
- After string claiming: deferred median ~7,344/cycle, drain median ~180-221us.
- The REMAINING 7.3k breaks down (str bucket now = interval-BEARING strings
  only, 212): **ByteCode ~4,244 (14-19%), Subr ~1,620 (5.3%), Vector ~279
  (header marks — see §3), records/closures/hash-tables <3% combined**, plus
  floats/markers/others.

So the drain is already sub-millisecond in the shipping config. This task is
the FINISHING move: claim the remaining safe kinds concurrently so the drain
approaches the fold-only floor (~60-90us). It is justified as the completion
of "zero-pause", not as a large absolute win — set expectations accordingly,
and re-run the profiler FIRST to confirm the numbers still hold on current
main.

## 2. What unblocked it

Two prior blockers are gone:

1. **Ownership.** The GC thread historically could not answer "is this pointer
   an owned heap object?" for non-cons kinds (the ownership oracle was a
   mutator-side `FxHashSet` it may not touch). Strings worked around it with
   the dump-span test. The arena work (commits "gc: generalize float pages to
   size-class ObjectArena..." through "gc: promotion page walk + full-page
   retirement") gives EXACT page-span ownership for floats, strings, vectors —
   and the string-claim path already consumes a **start-of-cycle immutable
   page-base snapshot** (built beside `owned_bases` in
   `launch_concurrent_mark`, published via the same Arc/channel
   happens-before). Extending the snapshot to more classes is mechanical.
2. **Mark bits.** `GcHeader.marked` is an `AtomicBool` with a parity
   interpretation (`is_marked_at(parity)` / `mark_claim_at(parity)`, added by
   the parity-mark-bits commit). The claim primitive is proven in production
   by the string path (`concurrent_try_mark_string`), including the
   correct memory-ordering discipline and the lost-claim-race-is-benign
   argument.

## 3. The Tier-A trap — read this twice

An earlier attempt ("Stage 2 Tier A", recorded in project memory) concurrently
marked the "no-Lisp-children" types (Buffer/Marker/Process/...) and produced
REAL CORRUPTION (8 partition-verify failures) because:

> "trace_veclike is a no-op for kind K" does NOT imply "K can skip the defer".
> The termination path performs KIND-SPECIFIC processing beyond marking —
> e.g. markers participate in per-buffer chains that `unchain_dead_markers`
> must splice BEFORE sweep; deferring is what guarantees the termination-time
> invariants those routines assume.

The string work survived because strings' termination processing was PROVEN to
be exactly {mark + trace intervals} and nothing else, and interval-bearing
strings still defer. **Every kind this task claims concurrently needs the same
per-kind inventory, in writing, verified by a critic.**

Also note the Vector bucket finding from the drain classifier: Tier-B
pre-traces vector BACKINGS' children concurrently, but each vector still costs
one termination `mark_value` because the GC thread never set the vector's own
header mark. That is exactly what this task fixes for vectors — the children
are already handled; only the header claim moves.

## 4. Target kinds, in order of safety

Work through these in separate commits, each with its own inventory + tests:

### 4a. Vector header claims (safest; children already concurrent)
- The GC thread discovers a TAG_VECLIKE value; classify via the vector-page
  snapshot (exact). If `type_tag == Vector` and owned: `mark_claim_at(parity)`;
  claimed -> done (its backing's children are already covered by the Tier-B
  snapshot scan + SATB clone-on-write); already-marked -> skip; snapshot miss
  -> DEFER (fail-safe, unchanged).
- Inventory to verify: does ANY termination logic distinguish "vector deferred
  and marked at drain" from "vector marked earlier"? Check: weak tables (keyed
  by vectors — `keep_weak_entry` reads final marks at termination, mark timing
  irrelevant — same argument as strings, restate it), `verify_dump_partition`
  / `verify_incremental_tricolor` (read marks post-drain; the tricolor
  verifier must see the vector's children non-white — they are, via Tier-B +
  SATB), the drain-kind classifier (diagnostics only), live-bytes recompute
  (reads final marks at sweep).
- CAUTION: a vector whose BACKING was snapshot-missed (allocated mid-cycle) is
  born-black and its contents born-reachable-elsewhere — but a mid-cycle
  vector is NOT in the vector-page-base snapshot -> defer -> fine. A vector in
  the snapshot whose backing GREW mid-cycle: clone-on-write retires the old
  backing; the GC thread's Tier-B scan reads the retired original; claiming
  the header is orthogonal. Argue this in the design doc you submit for
  critique.

### 4b. Float claims (POD, zero children, zero termination processing)
- Trivially the same shape via the float-page snapshot. The reason floats were
  NOT claimed in the arena v1 was to keep that increment minimal — the
  standing mandate "never mark-as-mapped on a snapshot miss; miss => defer"
  applies verbatim.

### 4c. ByteCode + Subr (the big count — 14-24% of the drain)
- ByteCodeObj is IMMUTABLE post-construction (the old Workstream-D analysis
  already identified immutable ByteCode as the one veclike safe without
  retired-buffer machinery). Its children (ops vec is Rust-only; `constants`
  are Lisp values in a `LispValueVec`) — the constants BACKING has the same
  concurrent-read question as vectors. Options: (i) claim the header AND
  trace constants via the same snapshot/atomic-read discipline Tier-B uses
  for plain vectors (requires adding bytecode backings to the Tier-B
  snapshot); (ii) claim the header, PUSH THE CHILDREN to a GC-thread-local
  gray extension... they're Lisp values reachable from an immutable backing —
  since ByteCode constants never mutate post-publish (verify! grep every
  `constants` mutation site; `replace_*` functions exist for vectors — do any
  target bytecode?), a plain non-atomic read is a data race in the Rust model
  even if bit-stable. The SOUND cheap route: extend the Tier-B snapshot to
  ByteCode backings (they are `LispValueVec` like vectors; the clone-on-write
  hook `with_vector_data_mut` must then also cover any bytecode mutation
  path, or you must PROVE there is none and document the immutability as a
  hard invariant with a debug assertion in the mutator).
- SubrObj: leaked-static, `function: Option<SubrFn>` + ids; children none
  (name is a NameId, not a Value — verify). Claim-only. But Subr objects are
  REGISTERED IN-PLACE-REWRITABLE (`update_static_subr_object_entry`) — that
  rewrites function/arity fields, NOT Lisp children; marking concurrently is
  orthogonal. State it.

### 4d. Explicitly NOT in scope (keep deferring)
- Interval-bearing strings (position-dependent; already decided).
- Records/closures/Lambda/Macro: <1% each of the drain AND their slot writes
  (`set_record_slot`/`set_closure_slot`) are NON-ATOMIC plain stores — the
  same reason they were excluded from Tier B. Not worth the atomicization for
  <3% combined.
- Hash tables, char-tables, obarray internals, markers, buffers, processes,
  windows, frames, overlays: termination-specific processing (weak protocol,
  marker chains, etc.) — the Tier-A graveyard. Do not touch.

## 5. Implementation sketch

1. **Re-measure first** (`gc_drain_kinds_profile_pdump/_plain` on current
   main): confirm the kind ranking; if bytecode+subr+vector no longer
   dominate, stop and re-rank.
2. Generalize the string-claim seam: `concurrent_try_mark_string` becomes a
   dispatcher `concurrent_try_mark_owned(value, job)` with per-tag arms
   (string / float / vector-header / bytecode / subr), each arm carrying its
   own snapshot-classify + claim + (kind-specific) children step, and each
   REFUSING (-> defer) anything not provably its case. Wire it at all three
   GC-thread discovery sinks (gray drain, obarray scan, vector-backing scan —
   the string work already touches all three; extend the match).
3. ConcurrentMarkJob grows the per-class page-base snapshots (float + vector
   + bytecode-if-4c). The snapshots are built where `owned_bases` is built
   (world-stopped start handshake) — same Arc publication.
4. Tier-B snapshot extension for bytecode backings (4c only), including the
   clone-on-write audit (see hazard H3).
5. Per-kind counters on the drain classifier (`str_claimed` pattern) so the
   measurement shows exactly which bucket shrank.

Commit structure: (1) dispatcher refactor, behavior-identical (strings only);
(2) vector header claims; (3) float claims; (4) bytecode+subr (with the
backing-snapshot work); each with the full gate matrix.

## 6. Hazards & invariants (the critique round must cover at least these)

- H1 (Tier-A): the per-kind termination-processing inventory, per kind, with
  file:line proof of "mark + children only".
- H2 (snapshot-miss direction): every arm must DEFER on any classification
  miss. Never "miss => mapped" (a mid-cycle heap object misclassified as
  mapped is a dropped mark = UAF — this was a confirmed hazard in the string
  design and the fix is recorded at the claim site).
- H3 (backing mutation): for any kind whose CHILDREN the GC thread reads
  (vector backings today, bytecode backings if 4c): every mutator write path
  must either go through the clone-on-write/retired-buffer discipline or be
  proven nonexistent. The historical grow-realloc TOCTOU (a backing realloc'd
  mid-read) is the named UAF; clone-on-write was the fix.
- H4 (parity): claims are `mark_claim_at(parity)` with parity from the job;
  a claimed-then-deferred object must NOT happen for kinds whose termination
  trace would early-return on the mark bit (the string work's
  null-check-BEFORE-claim ordering exists precisely because claim-then-defer
  skips the interval trace — generalize: any arm that can still defer after
  inspecting the object must do its inspection BEFORE claiming).
- H5 (retired/tenured pages): the page snapshots include retired pages
  (ownership invariant from the arena work); tenured objects short-circuit
  before parity everywhere — the claim arms must respect the same order
  (tenured check first; tenured => skip, it's already permanently live).
- H6 (weak tables): argue per kind that early marking cannot change weak
  retention (the argument that worked for strings: the concurrently-claimed
  set has no path into weak-entry enumeration on the GC thread; weak decisions
  read final marks at termination).

## 7. Tests

Per kind: (a) an object of that kind reachable ONLY via a rooted cons chain is
claimed concurrently (counter delta) and survives; (b) same object garbage ->
collected within two cycles; (c) mid-cycle-allocated object of the kind ->
deferred (counter shows no claim) and correctly resolved at termination; (d)
for 4c: a bytecode object's constant child, otherwise unreachable, survives a
cycle where the bytecode was claimed concurrently (children coverage); (e) the
existing concurrent-mark race tests extended to the new arms; (f) everything
under NEOVM_GC_VERIFY_PARTITION=1 (the tricolor verifier is the oracle) and
NEOVM_GC_STRESS=1 on the tagged filter.

## 8. Gates & measurement

Gate matrix per commit: `-E 'test(/tagged::|finalizer/)'` x {plain, VERIFY,
STRESS, combined}; clippy-clean diff; full suite at the end.
Measurement: `gc_drain_kinds_profile_pdump/_plain` before/after per commit —
report deferred/cycle per kind, drain_us, fold_us. Success criterion: pdump
drain median approaches the fold floor (~60-90us) with kind buckets for the
claimed types collapsing to ~0; NO regression in start handshake (the extra
snapshots add O(pages) work — measure `conssnap`/`vecsnap`-style timers for
the new snapshots and keep the start total under ~150us pdump).

## 9. Go/no-go

GO if the re-measured drain still shows bytecode+subr+vector as the bulk of
the remaining deferred set. NO-GO (or descope to 4a+4b) if the ranking
changed, or if the 4c immutability/COW audit finds bytecode backings mutable
through paths that cannot be cheaply covered — in that case ship vectors +
floats + subrs only and record the residual.
