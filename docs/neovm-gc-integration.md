# Wiring neovm-gc into neovm-core

**Date**: 2026-04-16
**Status**: Design
**Branch**: wire-neovm-gc
**Reference model**: V8 Orinoco GC

## Motivation

GNU Emacs's stop-the-world mark-sweep GC is one of its most criticized aspects.
Heavy Lisp workloads (LSP, magit, completion frameworks) generate allocation
pressure that causes multi-millisecond UI freezes. Neomacs currently uses the
same design (TaggedHeap: full-heap mark-sweep). This document describes how to
wire the neovm-gc crate into neovm-core to achieve sub-millisecond GC pauses
with concurrent marking and selective compaction, following V8's Orinoco GC
design.

### Why V8 is the right reference

V8 (Chrome/Node.js) faces the same constraints as an editor:
- Interactive, latency-sensitive (UI must never freeze)
- Tagged pointer representation (Smi + HeapObject, like neomacs Value)
- Burst allocation patterns (rendering, parsing, evaluation)
- Must compact to avoid fragmentation in long-running sessions

V8 proves that **tagged pointers + compaction + concurrent marking** all work
together at massive scale. No load barriers (unlike ZGC). No pinning. Objects
move freely; all pointer slots are recorded and fixed up after compaction.

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
3. **Selective compaction** -- defragment without load barriers
4. **Fast allocation** -- bump pointer for all types (especially cons cells)
5. **Keep tagged pointer performance** -- `cons_car` stays a single pointer deref
6. **Idle-time GC** -- do GC work while waiting for keystrokes

## Migration Strategy: Big-Bang Refactor

No adapter layer, no dual-heap period, no gradual migration. Replace TaggedHeap
with neovm-gc directly in one coordinated change. Rationale:

- **Simpler**: no shim layer to maintain, no two-GC state to reason about
- **Cleaner**: the allocation API surface (alloc_cons, alloc_string, etc.) is
  small (~17 functions) and well-defined
- **Testable**: the full Emacs bootstrap (byte-compile bytecomp.el, build pdump)
  is the integration test — it either works or it doesn't
- **The 7,000+ Value constructor call sites don't change** — they call
  Value::cons(), Value::string() etc. which internally call alloc_cons(),
  alloc_string(). Only the alloc_* implementations change.

## Key Design Decision: V8-Style Move + Fixup (Not Pin, Not Load Barrier)

### Three approaches compared

| Approach | Read cost | Compaction | Pause model |
|----------|-----------|------------|-------------|
| Pin only | 0 (raw deref) | None | Short pauses but memory fragments |
| ZGC load barrier | ~1-2ns per read | Concurrent | Ultra-low pause but constant read overhead |
| **V8 slot recording** | **0 (raw deref)** | **Selective, STW fixup** | **Zero read overhead + defragmentation** |

**Decision: V8-style.** Objects can move. During marking, record all pointer
slot locations (where pointers live, not just what they point to). After
compaction, walk recorded slots and fix them up. No read barriers. No
indirection tables. Tagged pointer reads remain a single deref.

### How it works

```
Normal operation (99.9% of time):
  cons_car = unsafe { (*ptr).car }     // raw deref, zero overhead
  Value::cons(car, cdr)                // bump allocate, zero overhead

Minor GC (sub-millisecond):
  1. Scan nursery for live objects
  2. Promote survivors to old-gen (copy or re-categorize)
  3. Update remembered set pointers
  4. Reclaim nursery

Major GC (concurrent):
  1. [concurrent] Background thread marks reachable objects
  2. [concurrent] Record all pointer SLOT LOCATIONS during mark
  3. [STW brief]  Final remark (handle mutations since concurrent mark)
  4. [STW brief]  Selective compaction: move objects in sparse blocks
  5. [STW brief]  Fixup pass: walk recorded slots, update moved pointers
  6. [concurrent] Lazy sweep: reclaim dead objects on demand
```

### Pointer slot recording

During the mark phase, instead of just visiting values, we visit the memory
locations that hold those values:

```rust
// Current trace_roots (read-only values):
trace_roots(&mut |root: Value| { heap.seed_root(root); });

// New trace_root_slots (mutable pointer locations):
trace_root_slots(&mut |slot: &mut Value| { heap.record_slot(slot); });
```

After compaction moves objects and installs forwarding pointers:

```rust
// Fixup pass: update all recorded pointer slots
for slot in recorded_slots {
    if slot.points_to_moved_object() {
        *slot = forwarding_table.resolve(*slot);
    }
}
```

### Where pointer slots live

All already enumerated by trace_roots():

| Location | Slot type | Count (typical) |
|----------|-----------|-----------------|
| `bc_buf: Vec<Value>` | `&mut bc_buf[i]` | ~1K per frame |
| cons cell car/cdr | `&mut (*cons).car`, `&mut (*cons).cdr` | millions |
| vector elements | `&mut items[i]` | varies |
| hash table values | `&mut data[key]` | varies |
| specpdl entries | `&mut old_value`, `&mut lexenv` | ~100-1K |
| obarray values | `&mut symbol.value`, `&mut symbol.function` | ~10K |
| condition stack | `&mut tag`, `&mut handler` | ~10-100 |
| closure env | `&mut slots[i]` | varies |

Total slots to record and fixup scales with **live pointer count**, not heap
size. For a 100MB heap with 5M pointers, the fixup pass is O(5M) pointer
updates -- a few milliseconds at most, and only during compaction.

## Architecture

```
Value (tagged u64, unchanged)
  |
  | raw pointer (points past ObjectHeader to payload)
  v
[ObjectHeader][ConsCell{car,cdr}]     <-- may move during compaction
[ObjectHeader][LispString{...}]       <-- may move during compaction
[ObjectHeader][LispVector{...}]       <-- may move during compaction
  |
  | managed by
  v
neovm-gc Heap
  +-- Nursery: bump-pointer TLAB (young objects, collected frequently)
  +-- Old-gen: Immix-style blocks (promoted objects, concurrent mark)
  +-- Concurrent marker thread (background marking)
  +-- Adaptive pacer (EWMA-driven collection scheduling)
  +-- Write barriers (generational + SATB for concurrent mark)
  +-- Selective compactor (evacuate sparse blocks, fixup slots)
  +-- Idle-time scheduler (GC during editor idle)
```

### Tagged pointer encoding (unchanged)

```
Tag (3 low bits)  Payload                         Fast check
000               Symbol (SymId << 3)              (v & 7) == 0
xx1               Fixnum (integer << 2)            (v & 3) == 1
                  Uses tags 001 and 101 for 62-bit signed range
010               Cons (pointer | 2)               (v & 7) == 2
011               Vectorlike (pointer | 3)          (v & 7) == 3
100               String (pointer | 4)              (v & 7) == 4
110               Float (pointer | 6)               (v & 7) == 6
111               Immediate (Qunbound sentinel)     (v & 7) == 7
```

The pointer in a tagged Value points to the **payload**, not the ObjectHeader.
To recover the ObjectHeader from a payload pointer:
`header = (payload_ptr as *const u8).sub(HEADER_SIZE) as *const ObjectHeader`

### Value access (unchanged performance, zero read barriers)

```rust
// cons_car -- one pointer deref, ~1ns, no barrier
pub fn cons_car(self) -> Self {
    unsafe { (*self.xcons_ptr()).car }
}

// No load barrier. No indirection. No forwarding check.
// Pointers are ALWAYS valid because fixup happens before
// the mutator resumes.
```

### Compaction correctness guarantee

Objects only move during STW compaction. The fixup pass runs before the mutator
resumes. Therefore, from the mutator's perspective, tagged pointers are always
valid. No stale pointers ever escape to user code.

```
Timeline:
  [mutator runs]  →  [STW: compact + fixup]  →  [mutator resumes]
                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^
                      All pointer slots updated here.
                      Mutator never sees stale pointers.
```

## Changes Required

### Part A: neovm-gc API additions (PROPOSED — these APIs do not exist yet)

#### 1.1 External root slot scanner

neovm-gc currently discovers roots only through RootStack. Add support for
external root scanning with mutable slot access:

```rust
impl Heap {
    /// Register a callback that provides mutable access to external root slots.
    /// Called during collection to discover roots AND record pointer locations
    /// for compaction fixup.
    pub fn set_root_slot_scanner<F>(&mut self, scanner: F)
    where F: FnMut(&mut dyn FnMut(&mut GcSlot)) + 'static;
}

/// A mutable reference to a location holding a GC-managed pointer.
/// Used for both root discovery and compaction fixup.
pub struct GcSlot {
    location: *mut TaggedValue,
}
```

During collection + compaction:
```rust
// 1. Root scanning: find live objects
root_slot_scanner(|slot| {
    mark(slot.value());
    recorded_slots.push(slot.location);
});

// 2. Object tracing: find live objects reachable from roots
// (trace_tagged_value also records interior pointer slots)
mark_all();

// 3. Compaction: move sparse objects, install forwarding
compact_sparse_blocks();

// 4. Fixup: update ALL recorded slots
for slot_ptr in &recorded_slots {
    let val = unsafe { **slot_ptr };
    if let Some(new_addr) = forwarding_table.get(val) {
        unsafe { **slot_ptr = new_addr; }
    }
}
```

#### 1.2 Mutable object tracing

Current Trace trait provides read-only edge access. For compaction fixup,
interior pointer slots must also be updatable:

```rust
pub unsafe trait Trace {
    fn trace(&self, tracer: &mut dyn Tracer);
    fn relocate(&self, relocator: &mut dyn Relocator);

    /// NEW: provide mutable access to interior pointer slots.
    /// Called during fixup pass to update pointers to moved objects.
    fn trace_slots(&mut self, visitor: &mut dyn FnMut(&mut TaggedValue)) {
        // Default: no interior pointer slots (leaf objects like Float)
    }
}

// Cons cell implementation:
unsafe impl Trace for GcConsCell {
    fn trace(&self, tracer: &mut dyn Tracer) {
        trace_tagged_value(tracer, self.car);
        trace_tagged_value(tracer, self.cdr);
    }
    fn relocate(&self, relocator: &mut dyn Relocator) {
        // V8-style: relocate is now handled by trace_slots + fixup pass
    }
    fn trace_slots(&mut self, visitor: &mut dyn FnMut(&mut TaggedValue)) {
        visitor(&mut self.car);
        visitor(&mut self.cdr);
    }
}
```

#### 1.3 Tagged pointer helpers

```rust
impl<T> Gc<T> {
    /// Get raw pointer to payload (for encoding into tagged Value).
    pub fn payload_ptr(&self) -> *const T;

    /// Recover Gc handle from a raw payload pointer.
    pub unsafe fn from_payload_ptr(ptr: *const T) -> Gc<T>;
}

impl ObjectHeader {
    /// Constant header size for pointer arithmetic.
    pub const FIXED_SIZE: usize = std::mem::size_of::<ObjectHeader>();

    /// Recover header from a payload pointer.
    pub unsafe fn from_payload_ptr<T>(ptr: *const T) -> &ObjectHeader {
        &*((ptr as *const u8).sub(Self::FIXED_SIZE) as *const ObjectHeader)
    }
}
```

#### 1.4 Idle-time collection API

V8's idle-time GC is critical for editors:

```rust
impl Heap {
    /// Perform GC work that fits within the given idle budget.
    /// Called from the editor's event loop when waiting for input.
    ///
    /// Returns how much of the budget was used.
    pub fn idle_notification(&mut self, idle_budget: Duration) -> Duration;
}
```

The implementation dispatches to incremental marking, lazy sweeping, or
selective compaction depending on what work is pending and how much budget
is available:

```rust
fn idle_notification(&mut self, budget: Duration) -> Duration {
    let start = Instant::now();
    if self.incremental_marking_in_progress() {
        self.advance_incremental_mark(budget);
    } else if self.sweeping_pending() {
        self.advance_lazy_sweep(budget);
    } else if self.fragmentation_ratio() > COMPACT_THRESHOLD {
        self.compact_one_block(budget);
    }
    start.elapsed()
}
```

### Part B: Trace impls for Lisp types (PROPOSED — pseudocode, trace_slots does not exist yet)

```rust
struct GcConsCell {
    car: TaggedValue,
    cdr: TaggedValue,
}

unsafe impl Trace for GcConsCell {
    fn trace(&self, tracer: &mut dyn Tracer) {
        trace_tagged_value(tracer, self.car);
        trace_tagged_value(tracer, self.cdr);
    }
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn trace_slots(&mut self, visitor: &mut dyn FnMut(&mut TaggedValue)) {
        visitor(&mut self.car);
        visitor(&mut self.cdr);
    }
}

struct GcLispString {
    inner: LispString,
    // text properties contain TaggedValues
    text_props: Vec<(usize, usize, Vec<(TaggedValue, TaggedValue)>)>,
}

unsafe impl Trace for GcLispString {
    fn trace(&self, tracer: &mut dyn Tracer) {
        for (_, _, props) in &self.text_props {
            for (key, val) in props {
                trace_tagged_value(tracer, *key);
                trace_tagged_value(tracer, *val);
            }
        }
    }
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn trace_slots(&mut self, visitor: &mut dyn FnMut(&mut TaggedValue)) {
        for (_, _, props) in &mut self.text_props {
            for (key, val) in props {
                visitor(key);
                visitor(val);
            }
        }
    }
}

struct GcVector {
    items: Vec<TaggedValue>,
}

unsafe impl Trace for GcVector {
    fn trace(&self, tracer: &mut dyn Tracer) {
        for item in &self.items {
            trace_tagged_value(tracer, *item);
        }
    }
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn trace_slots(&mut self, visitor: &mut dyn FnMut(&mut TaggedValue)) {
        for item in &mut self.items {
            visitor(item);
        }
    }
}

// Helper
fn trace_tagged_value(tracer: &mut dyn Tracer, val: TaggedValue) {
    if val.is_heap_pointer() {
        let header = unsafe { ObjectHeader::from_payload_ptr(val.heap_ptr()) };
        tracer.mark_erased(header.erased());
    }
}
```

Similar impls for: LispHashTable, LambdaData, ByteCodeFunction, OverlayData,
MarkerData, FloatObj, BignumObj, SubrObj, RecordObj.

### Part C: Replace TaggedHeap (PROPOSED — pseudocode showing target API shape)

Rewrite `tagged/gc.rs` to use neovm-gc directly. The struct keeps the same
public API (alloc_cons, alloc_string, etc.) but the implementation changes
from block allocator + linked list to neovm-gc mutator calls:

```rust
// tagged/gc.rs — rewritten internals, same public API
//
// NOTE: Mutator<'heap> borrows &Heap, so TaggedHeap must own the Heap
// and produce a Mutator on demand (or use an unsafe self-referential
// pattern). Two viable approaches:
//
//   (a) Own Heap in a Box/Pin, produce Mutator for each allocation
//       batch via an unsafe lifetime extension (the Heap outlives
//       every Mutator because TaggedHeap owns it).
//
//   (b) Store Heap in a separate long-lived allocation (Arc or leaked
//       Box), and keep a Mutator<'heap> that borrows it.
//
// Approach (b) is shown below (pseudocode — exact lifetime wiring
// depends on neovm-gc's API surface for long-lived mutators):

pub struct TaggedHeap {
    heap: Pin<Box<neovm_gc::Heap>>,
    // Mutator borrows heap; safe because heap is pinned and owned.
    // In practice this may require unsafe or a self-referential crate.
    mutator: neovm_gc::Mutator<'???>,  // lifetime TBD during implementation
    // Registries unchanged
    subr_registry: Vec<Option<TaggedValue>>,
    buffer_registry: FxHashMap<BufferId, TaggedValue>,
    window_registry: FxHashMap<u64, TaggedValue>,
    frame_registry: FxHashMap<u64, TaggedValue>,
    timer_registry: FxHashMap<u64, TaggedValue>,
    marker_ptrs: Vec<*mut MarkerObj>,
}

impl TaggedHeap {
    pub fn alloc_cons(&mut self, car: TaggedValue, cdr: TaggedValue) -> TaggedValue {
        let gc = self.mutator.alloc(GcConsCell { car, cdr });
        TaggedValue::from_cons_ptr(gc.payload_ptr())
    }

    pub fn alloc_string(&mut self, s: LispString) -> TaggedValue {
        let gc = self.mutator.alloc(GcLispString::from(s));
        TaggedValue::from_string_ptr(gc.payload_ptr())
    }

    pub fn alloc_float(&mut self, value: f64) -> TaggedValue {
        let gc = self.mutator.alloc(GcFloat { value });
        TaggedValue::from_float_ptr(gc.payload_ptr())
    }

    pub fn alloc_vector(&mut self, items: Vec<TaggedValue>) -> TaggedValue {
        let gc = self.mutator.alloc(GcVector { items });
        TaggedValue::from_veclike_ptr(gc.payload_ptr())
    }

    // ... same pattern for all 17 allocation functions
    // Public API unchanged. with_tagged_heap() unchanged.
    // ConsBlock, GcHeader, all_objects linked list — all removed.
}
```

Remove: ConsBlock, cons bump allocator, GcHeader, all_objects linked list,
mark bitmap, mark_all(), sweep_cons(), sweep_objects(), free_gc_object().

Keep: with_tagged_heap() accessor, registries, marker_ptrs, write tracking hooks.

### Part D: Wire write barriers

Two barriers, matching V8:

```rust
// 1. Generational barrier: old object stores pointer to young object
// 2. SATB marking barrier: object modified during concurrent mark
//
// neovm-gc API (mutator.rs:955-966):
//   pub fn post_write_barrier<Owner, Value>(
//       &mut self,
//       owner: Gc<Owner>,
//       slot: Option<usize>,
//       old_value: Option<Gc<Value>>,   // ← needed for SATB
//       new_value: Option<Gc<Value>>,
//   )
//
// INTEGRATION GAP: current note_heap_slot_write() in tagged/mutate.rs
// only records the new value, not the old value. Must be extended to
// capture the old value BEFORE the store for SATB correctness:

pub fn set_cons_car(cell: TaggedValue, value: TaggedValue) -> bool {
    if !cell.is_cons() { return false; }
    // Read old value BEFORE the store (SATB needs it)
    let old_value = unsafe { (*cell.xcons_ptr()).car };
    // Perform the store
    unsafe { (*(cell.xcons_ptr() as *mut ConsCell)).set_car(value); }
    // Post-write barrier with both old and new
    with_gc_heap(|h| {
        let owner_gc = tagged_to_gc(cell);
        let old_gc = tagged_to_gc_option(old_value);
        let new_gc = tagged_to_gc_option(value);
        h.mutator.post_write_barrier(owner_gc, Some(0), old_gc, new_gc);
    });
    true
}
```

This is the main integration gap: all ~15 mutation functions in tagged/mutate.rs
(set_cons_car, set_cons_cdr, set_vector_slot, etc.) must be updated to capture
the old value before the store and pass both old+new to the barrier.

### Part E: Wire root slot discovery

Connect trace_roots() to neovm-gc with mutable slot access for compaction:

```rust
// In Context:
fn trace_root_slots(&mut self, visitor: &mut dyn FnMut(&mut Value)) {
    // VM root frames
    for frame in &mut self.vm_root_frames {
        for root in frame.roots.iter_mut() {
            visitor(root);
        }
    }
    // Bytecode buffer
    for val in self.bc_buf.iter_mut() {
        visitor(val);
    }
    // Specpdl entries
    for entry in self.specpdl.iter_mut() {
        match entry {
            SpecBinding::Let { old_value, .. } => visitor(old_value),
            SpecBinding::LexicalEnv { old_lexenv } => visitor(old_lexenv),
            SpecBinding::GcRoot { value } => visitor(value),
            SpecBinding::Backtrace { function, args, .. } => {
                visitor(function);
                for arg in args.iter_mut() { visitor(arg); }
            }
            SpecBinding::UnwindProtect { forms, lexenv, .. } => {
                visitor(forms);
                visitor(lexenv);
            }
            _ => {}
        }
    }
    // Lexenv
    visitor(&mut self.lexenv);
    // Obarray (all symbol values, functions, plists)
    self.obarray.trace_slots(visitor);
    // Condition stack
    for frame in self.condition_stack.iter_mut() {
        frame.trace_slots(visitor);
    }
    // Subsystem managers
    self.buffers.trace_slots(visitor);
    self.processes.trace_slots(visitor);
    self.timers.trace_slots(visitor);
    self.frames.trace_slots(visitor);
    // ... all 15+ subsystems
}
```

After compaction, all pointer slots have been updated in-place. The mutator
resumes with all tagged pointers pointing to new locations.

### Part F: Wire idle-time GC

In the editor's event loop (neomacs-bin), call idle_notification when waiting
for input:

```rust
// In the winit event loop, during AboutToWait:
fn about_to_wait(&mut self) {
    // ... existing animation/cursor tick ...

    // GC idle work: use remaining idle budget for GC
    let idle_budget = Duration::from_millis(4); // 4ms budget, stay under 16ms frame
    with_gc_heap(|h| {
        h.heap.idle_notification(idle_budget);
    });
}
```

V8's Chrome integration does the same: when the renderer reports idle time,
V8 performs incremental marking, lazy sweeping, or compaction.

For neomacs, idle time is abundant -- the editor spends most time waiting for
keystrokes. GC work during idle is essentially free.

### Part G: Wire concurrent marker + adaptive pacer

```rust
// During Context initialization:
let shared_heap = gc_heap.heap.into_shared();
let marker = ConcurrentMarker::start(
    shared_heap.clone(),
    ConcurrentMarkerConfig {
        mark_slice_budget: 64,
        busy_sleep: Duration::from_micros(100),
        idle_sleep: Duration::from_millis(1),
    },
);

// GC safe point uses pacer recommendation:
fn gc_safe_point_exact(&mut self) {
    if self.gc_inhibit_depth > 0 { return; }
    with_gc_heap(|h| {
        if let Some(plan) = h.mutator.recommended_plan() {
            h.mutator.execute_plan(plan);
        }
    });
}
```

### Part H: Incremental marking (V8 innovation)

V8 doesn't just mark concurrently on a background thread -- it also marks
incrementally on the main thread, spread across allocation events:

```rust
fn gc_safe_point_exact(&mut self) {
    if self.gc_inhibit_depth > 0 { return; }
    with_gc_heap(|h| {
        // Incremental marking: do a small slice of mark work
        // each time we hit a safe point (every ~100 allocations)
        if h.incremental_marking_in_progress() {
            h.advance_incremental_mark_step(MARK_STEP_BUDGET);
        }
        // Pacer may also trigger full collection
        if let Some(plan) = h.mutator.recommended_plan() {
            h.mutator.execute_plan(plan);
        }
    });
}
```

This ensures marking completes quickly even if the background thread is slow,
because the main thread contributes marking work during normal execution.

### Part I: Lazy sweeping (V8 innovation)

Instead of sweeping all dead objects in one STW pause, sweep on demand:

```rust
// During allocation, if no free space in current block:
fn alloc_from_old_gen(&mut self, size: usize) -> *mut u8 {
    // Try current block
    if let Some(ptr) = self.current_block.try_alloc(size) {
        return ptr;
    }
    // Lazy sweep: sweep one dead block to get space
    if let Some(block) = self.sweep_next_dead_block() {
        self.current_block = block;
        return self.current_block.try_alloc(size).unwrap();
    }
    // No dead blocks: trigger collection
    self.collect();
    self.alloc_from_old_gen(size)
}
```

This spreads sweep cost across allocation events. V8 also sweeps concurrently
on a background thread.

## GC Cycle Breakdown

### Minor GC (nursery collection)

```
[STW ~100-300us]
1. Scan root slots for nursery pointers
2. Copy live nursery objects to old-gen
3. Update root slots to point to new old-gen locations
4. Update remembered set entries (old pointing to nursery)
5. Reset nursery bump pointer
```

### Major GC (concurrent mark + selective compact)

```
[Concurrent]      Incremental + background marking (SATB barriers active)
[STW ~200-500us]  Final remark: drain SATB buffer, handle mutations
[STW ~100-500us]  Select sparse blocks, evacuate objects, fixup pointer slots
[Concurrent]      Lazy sweep: reclaim dead blocks on demand
```

### Idle GC

```
[Idle budget]     Advance incremental mark / lazy sweep / compact one block
                  Uses editor idle time (waiting for keystrokes)
                  Budget: ~4ms per idle tick, fits under 16ms frame budget
```

## Performance Expectations

| Metric | Current (TaggedHeap) | After (V8-style neovm-gc) |
|--------|---------------------|---------------------------|
| Minor GC pause | N/A (full sweep) | 100-300 us |
| Major GC pause | 5-50ms (proportional to heap) | <1ms STW (remark + fixup) |
| Read (cons_car) | ~1ns (raw deref) | ~1ns (raw deref, no barrier) |
| Write (setcar) | ~1ns (no barrier) | ~40ns (write barrier) |
| Cons allocation | bump pointer ~10ns | TLAB bump pointer ~10ns |
| Background work | none | concurrent mark + lazy sweep |
| Idle utilization | none | incremental mark/sweep/compact |
| Compaction | none | selective (sparse blocks) |
| Fragmentation | grows over session | controlled |

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| ObjectHeader overhead (48B per cons) | Cons cells grow from 16B to 64B. Optimize header to 16B later (mark bit + type tag + forwarding ptr). |
| Compaction fixup correctness | V8 has shipped this at billion-user scale. neovm-gc already has forwarding + relocation tests. trace_root_slots is mechanical extension of trace_roots. |
| Write barrier overhead (~40ns) | Only on pointer stores, not reads. Reads outnumber writes ~10:1 in typical Lisp. Net positive from shorter pauses. |
| Concurrent mark SATB correctness | neovm-gc has concurrent stress tests + SATB barrier tests. Battle-tested. |
| Idle-time scheduling | Conservative budget (4ms). If idle notification takes too long, skip next cycle. Editor responsiveness is the priority. |
| Nursery promotion with tagged pointers | During minor GC, nursery objects move to old-gen. All slots pointing to nursery must be updated. Remembered set + root slot scan covers this. Same as V8 scavenger. |

## Open Questions

1. **ObjectHeader size**: 48 bytes per cons cell is 3x overhead. V8 uses 8-16
   byte headers. Can we shrink ObjectHeader for small fixed-size types (cons,
   float) while keeping the full header for variable-size types (string, vector)?

2. **Nursery size tuning**: How large should the nursery be? V8 uses 1-16MB
   depending on memory pressure. For an editor, 1-4MB nursery seems right.

3. **Compaction trigger**: When to compact? V8 uses page occupancy < 50%.
   neovm-gc has configurable density threshold. Need to tune for Lisp allocation
   patterns.

4. **Concurrent sweep thread**: neovm-gc's sweep is currently STW. Adding a
   concurrent sweep thread (like V8) would further reduce pauses.

5. **Integration testing**: Need a test harness that runs the full Emacs
   bootstrap (byte-compilation of bytecomp.el etc.) under the new GC to
   validate correctness before removing TaggedHeap.

## Implementation Order (Big-Bang)

All changes land together on the wire-neovm-gc branch. No intermediate
dual-heap state. The branch is validated by running the full Emacs bootstrap
before merging to main.

| Step | Scope | Description |
|------|-------|-------------|
| 1 | neovm-gc | External root slot scanner (mutable slot callback) |
| 2 | neovm-gc | Mutable object tracing (trace_slots for fixup) |
| 3 | neovm-gc | Tagged pointer helpers (payload_ptr, from_payload_ptr) |
| 4 | neovm-gc | Idle-time collection API (idle_notification) |
| 5 | neovm-core | Trace + trace_slots impls for all Lisp heap types |
| 6 | neovm-core | Rewrite tagged/gc.rs internals to use neovm-gc (same public API) |
| 7 | neovm-core | Wire write barriers (generational + SATB) |
| 8 | neovm-core | Add trace_root_slots (mutable slot visitor) to Context |
| 9 | neovm-core | Wire concurrent marker + adaptive pacer |
| 10 | neovm-core | Wire idle-time GC into editor event loop |
| 11 | neovm-core | Add incremental marking at safe points |
| 12 | neovm-core | Add lazy sweeping in allocation path |
| 13 | validate | Full Emacs bootstrap (byte-compile, pdump) under new GC |

Steps 1-4: neovm-gc API additions (can be developed independently).
Steps 5-8: big-bang replacement of TaggedHeap internals.
Steps 9-12: enable V8-style concurrent/incremental/idle GC.
Step 13: validation gate before merging to main.
