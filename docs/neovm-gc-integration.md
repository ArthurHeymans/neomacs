# Wiring neovm-gc into neovm-core

**Date**: 2026-04-16
**Status**: Design
**Branch**: wire-neovm-gc

## Motivation

GNU Emacs's stop-the-world mark-sweep GC is one of its most criticized aspects.
Heavy Lisp workloads (LSP, magit, completion frameworks) generate allocation
pressure that causes multi-millisecond UI freezes. Neomacs currently uses the
same design (TaggedHeap: full-heap mark-sweep). This document describes how to
wire the neovm-gc crate into neovm-core to achieve sub-millisecond GC pauses
with concurrent marking.

### Current state

**TaggedHeap** (neovm-core/src/tagged/gc.rs):
- Stop-the-world mark-sweep
- Cons cells: 64KB bump-allocated blocks, packed mark bitmap, intrusive free list
- Non-cons: system allocator (Box), intrusive GcHeader linked list
- Root discovery: `trace_roots()` callback walks 15+ subsystems
- Write barrier infrastructure exists but is unused
- GC trigger: threshold-based at safe points (~800KB default)

**neovm-gc** (neovm-gc/ crate):
- Generational: nursery (bump-pointer TLAB) + old-gen (Immix-style blocks)
- Concurrent marking: background mark thread, lock-alternating
- Physical compaction: selective block evacuation
- Adaptive pacer: Go-style EWMA, three stacked constraints
- Write barriers: post-write (old-to-young) + SATB (concurrent mark)
- Measured: minor GC 39-340us, major GC ~94us, allocation 6-7M elem/sec
- 25K+ lines of tests, production-quality

**Problem**: neovm-gc is a standalone crate not connected to the VM.

## Design Goals

1. **P99 GC pause < 1ms** -- never noticeable to the user
2. **Concurrent major marking** -- background thread, not stop-the-world
3. **Fast allocation** -- bump pointer for all types (especially cons cells)
4. **Keep tagged pointer performance** -- `cons_car` stays a single pointer deref
5. **Minimal VM API change** -- the 7,000+ `Value::cons()` etc. call sites don't change

## Key Design Decision: Pin, Don't Move

neovm-gc supports object compaction (moving objects and updating references).
Moving objects requires updating EVERY tagged pointer that references the moved
object -- a full-heap scan that defeats generational collection's purpose.

**Decision: pin all objects.** Tagged pointers embed raw heap addresses. Pinning
keeps them valid forever. We lose compaction but keep:
- Generational collection (nursery to old promotion by re-categorization)
- Concurrent marking (background thread marks in place)
- Adaptive pacer
- Sub-millisecond pauses

Compaction matters for long-running servers with heap fragmentation. Editors
restart regularly. The fragmentation cost is negligible.

## Architecture

```
Value (tagged u64, unchanged)
  |
  | raw pointer (points past ObjectHeader to payload)
  v
[ObjectHeader 48B][ConsCell 16B]    <-- pinned, never moves
[ObjectHeader 48B][LispString ...]  <-- pinned, never moves
[ObjectHeader 48B][LispVector ...]  <-- pinned, never moves
  |
  | managed by
  v
neovm-gc Heap
  - PinnedBumpSpace: TLAB bump allocation for cons/float/small objects
  - PinnedSpace: system allocator for strings/vectors/hash tables
  - Concurrent marker thread
  - Adaptive pacer
  - Write barrier tracking (card tables for old-to-young)
```

### Tagged pointer encoding (unchanged)

```
Tag (3 low bits)  Payload
000               Symbol (SymId index)
01x               Fixnum (62-bit signed)
010               Cons (pointer to ConsCell payload, past ObjectHeader)
011               Vectorlike (pointer to payload, past ObjectHeader)
100               String (pointer to StringObj payload, past ObjectHeader)
110               Float (pointer to FloatObj payload, past ObjectHeader)
```

The pointer in a tagged Value points to the **payload**, not the ObjectHeader.
To recover the ObjectHeader from a payload pointer:
`header = (payload_ptr as *const u8).sub(HEADER_SIZE) as *const ObjectHeader`

This works because ObjectHeader has a fixed size known at compile time.

### Value access (unchanged performance)

```rust
// cons_car -- still one pointer deref, ~1ns
pub fn cons_car(self) -> Self {
    unsafe { (*self.xcons_ptr()).car }
}

// No indirection table, no handle resolution, no overhead.
```

## Changes Required

### Phase 1: neovm-gc API additions

Three additions to the neovm-gc crate:

#### 1.1 External root callback

Current: neovm-gc discovers roots only through RootStack (HandleScope/Root).
Needed: neovm-gc calls a user-provided callback during collection to discover
external roots.

```rust
// New API in Heap
impl Heap {
    pub fn set_root_scanner<F>(&mut self, scanner: F)
    where F: FnMut(&mut dyn FnMut(GcErased)) + 'static;
}
```

During collection, the collector calls:
```rust
// In collector_exec.rs, collect_global_sources():
let mut external_roots = Vec::new();
if let Some(scanner) = &mut self.root_scanner {
    scanner(&mut |erased| external_roots.push(erased));
}
// Merge with RootStack roots and immortal objects
```

This lets neovm-core's `trace_roots()` feed roots into neovm-gc without
requiring HandleScope management for every bytecode operation.

#### 1.2 Pinned bump allocator

Current: PinnedSpace uses system allocator (Box::new). Slow for hot paths.
Needed: TLAB-style bump allocation for pinned objects (cons cells especially).

```rust
// New space type or PinnedSpace extension
impl PinnedBumpSpace {
    fn alloc_bump<T: Trace>(&mut self, value: T) -> *mut ObjectRecord;
}
```

Design: 64KB aligned blocks (like current TaggedHeap cons blocks). Each block
has ObjectHeader + payload pairs laid out contiguously. Bump pointer increments
by header_size + payload_size per allocation. Free list threading through dead
objects during sweep.

This matches neovm-gc's nursery TLAB design but for pinned (non-moving) space.

#### 1.3 Tagged pointer helpers

```rust
impl<T> Gc<T> {
    /// Get raw pointer to payload (for encoding into tagged Value).
    pub fn payload_ptr(&self) -> *const T;

    /// Recover Gc handle from a raw payload pointer.
    /// Safety: ptr must point to a live payload managed by this heap.
    pub unsafe fn from_payload_ptr(ptr: *const T) -> Gc<T>;
}

impl ObjectHeader {
    /// Recover header from a payload pointer.
    /// Safety: ptr must be a payload pointer from an neovm-gc allocation.
    pub unsafe fn from_payload_ptr<T>(ptr: *const T) -> &ObjectHeader {
        &*((ptr as *const u8).sub(Self::FIXED_SIZE) as *const ObjectHeader)
    }
}
```

### Phase 2: Trace impls for Lisp types

Implement neovm-gc's `Trace` trait for each Lisp heap type. These describe
GC edges so the marker can trace the object graph.

```rust
// Cons cell
struct GcConsCell {
    car: TaggedValue,  // may point to other heap objects
    cdr: TaggedValue,
}

unsafe impl Trace for GcConsCell {
    fn trace(&self, tracer: &mut dyn Tracer) {
        trace_tagged_value(tracer, self.car);
        trace_tagged_value(tracer, self.cdr);
    }
    fn relocate(&self, _relocator: &mut dyn Relocator) {
        // Pinned: no relocation needed
    }
    fn move_policy() -> MovePolicy { MovePolicy::Pinned }
}

// Helper: trace a TaggedValue if it points to a heap object
fn trace_tagged_value(tracer: &mut dyn Tracer, val: TaggedValue) {
    if val.is_heap_pointer() {
        let header = unsafe { ObjectHeader::from_payload_ptr(val.heap_ptr()) };
        tracer.mark_erased(header.erased());
    }
}
```

Similar impls for: LispString, LispVector, LispHashTable, LambdaData,
ByteCodeFunction, OverlayData, MarkerData, FloatObj, BignumObj, SubrObj,
RecordObj.

### Phase 3: GcHeap adapter

A new `GcHeap` struct that wraps neovm-gc's `Heap` and provides the same API
surface as TaggedHeap:

```rust
pub struct GcHeap {
    heap: neovm_gc::Heap,
    mutator: neovm_gc::Mutator<'static>,
    // Registries (same as TaggedHeap)
    subr_registry: Vec<Option<TaggedValue>>,
    buffer_registry: FxHashMap<BufferId, TaggedValue>,
    // ... etc
}

impl GcHeap {
    pub fn alloc_cons(&mut self, car: TaggedValue, cdr: TaggedValue) -> TaggedValue {
        let gc = self.mutator.alloc_pinned_bump(GcConsCell { car, cdr });
        TaggedValue::from_cons_ptr(gc.payload_ptr())
    }

    pub fn alloc_string(&mut self, s: LispString) -> TaggedValue {
        let gc = self.mutator.alloc_pinned(GcLispString(s));
        TaggedValue::from_string_ptr(gc.payload_ptr())
    }

    pub fn alloc_float(&mut self, value: f64) -> TaggedValue {
        let gc = self.mutator.alloc_pinned_bump(GcFloat { value });
        TaggedValue::from_float_ptr(gc.payload_ptr())
    }

    pub fn alloc_vector(&mut self, items: Vec<TaggedValue>) -> TaggedValue {
        let gc = self.mutator.alloc_pinned(GcVector { items });
        TaggedValue::from_veclike_ptr(gc.payload_ptr())
    }

    // ... same pattern for all 17 allocation functions
}
```

The `with_tagged_heap()` thread-local accessor switches to `with_gc_heap()`.
Since the API surface is the same, the 7,000+ Value constructor call sites
don't change.

### Phase 4: Wire write barriers

The existing `note_heap_write()` / `note_heap_slot_write()` hooks in
tagged/mutate.rs already intercept every setcar/setcdr/aset/puthash. Wire
these into neovm-gc's barrier system:

```rust
pub fn note_heap_slot_write(owner: TaggedValue, kind: HeapWriteKind,
                            _slot: usize, value: TaggedValue) {
    with_gc_heap(|h| {
        if owner.is_heap_pointer() && value.is_heap_pointer() {
            let owner_header = unsafe {
                ObjectHeader::from_payload_ptr(owner.heap_ptr())
            };
            let value_header = unsafe {
                ObjectHeader::from_payload_ptr(value.heap_ptr())
            };
            h.mutator.record_post_write(owner_header, value_header);
        }
    });
}
```

Cost: ~40-47ns per mutation (measured in neovm-gc benchmarks). This is the
price of generational collection.

### Phase 5: Wire root discovery

Connect neovm-core's trace_roots() to neovm-gc's external root callback:

```rust
// In Context initialization:
gc_heap.heap.set_root_scanner(|visitor| {
    // This closure is called by neovm-gc during collection.
    // It walks the same 15+ subsystems as current trace_roots().
    self.trace_roots(&mut |root: Value| {
        if root.is_heap_pointer() {
            let header = unsafe {
                ObjectHeader::from_payload_ptr(root.heap_ptr())
            };
            visitor(header.erased());
        }
    });
});
```

### Phase 6: Wire GC trigger and concurrent marker

```rust
// Replace gc_safe_point_exact() to use neovm-gc's pacer:
fn gc_safe_point_exact(&mut self) {
    if self.gc_inhibit_depth > 0 { return; }
    with_gc_heap(|h| {
        h.mutator.collect_if_recommended();
    });
}

// Start concurrent marker during Context initialization:
let shared_heap = gc_heap.heap.into_shared();
let marker = ConcurrentMarker::start(
    shared_heap.clone(),
    ConcurrentMarkerConfig {
        mark_slice_budget: 64,
        busy_sleep: Duration::from_micros(100),
        idle_sleep: Duration::from_millis(1),
    },
);
```

### Phase 7: Remove TaggedHeap

Once GcHeap is validated:
- Remove tagged/gc.rs (TaggedHeap)
- Remove ConsBlock, GcHeader, all_objects linked list
- Keep tagged/value.rs, tagged/header.rs (Value representation unchanged)
- Keep tagged/mutate.rs (write barrier hooks, now wired to neovm-gc)

## Performance Expectations

| Metric | Current (TaggedHeap) | After (neovm-gc) |
|--------|---------------------|-------------------|
| Minor GC pause | N/A (full sweep) | 39-340 us |
| Major GC pause | 5-50ms (proportional to heap) | concurrent, <1ms STW remark |
| Cons allocation | bump pointer ~10ns | TLAB bump pointer ~10ns |
| String allocation | Box::new ~50ns | pinned alloc ~50ns |
| setcar/setcdr | ~1ns (no barrier) | ~40ns (write barrier) |
| GC frequency | every ~800KB allocated | adaptive (pacer-driven) |
| Background GC | none | concurrent mark thread |
| Compaction | none | none (pinned) |

### Write barrier cost analysis

The ~40ns write barrier cost applies to setcar, setcdr, aset, puthash -- 
pointer-store mutations. It does NOT apply to:
- Reading values (cons_car, cons_cdr, aref) -- unchanged
- Creating new objects (cons, make-string) -- unchanged
- Fixnum/symbol operations -- no heap pointers involved
- Stack operations in bytecode VM -- bc_buf is not a heap object

For typical Emacs workloads, pointer stores are much less frequent than reads.
The 40ns cost is amortized across many read operations that benefit from shorter
GC pauses.

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| ObjectHeader overhead (48B per cons) | Cons cells grow from 16B to 64B. Memory ~4x for cons-heavy heaps. Acceptable for sub-ms pauses. Could optimize header to 16-24B later. |
| Write barrier bugs | neovm-gc has 25K+ lines of barrier tests. Integration tests cover setcar/setcdr/aset paths. |
| Root discovery mismatch | External root callback is a simple API addition. trace_roots() is battle-tested. |
| Concurrent mark correctness | SATB barriers prevent lost updates. neovm-gc has concurrent stress tests. |
| Tagged pointer recovery | from_payload_ptr is a constant subtraction. Same technique as GNU Emacs Lisp_Object recovery from struct fields. |

## Open Questions

1. **ObjectHeader size for cons cells**: 48 bytes of header for a 16-byte cons
   cell is a 4x memory overhead. Can we use a smaller header for cons cells
   (e.g., 8 bytes: mark bit + space tag + type tag)?

2. **Nursery for pinned objects**: neovm-gc's nursery is designed for moving
   objects (semispace copy). A pinned nursery has different semantics -- objects
   stay in place, just get re-categorized as old-gen. Need to validate this
   works with neovm-gc's promotion logic.

3. **Concurrent sweep**: Currently not in neovm-gc. Sweep is stop-the-world.
   For very large heaps, sweep pause could still be significant. Incremental
   sweep (spread across allocation events) would help.

4. **Integration testing**: Need a test harness that runs the full Emacs
   bootstrap (byte-compilation of bytecomp.el etc.) under the new GC to
   validate correctness before removing TaggedHeap.

## Implementation Order

| Step | Scope | Description |
|------|-------|-------------|
| 1 | neovm-gc | External root callback API |
| 2 | neovm-gc | Pinned bump allocator (TLAB for pinned space) |
| 3 | neovm-gc | Tagged pointer helpers (payload_ptr, from_payload_ptr) |
| 4 | neovm-core | Trace impls for all Lisp heap types |
| 5 | neovm-core | GcHeap adapter (wraps neovm-gc, matches TaggedHeap API) |
| 6 | neovm-core | Swap with_tagged_heap to use GcHeap |
| 7 | neovm-core | Wire write barriers to neovm-gc |
| 8 | neovm-core | Wire root scanner to neovm-gc |
| 9 | neovm-core | Wire concurrent marker + pacer |
| 10 | neovm-core | Validation: full bootstrap under new GC |
| 11 | neovm-core | Remove TaggedHeap |

Steps 1-3 can be done independently of steps 4-9.
Steps 4-6 are the core integration.
Steps 7-9 enable generational + concurrent collection.
Step 10 is the go/no-go gate before step 11.
