# SizeClassAllocator for neovm-gc PinnedSpace

**Date**: 2026-04-17
**Status**: Design
**Reference**: Go GC span model

## Problem

neovm-gc's PinnedSpace allocates each object via `std::alloc::alloc` (Box::new
equivalent). With 780K objects, this causes 1.3GB RSS due to per-allocation
system allocator overhead. The current workaround (arena bump allocation in
TaggedHeap) achieves 116MB RSS but bypasses neovm-gc's ObjectStore, so no
garbage collection is possible.

## Goal

Replace PinnedSpace's per-object system allocation with Go-style size-class
spans. Objects are bump-allocated from spans, tracked via per-span mark
bitmaps, and reclaimed via free-lists. No per-object ObjectStore entries.
neovm-gc's ConcurrentMarker, pacer, and write barriers continue to work
unchanged.

## Design

### Size Classes

Objects are rounded up to the nearest size class. Each size class has its own
pool of spans. The size classes cover the range of Lisp object sizes (all
include the 56-byte ObjectHeader):

```
Class  Slot size  Typical payload
  0       64B     GcFloat (8B payload)
  1       72B     GcCons (16B), GcBuffer/Window/Frame/Timer (16B)
  2       80B     (padding class)
  3       96B     GcVector/GcLambda/GcMacro/GcSubr (32B payload + alignment)
  4      128B     GcLispString (short), GcSymbolWithPos
  5      192B     GcHashTable (small), GcByteCode (small)
  6      256B     Medium strings/vectors
  7      384B     Larger strings
  8      512B     
  9      768B     
 10     1024B     
 11     2048B     Large objects below the individual-alloc threshold
```

Objects larger than 2048B are allocated individually via `std::alloc::alloc`
(rare — less than 0.1% of objects).

### Spans

A span is a contiguous block of memory holding fixed-size slots of one size
class.

```rust
struct Span {
    /// Base pointer to the allocated block.
    base: NonNull<u8>,
    /// Size class index.
    size_class: u8,
    /// Slot size in bytes (includes ObjectHeader).
    slot_size: u32,
    /// Total number of slots in this span.
    slot_count: u16,
    /// Number of slots allocated (bump cursor for fresh spans).
    allocated: u16,
    /// Mark bitmap: 1 bit per slot. Set during mark phase, cleared during sweep.
    mark_bits: Vec<u64>,
    /// Free list head: index of first free slot, or u16::MAX if none.
    free_head: u16,
    /// Generation: Young or Old. Starts Young, promoted after surviving GC.
    generation: Generation,
}
```

**Span size**: 32KB per span (configurable). A 32KB span with 72B slots holds
455 cons cells. With 64B slots: 512 objects.

**Memory layout within a span**:
```
[ObjectHeader|payload][ObjectHeader|payload][ObjectHeader|payload]...
 ←── slot_size ────→ ←── slot_size ────→
```

No per-slot overhead beyond the ObjectHeader (which is already needed for
type dispatch and mark bits).

### Allocation

```
alloc(size) → *mut u8:
  1. Compute size_class for requested size
  2. Check free_list for the size class's current span
     → If free slot available: pop from free list, return slot pointer
  3. Check bump cursor on current span
     → If space: bump cursor, return slot pointer
  4. Allocate new span (32KB), set as current span for this size class
     → Bump first slot, return pointer
```

Allocation is O(1) — either a free-list pop or a pointer bump.

### Free List

Dead slots are threaded into an intrusive free list through the slot memory
itself (same technique as GNU Emacs cons_free_list and the old TaggedHeap):

```
Dead slot:
[next_free: *mut u8][garbage...padding to slot_size]
 ↓
[next_free: *mut u8][garbage...padding to slot_size]
 ↓
null
```

The first 8 bytes of a dead slot store the pointer to the next free slot.
Since all slots in a span are the same size, any dead slot can be reused by
any new object of the same size class.

### Mark Bitmap

Each span has a mark bitmap: 1 bit per slot. For a 32KB span with 72B slots
(455 slots): 455 bits = 8 words = 64 bytes of bitmap.

```
mark_bits: [u64; (slot_count + 63) / 64]

mark(slot_index):    mark_bits[index / 64] |= 1 << (index % 64)
is_marked(index):    mark_bits[index / 64] & (1 << (index % 64)) != 0
clear_all():         mark_bits.fill(0)
```

The mark bit in ObjectHeader.mark_bits is still set by the ConcurrentMarker
(unchanged). During sweep, the span walks its slots and checks
ObjectHeader.mark_bits to build the free list. The per-span bitmap is
optional — we can use ObjectHeader.mark_bits directly and skip the bitmap.

**Decision: use ObjectHeader.mark_bits directly.** No separate bitmap needed.
The ConcurrentMarker already sets ObjectHeader.mark_bits. Sweep walks slots
sequentially and checks each header. This is simpler and avoids duplicating
mark state.

### Sweep

After marking completes:

```
sweep(span):
  free_head = NONE
  for slot_index in 0..span.allocated:
    header = span.slot_ptr(slot_index) as *ObjectHeader
    if header.is_marked():
      header.clear_mark()      // ready for next cycle
    else:
      // Dead object — drop payload, add to free list
      if header.desc().needs_drop:
        (header.desc().drop_in_place)(payload_ptr)
      write slot as free_list_entry(free_head)
      free_head = slot_index
  span.free_head = free_head
```

**Payload drop**: when a dead object's slot is reclaimed, the payload
destructor runs (e.g., Vec::drop for GcVector's backing buffer). The slot
memory stays in the span for reuse.

### Span Lifecycle

```
New span → Young, bump allocating
  ↓ (survives minor GC)
Promoted → Old, free-list recycling
  ↓ (all slots dead after GC)
Empty → returned to OS via munmap/madvise(MADV_FREE)
```

**Returning empty spans to OS**: when sweep finds ALL slots dead in a span,
the entire span is freed. This prevents unbounded memory growth in long-
running sessions where allocation patterns shift between size classes.

### Generational Collection

**Per-object generation bit** in ObjectHeader (reuse the existing `generation`
field):

- New objects: `Generation::Young`
- After surviving one GC: promoted to `Generation::Old` (just flip the field)

**Minor GC** (frequent, sub-millisecond):
1. Scan root slots
2. Mark reachable young objects (skip old objects during trace — they're assumed live)
3. Scan dirty cards (old objects that wrote pointers to young objects)
4. Sweep young objects only: dead → free-list, surviving → promote to Old
5. Clear dirty cards

**Major GC** (infrequent, concurrent):
1. Background ConcurrentMarker traces ALL objects (young + old)
2. Brief STW remark (handle mutations during concurrent mark)
3. Sweep ALL objects: dead → free-list or span return

### Card Table

For generational old-to-young tracking, each span has a card table. A card
covers 512 bytes of span memory.

```
32KB span / 512B per card = 64 cards per span = 8 bytes of card table
```

When a write barrier fires (old object stores pointer to young object), the
card covering the old object's span position is marked dirty. Minor GC
scans dirty cards to find additional roots in old objects.

neovm-gc's existing write barrier infrastructure
(`Mutator::post_write_barrier`) handles the barrier call. The card table
storage moves from old-gen blocks to spans.

### Integration with neovm-gc

**What changes**:

| Component | Change |
|-----------|--------|
| PinnedSpace | Replace 15-line stub with SizeClassAllocator |
| Mutator alloc (pinned path) | Call SizeClassAllocator instead of Box::new |
| ObjectStore | NOT USED for span-allocated objects |
| Sweep/reclaim | Walk spans instead of ObjectStore for pinned objects |
| HeapStats | Count span-allocated bytes from SizeClassAllocator |

**What stays unchanged**:

| Component | Why unchanged |
|-----------|---------------|
| ConcurrentMarker | Follows Trace edges, sets ObjectHeader.mark_bits — doesn't care where objects live |
| Pacer | Tracks bytes allocated/reclaimed — fed from SizeClassAllocator stats |
| Write barriers | Records card table entries — cards move to spans but barrier logic unchanged |
| Root scanner | Feeds GcErased to marker — same mechanism |
| Nursery + Old-gen | Still available for movable objects (other neovm-gc users) |
| ObjectHeader | Same structure, same mark_bits field |
| Trace impls | Same — edge tracing is per-object, independent of allocation strategy |

### Memory Overhead

Per-object overhead comparison:

| Component | Current (Box::new + ObjectStore) | SizeClassAllocator |
|-----------|----------------------------------|--------------------|
| ObjectHeader | 56B | 56B (same) |
| malloc metadata | ~32B per alloc | 0 (span-internal) |
| ObjectRecord | 24B | 0 (no ObjectStore) |
| Mark bitmap | 0 (in header) | 0 (use header) |
| Free-list | 0 | 0 (intrusive) |
| Card table | 0 | ~0.025B per object |
| Span metadata | 0 | ~0.1B per object (amortized) |
| **Total per object** | **~112B** | **~56B** |

For 780K objects:
- Current: 780K × 112B = 87MB (but actual RSS is 1.3GB due to malloc fragmentation)
- SizeClassAllocator: 780K × 56B = 44MB + span overhead = ~50MB
- Plus payload data: ~60MB
- **Expected RSS: ~110-120MB** (matching our arena prototype)

### Size Class Table

Based on Go's size class design, adapted for Lisp object sizes:

```
Index  Slot Size  Objects per 32KB span  Waste (max per object)
  0       64          512                    7B
  1       72          455                    7B
  2       80          409                   15B
  3       96          341                   15B
  4      128          256                   31B
  5      192          170                   63B
  6      256          128                   63B
  7      384           85                  127B
  8      512           64                  127B
  9      768           42                  255B
 10     1024           32                  255B
 11     2048           16                  511B
 ---   >2048    individual alloc (Box::new)
```

The most common Lisp objects:
- Cons cells (72B) → class 1, 455 per span, 0B waste
- Floats (64B) → class 0, 512 per span, 0B waste  
- Vectors (88B) → class 3, 341 per span, 8B waste
- Short strings (80-128B) → class 2-4
- Buffers/windows/frames (72B) → class 1

~80% of objects are cons cells (class 1). Zero waste for the dominant type.

### API Surface

```rust
/// Go-style size-class allocator for non-moving pinned objects.
pub struct SizeClassAllocator {
    /// Per-size-class span pools.
    classes: [SizeClassPool; NUM_SIZE_CLASSES],
    /// Individual allocations for objects > MAX_SIZE_CLASS.
    large_objects: Vec<LargeObject>,
    /// Statistics.
    allocated_bytes: usize,
    live_bytes: usize,
}

impl SizeClassAllocator {
    /// Allocate a slot for an object of the given layout.
    /// Returns the base pointer (ObjectHeader location).
    pub fn alloc(&mut self, layout: Layout) -> NonNull<u8>;

    /// Sweep all spans after marking. Dead objects' payloads are dropped
    /// and their slots are added to the free list.
    pub fn sweep(&mut self) -> SweepStats;

    /// Clear all mark bits in preparation for a new mark cycle.
    pub fn prepare_mark(&mut self);

    /// Return completely-empty spans to the OS.
    pub fn release_empty_spans(&mut self) -> usize;

    /// Iterate over all live (allocated, not-freed) objects.
    /// Used by the concurrent marker for initial object enumeration
    /// if needed, and by statistics collection.
    pub fn for_each_object(&self, f: impl FnMut(*const ObjectHeader));

    /// Total bytes allocated across all spans.
    pub fn allocated_bytes(&self) -> usize;

    /// Total bytes retained after last sweep.
    pub fn live_bytes(&self) -> usize;
}
```

### File Structure

```
neovm-gc/src/spaces/
  pinned.rs          → SizeClassAllocator (replaces current 15-line stub)
  pinned_span.rs     → Span, SizeClassPool, free-list operations
  pinned_config.rs   → PinnedSpaceConfig (size classes, span size, thresholds)
```

### Future Evolution

1. **ObjectHeader shrink**: Once ObjectStore is not used for pinned objects,
   ObjectHeader can be shrunk to 16B (desc + mark_bits). This would make
   cons cells 32B (16B header + 16B payload) — matching GNU Emacs's layout.

2. **Concurrent sweep**: Spread sweep work across allocation events (lazy
   sweep). When allocation needs a new span for a size class, sweep one
   span from that class first.

3. **NUMA-aware spans**: Allocate spans on the NUMA node closest to the
   thread that uses them (future multi-threaded Lisp).

4. **Compaction**: Optional periodic compaction of partially-full spans.
   Copy live objects from sparse spans to dense spans, update pointer
   slots via trace_root_slots, return empty spans to OS.
