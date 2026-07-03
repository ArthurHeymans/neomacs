# Task 10 — Fix the dirty_owners latent ABA (BEFORE it gets a consumer)

Status: SMALL + PRECISE; land opportunistically, and MANDATORY before task 06
(survival tenuring) or anything else consumes `dirty_owners`. Risk: LOW (the
fix); the ABA it prevents is a future-UAF class. Effort: hours.

## 1. The finding (from the arena stage-3 GC critique — CONFIRMED latent)

`TaggedHeap.dirty_owners` / `dirty_owner_bits` / `dirty_writes` (the
Steele-style owner-tracking side tables in `tagged/gc.rs`) are:
- populated UNCONDITIONALLY in `record_heap_write` (every barriered heap
  write), keyed by non-cons OWNER address bits,
- cleared only at END-of-collection,
- NEVER evicted on object free.

During the cooperative deferred sweep, the mutator keeps running (and keeps
firing `record_heap_write`) BETWEEN sweep slices while the sweep frees
objects. Sequence: owner O is freed by a slice; its address lingers in
`dirty_owners`; the arena allocator (same-class slot reuse is now
near-certain, not allocator-random) hands O's address to a NEW object O';
O' takes a barriered write; the dedup sees the stale entry and SKIPS
recording O''s write.

TODAY this is INERT: the only readers of `take_dirty_owners`/the bit tables
are tests — no production consumer exists (the live marking barrier is SATB;
the dirty-owner tables are the "minimal remembered-set precursor" kept for
the generational future). The moment task 06 (or anything) makes them
load-bearing, the ABA becomes: a skipped remembered-write -> a tenured->young
edge missed -> young child swept while referenced -> UAF.

## 2. The fix (pick one, both acceptable)

- Option A (evict-on-free): remove the owner's entry in the same places the
  ownership bookkeeping is updated at free — the page sweeps' free hooks and
  `free_gc_object` for Box objects. Mirrors the `vector_object_addrs`
  evict-before-free discipline the critics verified as the ABA-safe pattern.
  Cost: a hash remove per freed non-cons object (only while the tables are
  non-empty — gate on a `!dirty_owners.is_empty()` fast check).
- Option B (clear-at-begin): clear the tables in `begin_collection` alongside
  the per-cycle SATB set clears (they are per-cycle data by design; the
  no-free-during-mark invariant then makes in-cycle reuse impossible, which
  is the exact argument that makes the SATB dedup sets ABA-safe).
  Cost: nothing per-free; semantics change from "since last collection END"
  to "since last collection START" — verify no test depends on the
  cross-window contents (the readers are tests; update them).

Option B is cheaper and aligns the tables with the SATB sets' proven
lifecycle — RECOMMENDED, unless the future consumer (task 06) needs
cross-cycle accumulation, in which case A.

## 3. Tests

(a) A regression test encoding the ABA: fill dirty_owners via writes to O;
free O (drive a sweep); reallocate into the same slot (arena same-class
reuse makes this deterministic — allocate until the address recurs, assert it
does); write to O'; assert the tables record O''s write (would fail under
the stale-dedup). (b) The existing owner-tracking tests updated for the
chosen lifecycle. Gate: tagged matrix x4 + clippy.

## 4. Note for reviewers

Do NOT be tempted to delete the tables outright "since nothing reads them" —
they are the designed seam for task 06's remembered set, and the recorded
analysis (rootscan work) already proved the shape their consumer needs
("tenure-the-young-children + drop-owner-until-redirtied"; a high-water mark
alone is UNSOUND because young children need re-marking every cycle). Keep
the seam; fix its lifecycle.
