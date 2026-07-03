# Task 06 — Survival-based tenuring (generational GC, page-grain) — prerequisite design

Status: EVIDENCE-GATED; the PREREQUISITE (page evacuation/retirement for
recurring promotion) is designable now, the feature itself needs a profile.
Risk: HIGH (generational invariants). Effort: prerequisite design ~1 session
+ critique; the full feature multi-session.

## 1. Current state (what "generational" means in neovm-core today)

- ONE-TIME promotion: `promote_and_blacken` runs at the FIRST partitioned
  cycle (post-pdump-load), splicing every young survivor onto
  `tenured_objects` (list objects) and, since arena stage 3, flipping
  `header.tenured` per allocated page slot via the promotion page walk +
  retiring FULL pages (partial pages stay in sweep rotation as MIXED with a
  perpetual tenured-skip). Tenured objects: born-black forever, never
  re-traced (mark_value short-circuits on `header.tenured` BEFORE the parity
  bit — load-bearing order), never swept, freed only at heap Drop.
- Dump->heap and tenured->young edges are caught by `record_heap_write` ->
  `mapped_remembered` (append-only; owners re-traced each cycle at the
  termination reseed).
- Everything allocated AFTER the first cycle stays YOUNG FOREVER and is
  re-marked every cycle. The pacer (live-proportional trigger) and the
  concurrent marker make this cheap in pauses, but mark CPU grows with
  long-session live heaps.

## 2. The trigger

Build this only when a REAL long session shows GC CPU share (concurrent mark
thread time + handshakes + sweep slices) becoming material — the permanent
instrumentation (`gc_collections`, `incremental_mark_us`,
`gc_total_elapsed_us`, `SweepStats`, handshake stats) makes this measurable:
run a day-long real session (or a replay proxy) and chart mark-us per cycle
vs live_bytes. The pause story does NOT need generations (marking is
off-thread; handshakes are ~134us); this is a THROUGHPUT feature only.

## 3. Why the naive extension is forbidden (critic-proven)

Recurring promotion at PAGE grain hits the stage-3 critics' confirmed gap:
a partially-filled page containing both survivors-to-tenure and free slots
cannot simply be flipped:
- If the page's free slots keep feeding the young allocator while the page is
  "tenured", new objects inherit tenured context wrongly (born-tenured =
  never swept = leak + their young children unremembered = child UAF).
- If promotion only flips per-object bits, mixed pages accumulate forever and
  every sweep pays the per-slot tenured-skip on a growing population; the
  one-time version of this is bounded by the loadup set — a RECURRING version
  is not bounded.
- `scan_permanents_for_young_children` (the promotion-time remembered-set
  builder) currently walks `tenured_objects` + (post stage 3) tenured pages;
  its "no young children at promotion" assumption is ALREADY false for cons
  children (conses never tenure — stage 3 added the page walk for exactly
  this, proven by a UAF test). A recurring promotion multiplies exposure.

Sound page-grain recurring promotion therefore needs ONE of:
- **RETIREMENT-ONLY (cheap, lossy):** promote only FULL pages (occupancy ==
  slots, all survivors); partial pages' survivors simply stay young another
  round. Converges if allocation is generationally clustered (it usually is);
  degenerate case: fragmentation keeps survivors young forever — measure.
- **EVACUATION (real, expensive):** copy survivors from partial pages into
  compacting tenured pages, then recycle the emptied young page. Copying
  MOVES OBJECTS — which this GC has explicitly and correctly rejected
  globally (non-moving is a foundational invariant: raw Value bits are baked
  in JIT code, held in Rust structures, snapshot buffers, etc.). Evacuation
  is therefore ONLY thinkable for objects with NO ambient raw references —
  which in this VM is provable for NOTHING cheaply. **Effectively: evacuation
  is OFF THE TABLE; the design space is retirement-only + sticky mark bits.**

## 4. The realistic design (sticky-mark minor cycles, non-moving)

The non-moving generational scheme that fits this collector:
1. Add a per-cycle mode: MINOR (default) vs MAJOR (occasional). MINOR cycles
   do NOT flip/clear old-generation state and do NOT trace into objects whose
   header.tenured is set (already true); the NEW part is a second "aged"
   young state: an object surviving K minor cycles (K=1 or 2, sticky parity
   bits can encode "survived last cycle" as bit==old-parity at flip time —
   the parity flip already gives one bit of age for free!) gets
   header.tenured set IN PLACE (no motion) — page-grain bookkeeping via an
   occupancy-of-tenured counter per page; pages crossing the all-tenured
   threshold retire (stop being swept), partial pages keep the tenured-skip.
2. The write barrier must then catch ALL tenured->young edges, not just
   dump-era ones: `record_heap_write` already fires on every heap write with
   `value_is_tenured(owner)` routing to `mapped_remembered` — VERIFY the
   barrier's owner-classification covers newly-tenured objects (it gates on
   ownership + tenured flag — it does), and that `mapped_remembered`'s
   append-only growth gets a companion: entries whose owner's young children
   have all tenured could be dropped — the recorded correct shape is
   "tenure-the-young-children + drop-owner-until-redirtied", which needs its
   own design round (a high-water mark alone is UNSOUND — young children need
   re-marking every cycle; this was explicitly proven in the rootscan
   analysis).
3. MAJOR cycles (rare, e.g. every N minors or on memory-full pressure): treat
   everything as young — clear tenured bits page-by-page? NO — that
   reintroduces the clear walk. Better: majors keep tenured objects tenured
   but re-verify via the partition verifier machinery; reclamation of dead
   TENURED objects is the actual purpose of a major — options: (i) never (the
   current model — tenured garbage is a permanent leak, acceptable for
   loadup-era data, NOT acceptable once tenuring is survival-based and
   routine); (ii) a major = full STW mark of the tenured generation with its
   own parity plane (a SECOND bit) + page sweep of tenured pages. The second
   bit exists in the header padding (GcHeader has 5 pad bytes) — a
   `tenured_marked: AtomicBool` plane makes majors independent of minor
   parity. This is the core new machinery of the feature.
4. The pacer needs a second knob: minor trigger (current live-proportional)
   + major trigger (tenured-bytes growth since last major).

## 5. Hazards to hand the critics (when the trigger fires)

- The barrier completeness proof for tenured->young after IN-PLACE tenuring
  (H1 — the whole scheme rests on it; enumerate every write path that can
  store a young ref into a tenured object incl. vectors' COW path, string
  intervals, record/closure slots [non-atomic!], obarray cells [seqlock
  path], bytecode constants [immutable — verify]).
- Sticky-parity age encoding vs allocate-black (a mid-cycle-born object reads
  bit==parity — identical to a survivor; age-K must not tenure objects born
  during the cycle; the link-seam knows the difference — encode it).
- Weak tables + finalizers vs tenured garbage (a tenured weak VALUE never
  collected keeps entries alive forever until majors exist).
- The permanent_weak_hash_tables / finalizer_registry / mapped_remembered
  interaction with newly-tenured objects.
- The dirty_owners ABA (task 10) MUST be fixed first — this feature is the
  consumer that arms it.

## 6. Measurement protocol

Before: mark-us/cycle vs live-bytes chart on a long session; after: same
chart + minor/major split + tenured-generation size + floating-garbage bound
(tenured-dead bytes between majors). Success: mark CPU flat as live heap
grows, pauses unchanged, no leak growth beyond the major cadence bound.
