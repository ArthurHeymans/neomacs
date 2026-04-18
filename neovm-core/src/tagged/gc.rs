//! Garbage collector for the tagged pointer value system.
//!
//! # Design
//!
//! All heap objects (cons cells, strings, floats, vectorlike) are allocated
//! through a `neovm_gc::Heap`. The `TaggedHeap` owns a leaked `Heap` reference
//! (giving a `'static` lifetime) and creates on-demand `Mutator` views for
//! each allocation.
//!
//! - **Allocation**: each `alloc_*` method creates the corresponding `Gc*`
//!   wrapper type (from `gc_trace_impls.rs`), allocates through
//!   `neovm_gc::Mutator`, extracts the payload pointer, and encodes it as a
//!   `TaggedValue` with the appropriate tag bits.
//!
//! - **Collection**: `collect()` / `collect_exact()` delegate to
//!   `neovm_gc::Mutator::collect()`.
//!
//! - **Registries**: subr, buffer, window, frame, timer registries are
//!   unchanged — they are TaggedHeap-owned lookup tables.
//!
//! - **Write tracking**: dirty_owners / dirty_writes are unchanged.

use super::gc_trace_impls::*;
use super::header::*;
use super::value::TaggedValue;
use crate::buffer::text_props::TextPropertyTable;
use crate::emacs_core::intern::SymId;
use neovm_gc::descriptor::GcErased;
use neovm_gc::plan::CollectionKind;
use neovm_gc::root::Gc;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::Cell;
use std::cell::UnsafeCell;
use std::mem::size_of;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteTrackingMode {
    Disabled,
    OwnersOnly,
    OwnersAndRecords,
}

/// Classifies the kind of heap mutation that occurred.
///
/// GNU Emacs performs direct object/cell writes (`XSETCAR`, `XSETCDR`, `ASET`,
/// symbol value writes, etc.).  Neomacs keeps the same Lisp-visible semantics,
/// but records mutation metadata here so future generational or incremental
/// collectors have a single write-barrier surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeapWriteKind {
    ConsCar,
    ConsCdr,
    VectorSlot,
    VectorBulk,
    RecordSlot,
    RecordBulk,
    ClosureSlot,
    ClosureBulk,
    StringTextProps,
    StringData,
    HashTableData,
    ByteCodeData,
    MarkerData,
    OverlayData,
}

/// A single heap mutation event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeapWriteRecord {
    pub owner: TaggedValue,
    pub kind: HeapWriteKind,
    pub slot: Option<usize>,
    pub value: Option<TaggedValue>,
}

impl HeapWriteRecord {
    pub const fn bulk(owner: TaggedValue, kind: HeapWriteKind) -> Self {
        Self {
            owner,
            kind,
            slot: None,
            value: None,
        }
    }

    pub const fn slot(
        owner: TaggedValue,
        kind: HeapWriteKind,
        slot: usize,
        value: TaggedValue,
    ) -> Self {
        Self {
            owner,
            kind,
            slot: Some(slot),
            value: Some(value),
        }
    }
}

// ---------------------------------------------------------------------------
// Opt-in collection toggle
// ---------------------------------------------------------------------------

/// Check whether the environment requests actual neovm-gc collection.
///
/// Default: enabled. Set NEOVM_GC_ENABLE_COLLECTION to 0, false, or no
/// to disable (useful for bisecting a regression back to the old no-op
/// flush_roots_and_collect behavior). Any other value -- empty string,
/// unset, 1, true, yes, anything -- leaves collection on.
fn gc_collection_enabled() -> bool {
    match std::env::var("NEOVM_GC_ENABLE_COLLECTION") {
        Ok(v) => {
            let lower = v.to_ascii_lowercase();
            lower != "0" && lower != "false" && lower != "no"
        }
        Err(_) => true,
    }
}

// ---------------------------------------------------------------------------
// Thread-local heap access
// ---------------------------------------------------------------------------

thread_local! {
    static TAGGED_HEAP: Cell<*mut TaggedHeap> = const { Cell::new(std::ptr::null_mut()) };
    static TAGGED_HEAP_WRITE_TRACKING_MODE: Cell<WriteTrackingMode> =
        const { Cell::new(WriteTrackingMode::Disabled) };
    /// Auto-allocated heap for tests that construct Values without a Context.
    #[cfg(test)]
    static TEST_FALLBACK_TAGGED_HEAP: std::cell::RefCell<Option<Box<TaggedHeap>>> =
        const { std::cell::RefCell::new(None) };
}

/// Set the thread-local tagged heap pointer.
pub fn set_tagged_heap(heap: &mut TaggedHeap) {
    TAGGED_HEAP.with(|h| h.set(heap as *mut TaggedHeap));
    TAGGED_HEAP_WRITE_TRACKING_MODE.with(|mode| mode.set(heap.write_tracking_mode()));
}

/// Access the thread-local tagged heap.
///
/// In test mode, auto-creates a fallback heap if none is set.
/// In production, panics if no heap is set.
#[inline]
pub fn with_tagged_heap<R>(f: impl FnOnce(&mut TaggedHeap) -> R) -> R {
    TAGGED_HEAP.with(|h| {
        let ptr = h.get();
        if !ptr.is_null() {
            return f(unsafe { &mut *ptr });
        }
        #[cfg(test)]
        {
            TEST_FALLBACK_TAGGED_HEAP.with(|fb| {
                let mut borrow = fb.borrow_mut();
                if borrow.is_none() {
                    *borrow = Some(Box::new(TaggedHeap::new()));
                }
                let heap_ref: &mut TaggedHeap = borrow.as_mut().unwrap();
                let ptr = heap_ref as *mut TaggedHeap;
                h.set(ptr);
                f(unsafe { &mut *ptr })
            })
        }
        #[cfg(not(test))]
        {
            panic!("no TaggedHeap set for this thread");
        }
    })
}

/// Central mutation hook for bulk writes to the tagged heap.
#[inline]
pub fn note_heap_write(owner: TaggedValue, kind: HeapWriteKind) {
    note_heap_write_record(HeapWriteRecord::bulk(owner, kind));
}

/// Central mutation hook for slot writes to the tagged heap.
#[inline]
pub fn note_heap_slot_write(
    owner: TaggedValue,
    kind: HeapWriteKind,
    slot: usize,
    value: TaggedValue,
) {
    note_heap_write_record(HeapWriteRecord::slot(owner, kind, slot, value));
}

#[inline]
fn note_heap_write_record(record: HeapWriteRecord) {
    if !record.owner.is_heap_object() {
        return;
    }
    if TAGGED_HEAP_WRITE_TRACKING_MODE.with(|mode| mode.get()) == WriteTrackingMode::Disabled {
        return;
    }
    with_tagged_heap(|heap| heap.record_heap_write(record));
}

// ---------------------------------------------------------------------------
// Tag constants (mirrored from value.rs for pointer encoding)
// ---------------------------------------------------------------------------

const TAG_CONS: usize = 0b010;
const TAG_VECLIKE: usize = 0b011;
const TAG_STRING: usize = 0b100;
const TAG_FLOAT: usize = 0b110;

// ---------------------------------------------------------------------------
// TaggedHeap — the main GC-managed heap
// ---------------------------------------------------------------------------

/// The tagged pointer heap. Owns all heap-allocated Lisp objects.
///
/// Internally delegates allocation and collection to a `neovm_gc::Heap`.
/// The Heap is leaked to obtain a `'static` reference so that on-demand
/// `Mutator` views can be created without self-referential lifetime issues.
///
/// All allocations route through neovm-gc's `SpaceKind::Pinned`. Objects
/// up to [`neovm_gc::spaces::MAX_PINNED_SLOT_SIZE`] bytes are served from
/// the span-based size-class allocator; larger objects fall back to the
/// system allocator. Objects are tracked in the heap's `ObjectStore` so
/// future concurrent marking can enumerate them.
pub struct TaggedHeap {
    /// The neovm-gc Heap, leaked for 'static lifetime.
    gc_heap: &'static neovm_gc::Heap,
    /// Raw pointer for Drop cleanup.
    gc_heap_box: *mut neovm_gc::Heap,
    /// Persistent `Mutator` view so consecutive allocations share one
    /// per-thread `ObjectPublishLocal`. A fresh `Mutator` per allocation
    /// reserves a new 40KB `ObjectStore` chunk on first publish, burning
    /// 1023 slots per single allocation — ~40GB wasted over a bootstrap.
    /// Boxed to obtain a stable address and give the mutator a `'static`
    /// borrow against the leaked `gc_heap`.
    ///
    /// Wrapped in `Option` so `Drop` can take and drop it *before*
    /// reclaiming the `Heap` box, avoiding a use-after-free on the
    /// safepoint read guard the mutator holds against the heap.
    gc_mutator: Option<Box<neovm_gc::Mutator<'static>>>,

    /// Total number of allocated objects (cons + non-cons).
    pub allocated_count: usize,

    /// GC threshold in approximate Lisp heap bytes.
    gc_threshold: usize,
    /// When true, `gc_threshold` was explicitly overridden by tests or host
    /// code and should not be recomputed from Lisp-visible GC variables.
    gc_threshold_overridden: bool,
    /// Approximate Lisp heap bytes allocated since the last full
    /// (Major or Full) collection.
    bytes_since_gc: usize,
    /// Approximate Lisp heap bytes allocated since the last Minor
    /// (or Major/Full) collection. Used to drive
    /// `should_collect_minor` in [`Context::gc_safe_point_exact`].
    bytes_since_minor: usize,
    /// Threshold, in bytes, above which `should_collect_minor`
    /// returns true. Kept independent of `gc_threshold` so the
    /// fast-path cycle can be tuned separately from the full-GC
    /// frequency.
    gc_minor_threshold: usize,
    /// Approximate bytes retained by the live heap after the last sweep.
    live_bytes: usize,

    /// Tracking list of all allocated marker objects for bulk operations
    /// like clearing markers when buffers are killed.
    marker_ptrs: Vec<*mut GcMarker>,

    /// Canonical runtime handle wrappers keyed by their underlying object id.
    buffer_registry: FxHashMap<crate::buffer::BufferId, TaggedValue>,
    window_registry: FxHashMap<u64, TaggedValue>,
    frame_registry: FxHashMap<u64, TaggedValue>,
    timer_registry: FxHashMap<u64, TaggedValue>,

    /// Owners mutated since the last full collection.
    ///
    /// This is the minimal remembered-set precursor for future generational
    /// or incremental GC. We keep owner identity, not child edges, because the
    /// current collector is still full-heap mark-sweep.
    write_tracking_mode: WriteTrackingMode,
    dirty_owners: Vec<TaggedValue>,
    dirty_owner_bits: FxHashSet<usize>,
    dirty_writes: Vec<HeapWriteRecord>,

    /// Buffered external roots collected during `seed_root()` calls between
    /// `begin_collection()` and `complete_collection()`. These are converted
    /// to `GcErased` handles and fed to neovm-gc's external root scanner
    /// so the collector can trace all live VM objects.
    ///
    /// Wrapped in `Arc<Mutex<>>` so the scanner callback installed on the
    /// neovm-gc heap — which must be `FnMut + Send + 'static` and therefore
    /// cannot borrow `self` — can drain the buffer when the marker runs.
    gc_root_buffer: std::sync::Arc<std::sync::Mutex<Vec<GcErased>>>,

    /// Monotonic counter of collection cycles. Drives the simple
    /// pacer in `flush_roots_and_collect`: every `GC_FULL_EVERY`-th
    /// cycle escalates from Minor to Full so nursery evacuation
    /// stays cheap on the fast path while old-gen dead objects
    /// still get reclaimed periodically.
    gc_cycle_counter: u64,
}

/// Every Nth collect cycle runs Full instead of Minor. Chosen
/// empirically: Minor-only cycles are cheap but never reclaim
/// old-gen survivors; Full every cycle is wasteful. 16 keeps Full
/// under ~7% of cycles while still bounding old-gen growth.
const GC_FULL_EVERY: u64 = 16;

impl TaggedHeap {
    /// Create a `Mutator` view for write barrier calls.
    ///
    /// This is `pub(crate)` so that `mutate.rs` can invoke neovm-gc
    /// write barriers without exposing `gc_heap` directly.
    pub(crate) fn mutator(&self) -> neovm_gc::Mutator<'_> {
        self.gc_heap.mutator()
    }

    pub fn new() -> Self {
        let config = neovm_gc::HeapConfig::default();
        let heap_box = Box::into_raw(Box::new(neovm_gc::Heap::new(config)));
        let gc_heap: &'static neovm_gc::Heap = unsafe { &*heap_box };
        let mut mutator = gc_heap.mutator();
        // Pin the safepoint so subsequent allocations reuse the same
        // ObjectPublishLocal chunk reservations.
        mutator.pin_safepoint();
        let gc_mutator = Some(Box::new(mutator));

        // Install the external-root scanner callback. When the collector
        // runs, it calls this closure to discover every VM-owned root.
        // The closure drains `gc_root_buffer` (populated by the VM's
        // `trace_roots()` pass via `seed_root()`) into the Vec supplied
        // by the collector.
        let gc_root_buffer: std::sync::Arc<std::sync::Mutex<Vec<GcErased>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        {
            let roots = std::sync::Arc::clone(&gc_root_buffer);
            gc_heap.set_external_root_scanner(move |out: &mut Vec<GcErased>| {
                let mut guard = roots.lock().expect("gc_root_buffer lock poisoned");
                out.extend(guard.drain(..));
            });
        }

        // Install the external-root relocator callback. When an
        // evacuating collection (Minor or Full) forwards objects out
        // of the nursery, the collector invokes this closure so the
        // VM can rewrite every live tagged pointer to its
        // post-evacuation address. Finds the live Context via
        // `GC_RELOCATOR_CONTEXT`, a thread-local raw pointer that
        // `gc_collect_from_current_roots` installs around the
        // collect call.
        gc_heap.set_external_root_relocator(
            move |relocator: &mut dyn neovm_gc::descriptor::Relocator| {
                let ctx_ptr = crate::emacs_core::eval::GC_RELOCATOR_CONTEXT
                    .with(|slot| slot.get());
                if ctx_ptr.is_null() {
                    return;
                }
                // SAFETY: STW collection runs on the same thread that
                // installed `ctx_ptr`, and no other code path reads
                // the Context for the duration of the collect call.
                let ctx = unsafe { &mut *ctx_ptr };
                ctx.trace_roots_mut(&mut |slot: &mut TaggedValue| {
                    relocate_tagged_slot(slot, relocator);
                });
            },
        );
        Self {
            gc_heap,
            gc_heap_box: heap_box,
            gc_mutator,
            allocated_count: 0,
            gc_threshold: 100_000 * size_of::<usize>(),
            gc_threshold_overridden: false,
            bytes_since_gc: 0,
            bytes_since_minor: 0,
            // Phase gamma landing compromise: Minor evacuation is
            // live for GcFloat but something still dangles when it
            // fires during cons-heavy bootstrap. Disable the
            // Minor-threshold trigger for now so Full (every 16th
            // cycle) is the only path that evacuates the Nursery.
            // This keeps the pause profile bounded (Major skips
            // Nursery reclaim -> cheaper than Pinned-cons Major)
            // while we audit the remaining non-rewritten Value
            // holder. Safe-point callers can still run an explicit
            // Minor via collect_minor if they handle the root
            // seeding themselves.
            gc_minor_threshold: usize::MAX,
            live_bytes: 0,
            marker_ptrs: Vec::new(),
            buffer_registry: FxHashMap::default(),
            window_registry: FxHashMap::default(),
            frame_registry: FxHashMap::default(),
            timer_registry: FxHashMap::default(),
            write_tracking_mode: WriteTrackingMode::Disabled,
            dirty_owners: Vec::new(),
            dirty_owner_bits: FxHashSet::default(),
            dirty_writes: Vec::new(),
            gc_root_buffer,
            gc_cycle_counter: 0,
        }
    }

    pub fn set_stack_bottom(&mut self, bottom: *const u8) {
        let _ = bottom;
    }

    pub fn set_write_tracking_mode(&mut self, mode: WriteTrackingMode) {
        self.write_tracking_mode = mode;
        TAGGED_HEAP_WRITE_TRACKING_MODE.with(|current| current.set(mode));
        if mode == WriteTrackingMode::Disabled {
            self.clear_dirty_owners();
            self.clear_dirty_writes();
        }
    }

    pub fn write_tracking_mode(&self) -> WriteTrackingMode {
        self.write_tracking_mode
    }

    /// Get neovm-gc heap statistics snapshot.
    pub fn gc_heap_stats(&self) -> neovm_gc::stats::HeapStats {
        self.gc_heap.stats()
    }

    pub fn should_collect(&self) -> bool {
        self.bytes_since_gc >= self.gc_threshold
    }

    /// Returns true if enough nursery traffic has accumulated since
    /// the last Minor collection to make a fast-path Minor
    /// worthwhile. Independent of [`Self::should_collect`]; both
    /// can fire in the same safe point.
    pub fn should_collect_minor(&self) -> bool {
        self.bytes_since_minor >= self.gc_minor_threshold
    }

    pub fn gc_minor_threshold(&self) -> usize {
        self.gc_minor_threshold
    }

    pub fn set_gc_minor_threshold(&mut self, threshold: usize) {
        self.gc_minor_threshold = threshold.max(1);
    }

    pub fn gc_threshold(&self) -> usize {
        self.gc_threshold
    }

    pub fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold.max(1);
        self.gc_threshold_overridden = true;
    }

    pub fn set_gc_threshold_from_runtime(&mut self, threshold: usize) {
        if !self.gc_threshold_overridden {
            self.gc_threshold = threshold.max(1);
        }
    }

    pub fn clear_gc_threshold_override(&mut self) {
        self.gc_threshold_overridden = false;
    }

    pub fn gc_threshold_is_overridden(&self) -> bool {
        self.gc_threshold_overridden
    }

    pub fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    pub fn bytes_since_gc(&self) -> usize {
        self.bytes_since_gc
    }

    pub fn live_bytes(&self) -> usize {
        self.live_bytes
    }

    pub fn buffer_value(&self, id: crate::buffer::BufferId) -> Option<TaggedValue> {
        self.buffer_registry.get(&id).copied()
    }

    pub fn register_buffer_value(&mut self, id: crate::buffer::BufferId, value: TaggedValue) {
        self.buffer_registry.insert(id, value);
    }

    pub fn window_value(&self, id: u64) -> Option<TaggedValue> {
        self.window_registry.get(&id).copied()
    }

    pub fn register_window_value(&mut self, id: u64, value: TaggedValue) {
        self.window_registry.insert(id, value);
    }

    pub fn frame_value(&self, id: u64) -> Option<TaggedValue> {
        self.frame_registry.get(&id).copied()
    }

    pub fn register_frame_value(&mut self, id: u64, value: TaggedValue) {
        self.frame_registry.insert(id, value);
    }

    pub fn timer_value(&self, id: u64) -> Option<TaggedValue> {
        self.timer_registry.get(&id).copied()
    }

    pub fn register_timer_value(&mut self, id: u64, value: TaggedValue) {
        self.timer_registry.insert(id, value);
    }

    pub fn dirty_owner_count(&self) -> usize {
        self.dirty_owners.len()
    }

    pub fn is_dirty_owner(&self, owner: TaggedValue) -> bool {
        self.dirty_owner_bits.contains(&owner.bits())
    }

    pub fn take_dirty_owners(&mut self) -> Vec<TaggedValue> {
        self.dirty_owner_bits.clear();
        std::mem::take(&mut self.dirty_owners)
    }

    pub fn clear_dirty_owners(&mut self) {
        self.dirty_owners.clear();
        self.dirty_owner_bits.clear();
    }

    pub fn dirty_write_count(&self) -> usize {
        self.dirty_writes.len()
    }

    pub fn dirty_writes(&self) -> &[HeapWriteRecord] {
        &self.dirty_writes
    }

    pub fn take_dirty_writes(&mut self) -> Vec<HeapWriteRecord> {
        std::mem::take(&mut self.dirty_writes)
    }

    pub fn clear_dirty_writes(&mut self) {
        self.dirty_writes.clear();
    }

    fn record_heap_write(&mut self, record: HeapWriteRecord) {
        if self.write_tracking_mode == WriteTrackingMode::Disabled {
            return;
        }
        if self.dirty_owner_bits.insert(record.owner.bits()) {
            self.dirty_owners.push(record.owner);
        }
        if self.write_tracking_mode == WriteTrackingMode::OwnersAndRecords {
            self.dirty_writes.push(record);
        }
    }

    fn note_allocation_bytes(&mut self, bytes: usize) {
        self.bytes_since_gc = self.bytes_since_gc.saturating_add(bytes);
        self.bytes_since_minor = self.bytes_since_minor.saturating_add(bytes);
        self.live_bytes = self.live_bytes.saturating_add(bytes);
    }

    // -----------------------------------------------------------------------
    // Internal helper: allocate through neovm-gc and extract payload pointer
    // -----------------------------------------------------------------------

    /// Allocate a value of type T through the neovm-gc Heap and return
    /// the raw payload pointer. The object is managed by neovm-gc from
    /// this point forward.
    ///
    /// Routes through `Mutator::alloc_pinned_raw` — small objects land
    /// in span-based size-class pools; large objects fall back to the
    /// system allocator. All records are published to neovm-gc's
    /// `ObjectStore` so the marker can enumerate them.
    fn gc_alloc<T: neovm_gc::Trace + 'static>(&mut self, value: T) -> *const T {
        let mutator = self
            .gc_mutator
            .as_mut()
            .expect("gc_mutator is only None during Drop");
        let ptr = mutator
            .alloc_pinned_raw(value)
            .expect("pinned allocation should succeed");
        ptr.as_ptr() as *const T
    }

    // -----------------------------------------------------------------------
    // Allocation
    // -----------------------------------------------------------------------

    /// Allocate a cons cell. Returns a tagged Value.
    pub fn alloc_cons(&mut self, car: TaggedValue, cdr: TaggedValue) -> TaggedValue {
        let gc_cons = GcCons {
            car: GcSlot::new(car),
            cdr: GcSlot::new(cdr),
        };
        let mutator = self
            .gc_mutator
            .as_mut()
            .expect("gc_mutator is only None during Drop");
        let ptr = mutator
            .alloc_external_raw(gc_cons)
            .expect("cons allocation should succeed");
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcCons>());
        TaggedValue(ptr.as_ptr() as usize | TAG_CONS)
    }

    /// Allocate a string object.
    pub fn alloc_string(&mut self, s: crate::heap_types::LispString) -> TaggedValue {
        let byte_len = s.byte_len();
        let gc_str = GcLispString {
            data: s,
            text_props: TextPropertyTable::new(),
        };
        let ptr = self.gc_alloc(gc_str);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcLispString>().saturating_add(byte_len));
        TaggedValue(ptr as usize | TAG_STRING)
    }

    /// Allocate a float object.
    ///
    /// Routes through the policy-aware `alloc_external_raw` path so that
    /// flipping `GcFloat::move_policy()` to `Movable` (phase beta of the
    /// moving-nursery design) transparently lands allocations in the
    /// nursery.
    pub fn alloc_float(&mut self, value: f64) -> TaggedValue {
        let gc_float = GcFloat { value };
        let mutator = self
            .gc_mutator
            .as_mut()
            .expect("gc_mutator is only None during Drop");
        let ptr = mutator
            .alloc_external_raw(gc_float)
            .expect("float allocation should succeed");
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcFloat>());
        TaggedValue(ptr.as_ptr() as usize | TAG_FLOAT)
    }

    /// Allocate a canonical subr object.
    pub fn alloc_subr(
        &mut self,
        name: crate::emacs_core::intern::NameId,
        function: Option<SubrFn>,
        min_args: u16,
        max_args: Option<u16>,
        dispatch_kind: SubrDispatchKind,
    ) -> TaggedValue {
        let gc_subr = GcSubr {
            type_tag: VecLikeType::Subr,
            name,
            function,
            min_args,
            max_args,
            dispatch_kind,
        };
        let ptr = self.gc_alloc(gc_subr);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcSubr>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a vector.
    pub fn alloc_vector(&mut self, items: Vec<TaggedValue>) -> TaggedValue {
        let storage_bytes = items.capacity().saturating_mul(size_of::<TaggedValue>());
        let gc_vec = GcVector {
            type_tag: VecLikeType::Vector,
            items: UnsafeCell::new(items),
        };
        let ptr = self.gc_alloc(gc_vec);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcVector>().saturating_add(storage_bytes));
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a hash table.
    pub fn alloc_hash_table(
        &mut self,
        table: crate::emacs_core::value::LispHashTable,
    ) -> TaggedValue {
        let gc_ht = GcHashTable {
            type_tag: VecLikeType::HashTable,
            table: UnsafeCell::new(table),
        };
        let ptr = self.gc_alloc(gc_ht);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcHashTable>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a lambda (interpreted closure) as a Value vector.
    /// Matches GNU Emacs's PVEC_CLOSURE: all slots are GC-traced Values.
    pub fn alloc_lambda(&mut self, slots: Vec<TaggedValue>) -> TaggedValue {
        let storage_bytes = slots.capacity().saturating_mul(size_of::<TaggedValue>());
        let gc_lambda = GcLambda {
            type_tag: VecLikeType::Lambda,
            data: UnsafeCell::new(slots),
        };
        let ptr = self.gc_alloc(gc_lambda);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcLambda>().saturating_add(storage_bytes));
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a lambda from a LambdaData (bridge for migration).
    /// Converts LambdaData fields to the Value vector layout.
    pub fn alloc_lambda_from_data(
        &mut self,
        data: crate::emacs_core::value::LambdaData,
    ) -> TaggedValue {
        let slots = data.to_closure_slots();
        self.alloc_lambda(slots)
    }

    /// Allocate a macro as a Value vector.
    pub fn alloc_macro(&mut self, slots: Vec<TaggedValue>) -> TaggedValue {
        let storage_bytes = slots.capacity().saturating_mul(size_of::<TaggedValue>());
        let gc_macro = GcMacro {
            type_tag: VecLikeType::Macro,
            data: UnsafeCell::new(slots),
        };
        let ptr = self.gc_alloc(gc_macro);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcMacro>().saturating_add(storage_bytes));
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a macro from a LambdaData (bridge for migration).
    pub fn alloc_macro_from_data(
        &mut self,
        data: crate::emacs_core::value::LambdaData,
    ) -> TaggedValue {
        let slots = data.to_closure_slots();
        self.alloc_macro(slots)
    }

    /// Allocate a buffer reference.
    pub fn alloc_buffer(&mut self, id: crate::buffer::BufferId) -> TaggedValue {
        let gc_buf = GcBuffer { type_tag: VecLikeType::Buffer, id };
        let ptr = self.gc_alloc(gc_buf);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcBuffer>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a window reference.
    pub fn alloc_window(&mut self, id: u64) -> TaggedValue {
        let gc_win = GcWindow { type_tag: VecLikeType::Window, id };
        let ptr = self.gc_alloc(gc_win);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcWindow>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a frame reference.
    pub fn alloc_frame(&mut self, id: u64) -> TaggedValue {
        let gc_frame = GcFrame { type_tag: VecLikeType::Frame, id };
        let ptr = self.gc_alloc(gc_frame);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcFrame>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a timer reference.
    pub fn alloc_timer(&mut self, id: u64) -> TaggedValue {
        let gc_timer = GcTimer { type_tag: VecLikeType::Timer, id };
        let ptr = self.gc_alloc(gc_timer);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcTimer>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a bytecode function.
    pub fn alloc_bytecode(
        &mut self,
        data: crate::emacs_core::bytecode::ByteCodeFunction,
    ) -> TaggedValue {
        let gc_bc = GcByteCode {
            type_tag: VecLikeType::ByteCode,
            data: UnsafeCell::new(data),
        };
        let ptr = self.gc_alloc(gc_bc);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcByteCode>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a record.
    pub fn alloc_record(&mut self, items: Vec<TaggedValue>) -> TaggedValue {
        let storage_bytes = items.capacity().saturating_mul(size_of::<TaggedValue>());
        let gc_record = GcRecord {
            type_tag: VecLikeType::Record,
            items: UnsafeCell::new(items),
        };
        let ptr = self.gc_alloc(gc_record);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcRecord>().saturating_add(storage_bytes));
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate an overlay.
    pub fn alloc_overlay(&mut self, data: crate::heap_types::OverlayData) -> TaggedValue {
        let gc_overlay = GcOverlay {
            type_tag: VecLikeType::Overlay,
            data: UnsafeCell::new(data),
        };
        let ptr = self.gc_alloc(gc_overlay);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcOverlay>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a marker.
    pub fn alloc_marker(&mut self, data: crate::heap_types::MarkerData) -> TaggedValue {
        let gc_marker = GcMarker { type_tag: VecLikeType::Marker, data };
        let ptr = self.gc_alloc(gc_marker);
        self.marker_ptrs.push(ptr as *mut GcMarker);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcMarker>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    /// Allocate a bignum (arbitrary-precision integer).
    ///
    /// Mirrors GNU `make_bignum` (`src/bignum.c:113`): the caller is
    /// responsible for ensuring the value is outside fixnum range.
    /// Use `Value::make_integer` for the canonical "fixnum-or-bignum"
    /// constructor that delegates here only when promotion is needed.
    pub fn alloc_bignum(&mut self, value: rug::Integer) -> TaggedValue {
        let gc_bignum = GcBignum { type_tag: VecLikeType::Bignum, value };
        let ptr = self.gc_alloc(gc_bignum);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcBignum>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    pub fn alloc_symbol_with_pos(&mut self, sym: TaggedValue, pos: TaggedValue) -> TaggedValue {
        let gc_swp = GcSymbolWithPos { type_tag: VecLikeType::SymbolWithPos, sym, pos };
        let ptr = self.gc_alloc(gc_swp);
        self.allocated_count += 1;
        self.note_allocation_bytes(size_of::<GcSymbolWithPos>());
        TaggedValue(ptr as usize | TAG_VECLIKE)
    }

    // -----------------------------------------------------------------------
    // Marker operations
    // -----------------------------------------------------------------------

    /// Clear buffer association for all markers belonging to any of the
    /// killed buffers.
    pub fn clear_markers_for_buffers<S>(
        &mut self,
        killed: &std::collections::HashSet<crate::buffer::BufferId, S>,
    ) where
        S: std::hash::BuildHasher,
    {
        for ptr in &self.marker_ptrs {
            let marker = unsafe { &mut (**ptr).data };
            if marker.buffer.is_some_and(|b| killed.contains(&b)) {
                marker.buffer = None;
                marker.position = None;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Garbage collection — delegates to neovm-gc
    // -----------------------------------------------------------------------

    /// Convert a heap-tagged `TaggedValue` to a `GcErased` handle.
    ///
    /// The value must be a heap pointer (cons, string, float, or veclike).
    /// Non-heap values (fixnums, symbols, nil, t, immediates) must be
    /// filtered out by the caller.
    ///
    /// # Safety
    ///
    /// The tagged pointer must reference a live object allocated through
    /// neovm-gc (i.e., its payload sits after an `ObjectHeader`).
    unsafe fn tagged_to_erased(val: TaggedValue) -> GcErased {
        let payload_ptr = (val.0 & !0b111) as *const u8;
        let gc: Gc<u8> = unsafe { Gc::from_payload_ptr(payload_ptr) };
        gc.erase()
    }

    /// Buffer a single root for the current collection cycle.
    ///
    /// If `root` is a heap-tagged pointer, its `GcErased` handle is
    /// appended to `gc_root_buffer`. Non-heap values are silently
    /// ignored.
    fn buffer_root(&mut self, root: TaggedValue) {
        if root.is_heap_object() {
            let erased = unsafe { Self::tagged_to_erased(root) };
            self.gc_root_buffer
                .lock()
                .expect("gc_root_buffer lock poisoned")
                .push(erased);
        }
    }

    /// Trigger a major collection on the neovm-gc heap.
    ///
    /// The external root scanner installed in [`Self::new`] drains
    /// `gc_root_buffer` into the collector's root vec. SATB write
    /// barriers fired from `gc_post_write_barrier` feed the marker's
    /// edge updates. After the mark phase, the sweep reclaims dead
    /// objects from pinned span pools.
    ///
    /// Currently this runs one full Major cycle synchronously
    /// (begin_mark + mark + remark + reclaim). An earlier attempt
    /// split the work across safe points via `begin_major_mark` +
    /// `assist_major_mark` + `finish_major_collection`, but the
    /// sweep/reclaim phase (finish_major_collection) still has to
    /// run in a single STW step, and that's where most of the cost
    /// is -- splitting the mark alone didn't reduce observed pause
    /// times. Meaningful pause reduction needs either (a) a
    /// background worker thread (requires SharedHeap) or (b) a
    /// cheaper sweep inside neovm-gc.
    ///
    /// Enabled by default; set `NEOVM_GC_ENABLE_COLLECTION=0` to
    /// disable (useful for bisecting whether a regression involves
    /// actual collection vs the surrounding infrastructure).
    fn flush_roots_and_collect(&mut self) {
        if gc_collection_enabled() {
            if let Some(mutator) = self.gc_mutator.as_mut() {
                // Pacer: Major is the default — it sweeps Pinned
                // and Old without evacuating the Nursery, matching
                // pre-phase-beta behavior for every allocation type
                // except GcFloat. Every GC_FULL_EVERY-th cycle
                // escalates to Full so the Nursery gets evacuated
                // and Movable types (currently just GcFloat) go
                // through the relocator. Minor is deliberately not
                // used here because today Pinned dominates the
                // working set; once more types flip to Movable in
                // Phase γ / ε this ratio inverts and Minor becomes
                // the fast path.
                self.gc_cycle_counter = self.gc_cycle_counter.wrapping_add(1);
                let kind = if self.gc_cycle_counter % GC_FULL_EVERY == 0 {
                    neovm_gc::plan::CollectionKind::Full
                } else {
                    neovm_gc::plan::CollectionKind::Major
                };
                let _ = mutator.collect(kind);
            }
            self.resync_post_collection_counters();
        }
        self.clear_post_collection_residue();
    }

    /// Run one Minor collection cycle with the already-seeded roots.
    ///
    /// Minor evacuates the Nursery and runs the relocator; it does
    /// not sweep Pinned or reclaim Old. Cheap (target <5ms) and safe
    /// to run between Major cycles so Movable-type allocations don't
    /// accumulate in the Nursery between heavier reclaims.
    ///
    /// Only `bytes_since_minor` resets here — `bytes_since_gc`
    /// continues to accumulate so the Major/Full threshold still
    /// fires on schedule.
    fn flush_roots_and_collect_minor(&mut self) {
        if gc_collection_enabled() {
            if let Some(mutator) = self.gc_mutator.as_mut() {
                let _ = mutator.collect(neovm_gc::plan::CollectionKind::Minor);
            }
            self.resync_post_collection_counters();
        }
        self.gc_root_buffer
            .lock()
            .expect("gc_root_buffer lock poisoned")
            .clear();
        self.bytes_since_minor = 0;
        // Dirty-card / dirty-owner tracking is consumed by the
        // collection itself (see `collect_dirty_card_root_indices`
        // / legacy remembered-set paths) so we clear it here too.
        // `bytes_since_gc` is intentionally not reset — Minor is a
        // fast-path cycle that still counts toward the full-GC
        // threshold.
        self.clear_dirty_owners();
        self.clear_dirty_writes();
    }

    fn resync_post_collection_counters(&mut self) {
        // Resync our cumulative allocation counter with neovm-gc's
        // live object count so callers that treat allocated_count
        // as "currently live" see the post-sweep value.
        self.allocated_count = self.gc_heap.object_count();
        self.live_bytes = self.gc_heap.stats().pinned.live_bytes;
    }

    fn clear_post_collection_residue(&mut self) {
        // External root scanner drained gc_root_buffer during the
        // collection (or we are skipping it); clear any residue.
        self.gc_root_buffer
            .lock()
            .expect("gc_root_buffer lock poisoned")
            .clear();
        self.bytes_since_gc = 0;
        self.bytes_since_minor = 0;
        self.clear_dirty_owners();
        self.clear_dirty_writes();
    }

    /// Run a garbage collection using the explicit roots provided.
    pub fn collect(&mut self, roots: impl Iterator<Item = TaggedValue>) {
        self.begin_collection();
        for root in roots {
            self.seed_root(root);
        }
        self.flush_roots_and_collect();
    }

    /// Run a collection using only the explicit roots provided.
    pub fn collect_exact(&mut self, roots: impl Iterator<Item = TaggedValue>) {
        self.begin_collection();
        for root in roots {
            self.seed_root(root);
        }
        self.flush_roots_and_collect();
    }

    /// Begin a collection cycle (called before `seed_root` calls).
    ///
    /// Clears the root buffer in preparation for the VM's root
    /// enumeration pass.
    pub(crate) fn begin_collection(&mut self) {
        self.gc_root_buffer
            .lock()
            .expect("gc_root_buffer lock poisoned")
            .clear();
    }

    /// Buffer a single VM root for the in-progress collection cycle.
    ///
    /// Called repeatedly by `Context::trace_roots()` between
    /// `begin_collection()` and `complete_collection()`. Each
    /// heap-tagged value is converted to a `GcErased` handle and
    /// accumulated in the root buffer.
    pub(crate) fn seed_root(&mut self, root: TaggedValue) {
        self.buffer_root(root);
    }

    /// Finish the collection cycle: install buffered roots and collect.
    ///
    /// Takes the accumulated root buffer, registers it as the external
    /// root scanner callback on the neovm-gc heap, and triggers a
    /// `Minor` collection. The collector calls the scanner to discover
    /// all external roots, then traces through `Trace` implementations
    /// on the `Gc*` types to find the full reachable object graph.
    pub(crate) fn complete_collection(&mut self) {
        self.flush_roots_and_collect();
    }

    /// Variant of [`Self::complete_collection`] that runs only a
    /// Minor cycle. Roots must already be seeded via `seed_root`
    /// between a prior `begin_collection` and this call.
    pub(crate) fn complete_collection_minor(&mut self) {
        self.flush_roots_and_collect_minor();
    }
}

/// Rewrite a heap-tagged `TaggedValue` through a collector-supplied
/// `Relocator` so the slot's payload pointer tracks its new location
/// after evacuation. No-op for non-heap values.
fn relocate_tagged_slot(
    slot: &mut TaggedValue,
    relocator: &mut dyn neovm_gc::descriptor::Relocator,
) {
    if !slot.is_heap_object() {
        return;
    }
    let tag = slot.0 & 0b111;
    let payload_ptr = (slot.0 & !0b111) as *const u8;
    // SAFETY: caller guarantees the tagged pointer references a live
    // object allocated through neovm-gc (its payload sits after an
    // ObjectHeader).
    let gc: Gc<u8> = unsafe { Gc::from_payload_ptr(payload_ptr) };
    let erased = gc.erase();
    let new_erased = relocator.relocate_erased(erased);
    let new_gc: Gc<u8> = unsafe { Gc::<u8>::from_erased(new_erased) };
    let new_payload = new_gc.payload_ptr() as usize;
    slot.0 = new_payload | tag;
}

impl Drop for TaggedHeap {
    fn drop(&mut self) {
        // Clear the thread-local pointer if it still refers to us so
        // callers that outlive this heap fall back to
        // `TEST_FALLBACK_TAGGED_HEAP` (in test mode) or panic cleanly
        // (in release) instead of dereferencing a dangling pointer.
        let self_ptr = self as *mut TaggedHeap;
        TAGGED_HEAP.with(|h| {
            if h.get() == self_ptr {
                h.set(std::ptr::null_mut());
            }
        });
        // Drop the mutator BEFORE reclaiming the Heap box: the mutator
        // holds a safepoint read guard borrowed from the heap, and
        // dropping the heap first would invalidate that borrow.
        self.gc_mutator.take();
        // Reclaim the leaked neovm-gc Heap.
        unsafe {
            drop(Box::from_raw(self.gc_heap_box));
        }
    }
}

// ---------------------------------------------------------------------------
// neovm-gc write barrier bridge
// ---------------------------------------------------------------------------

/// Call the neovm-gc post-write barrier after a tagged value mutation.
///
/// `owner` is the tagged value being mutated (cons, vector, record, closure).
/// `slot` is the logical slot index within the owner.
/// `old_val` is the value that was in the slot BEFORE the store (needed for SATB).
/// `new_val` is the value that was just written.
///
/// This is a no-op when the owner is not a heap object, or when the heap
/// is not yet initialized (e.g., during pdump deserialization).
#[inline]
pub fn gc_post_write_barrier(
    owner: TaggedValue,
    slot: usize,
    old_val: TaggedValue,
    new_val: TaggedValue,
) {
    if !owner.is_heap_object() {
        return;
    }
    if TAGGED_HEAP.with(|h| h.get().is_null()) {
        return;
    }
    with_tagged_heap(|heap| {
        let Some(mutator) = heap.gc_mutator.as_mut() else {
            return;
        };
        let owner_gc = unsafe { TaggedHeap::tagged_to_erased(owner) };
        let old_gc = old_val
            .is_heap_object()
            .then(|| unsafe { TaggedHeap::tagged_to_erased(old_val) });
        let new_gc = new_val
            .is_heap_object()
            .then(|| unsafe { TaggedHeap::tagged_to_erased(new_val) });
        let owner_gc: neovm_gc::root::Gc<u8> = unsafe { neovm_gc::root::Gc::from_erased(owner_gc) };
        let old_gc = old_gc.map(|e| unsafe { neovm_gc::root::Gc::<u8>::from_erased(e) });
        let new_gc = new_gc.map(|e| unsafe { neovm_gc::root::Gc::<u8>::from_erased(e) });
        mutator.post_write_barrier(owner_gc, Some(slot), old_gc, new_gc);
    });
}

/// Call the neovm-gc post-write barrier for a bulk mutation (no single old value).
///
/// Used when the caller cannot cheaply produce the old slot value (e.g.,
/// `fillarray`, `copy-sequence`, `delete!`). SATB still needs to see the
/// mutation, so we fire the barrier with `old_value = None` and let the
/// marker re-scan the owner during its next visit.
#[inline]
pub fn gc_post_write_barrier_bulk(owner: TaggedValue) {
    if !owner.is_heap_object() {
        return;
    }
    if TAGGED_HEAP.with(|h| h.get().is_null()) {
        return;
    }
    with_tagged_heap(|heap| {
        let Some(mutator) = heap.gc_mutator.as_mut() else {
            return;
        };
        let owner_erased = unsafe { TaggedHeap::tagged_to_erased(owner) };
        let owner_gc: neovm_gc::root::Gc<u8> =
            unsafe { neovm_gc::root::Gc::from_erased(owner_erased) };
        mutator.post_write_barrier::<u8, u8>(owner_gc, None, None, None);
    });
}

pub fn read_stack_end_from_proc() -> Option<usize> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        if line.contains("[stack]") {
            let dash = line.find('-')?;
            let space = line.find(' ')?;
            let end_hex = &line[dash + 1..space];
            return usize::from_str_radix(end_hex, 16).ok();
        }
    }
    None
}

#[cfg(test)]
mod arena_tests {
    use super::*;

    #[test]
    fn arena_cons_round_trip() {
        let mut heap = TaggedHeap::new();
        let car = TaggedValue::fixnum(42);
        let cdr = TaggedValue::fixnum(99);
        let val = heap.alloc_cons(car, cdr);
        assert!(val.is_cons());
        let read_car = unsafe { (*val.xcons_ptr()).car };
        let read_cdr = unsafe { (*val.xcons_ptr()).cdr() };
        assert_eq!(read_car.0, car.0, "car mismatch: got {:#x}, expected {:#x}", read_car.0, car.0);
        assert_eq!(read_cdr.0, cdr.0, "cdr mismatch: got {:#x}, expected {:#x}", read_cdr.0, cdr.0);
    }

    #[test]
    fn arena_float_round_trip() {
        let mut heap = TaggedHeap::new();
        let val = heap.alloc_float(3.14);
        assert!(val.is_float());
        let ptr = (val.0 & !0b111) as *const super::super::gc_trace_impls::GcFloat;
        let read = unsafe { (*ptr).value };
        assert_eq!(read, 3.14);
    }
}

#[cfg(test)]
mod arena_tests2 {
    use super::*;
    use crate::emacs_core::value::Value;

    #[test]
    fn arena_vector_round_trip() {
        let mut heap = TaggedHeap::new();
        crate::tagged::gc::set_tagged_heap(&mut heap);
        let v = Value::vector(vec![Value::fixnum(1), Value::fixnum(2), Value::fixnum(3)]);
        assert!(v.is_vector());
        let data = v.as_vector_data().unwrap();
        assert_eq!(data.len(), 3);
        assert_eq!(data[0], Value::fixnum(1));
        assert_eq!(data[1], Value::fixnum(2));
        assert_eq!(data[2], Value::fixnum(3));
    }

    #[test]
    fn arena_string_round_trip() {
        let mut heap = TaggedHeap::new();
        crate::tagged::gc::set_tagged_heap(&mut heap);
        let v = Value::string("hello");
        assert!(v.is_string());
        let s = v.as_utf8_str().unwrap();
        assert_eq!(s, "hello");
    }
}
