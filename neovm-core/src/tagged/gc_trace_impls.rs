//! Trace implementations for neovm-gc integration.
//!
//! Each Lisp heap type gets a corresponding struct implementing
//! `neovm_gc::descriptor::Trace` so the collector can discover
//! GC edges (outgoing tagged-value pointers) and optionally
//! relocate them after compaction.
//!
//! These types mirror the existing header.rs types but are designed
//! for allocation through neovm-gc instead of TaggedHeap.

use std::cell::UnsafeCell;

use neovm_gc::descriptor::{GcErased, LayoutKind, MovePolicy, Relocator, Trace, Tracer};
use neovm_gc::root::Gc;

use super::value::TaggedValue;
use crate::buffer::text_props::TextPropertyTable;
use crate::emacs_core::bytecode::ByteCodeFunction;
use crate::emacs_core::value::LispHashTable;
use crate::heap_types::{LispString, MarkerData, OverlayData};

// ---------------------------------------------------------------------------
// UnsafeCell wrapper for TaggedValue fields that need mutation during relocate.
// The collector guarantees exclusive access during relocation, so interior
// mutability through UnsafeCell is safe under that contract.
// ---------------------------------------------------------------------------

/// A TaggedValue slot that can be mutated during GC relocation.
#[repr(transparent)]
pub struct GcSlot(UnsafeCell<TaggedValue>);

impl GcSlot {
    #[inline]
    pub fn new(val: TaggedValue) -> Self {
        Self(UnsafeCell::new(val))
    }

    #[inline]
    pub fn get(&self) -> TaggedValue {
        unsafe { *self.0.get() }
    }

    #[inline]
    pub fn set(&self, val: TaggedValue) {
        unsafe { *self.0.get() = val; }
    }

    #[inline]
    fn get_mut_ptr(&self) -> &mut TaggedValue {
        unsafe { &mut *self.0.get() }
    }
}

// ---------------------------------------------------------------------------
// Helper: trace a TaggedValue if it points to a heap-managed object.
// ---------------------------------------------------------------------------

/// Check whether a TaggedValue points to a heap object (cons, string,
/// float, or vectorlike). Fixnums, symbols, nil, t, and immediates
/// are not heap pointers and need no tracing.
#[inline]
fn is_heap_tagged(val: TaggedValue) -> bool {
    // Tags 010 (cons), 011 (veclike), 100 (string), 110 (float)
    // are heap pointers. Tags 000 (symbol), xx1 (fixnum), 111 (immediate)
    // are not.
    let tag = val.0 & 0b111;
    matches!(tag, 0b010 | 0b011 | 0b100 | 0b110)
}

/// Recover a GcErased handle from a tagged heap pointer.
///
/// # Safety
///
/// `val` must be a heap-tagged value whose pointer was allocated
/// through neovm-gc (i.e., the payload sits after an ObjectHeader).
#[inline]
unsafe fn tagged_to_gc_erased(val: TaggedValue) -> GcErased {
    let payload_ptr = (val.0 & !0b111) as *const u8;
    let gc: Gc<u8> = unsafe { Gc::from_payload_ptr(payload_ptr) };
    gc.erase()
}

/// Trace a TaggedValue: if it's a heap pointer, mark the target object.
#[inline]
fn trace_tagged(tracer: &mut dyn Tracer, val: TaggedValue) {
    if is_heap_tagged(val) {
        let erased = unsafe { tagged_to_gc_erased(val) };
        tracer.mark_erased(erased);
    }
}

/// Relocate a TaggedValue in place: if the target was moved, update
/// the pointer while preserving the tag bits.
#[inline]
fn relocate_tagged(relocator: &mut dyn Relocator, slot: &mut TaggedValue) {
    if !is_heap_tagged(*slot) {
        return;
    }
    let tag = slot.0 & 0b111;
    let old_erased = unsafe { tagged_to_gc_erased(*slot) };
    let new_erased = relocator.relocate_erased(old_erased);
    if old_erased != new_erased {
        // Recover the new payload pointer from the new GcErased
        let new_gc: Gc<u8> = unsafe { Gc::from_erased(new_erased) };
        let new_payload = new_gc.payload_ptr() as usize;
        *slot = TaggedValue(new_payload | tag);
    }
}

// ---------------------------------------------------------------------------
// GcCons — cons cell for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Cons cell managed by neovm-gc. Two TaggedValue fields wrapped in
/// GcSlot for interior mutability during relocation.
#[repr(C)]
pub struct GcCons {
    pub car: GcSlot,
    pub cdr: GcSlot,
}

unsafe impl Trace for GcCons {
    fn trace(&self, tracer: &mut dyn Tracer) {
        trace_tagged(tracer, self.car.get());
        trace_tagged(tracer, self.cdr.get());
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        relocate_tagged(relocator, self.car.get_mut_ptr());
        relocate_tagged(relocator, self.cdr.get_mut_ptr());
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }
}

// ---------------------------------------------------------------------------
// GcFloat — float for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Float managed by neovm-gc. Leaf object (no GC edges).
#[repr(C)]
pub struct GcFloat {
    pub value: f64,
}

unsafe impl Trace for GcFloat {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }
}

// ---------------------------------------------------------------------------
// GcLispString — string for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Lisp string managed by neovm-gc. Text properties may contain
/// TaggedValue edges that must be traced.
pub struct GcLispString {
    pub data: LispString,
    pub text_props: TextPropertyTable,
}

unsafe impl Trace for GcLispString {
    fn trace(&self, tracer: &mut dyn Tracer) {
        // Text properties contain TaggedValue edges in their
        // PropertyInterval HashMap<Value, Value> entries.
        for interval in self.text_props.intervals_snapshot() {
            for (key, val) in interval.ordered_properties() {
                trace_tagged(tracer, key);
                trace_tagged(tracer, *val);
            }
        }
    }

    fn relocate(&self, _relocator: &mut dyn Relocator) {
        // Text property relocation requires mutable access to the
        // PropertyInterval HashMaps. For now, text properties are
        // relocated via the fixup pass on root slots and interior
        // edges. A full implementation would rebuild the HashMap
        // with relocated keys.
        //
        // TODO: implement interior text property relocation when
        // compaction is enabled.
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External // owns Vec<u8> backing store
    }
}

// ---------------------------------------------------------------------------
// GcVector — vector for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Lisp vector managed by neovm-gc. All elements are TaggedValue edges.
pub struct GcVector {
    pub items: UnsafeCell<Vec<TaggedValue>>,
}

unsafe impl Trace for GcVector {
    fn trace(&self, tracer: &mut dyn Tracer) {
        for item in unsafe { &*self.items.get() } {
            trace_tagged(tracer, *item);
        }
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        for item in unsafe { &mut *self.items.get() } {
            relocate_tagged(relocator, item);
        }
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

// ---------------------------------------------------------------------------
// GcHashTable — hash table for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Lisp hash table managed by neovm-gc.
pub struct GcHashTable {
    pub table: UnsafeCell<LispHashTable>,
}

unsafe impl Trace for GcHashTable {
    fn trace(&self, tracer: &mut dyn Tracer) {
        let table = unsafe { &*self.table.get() };
        for value in table.data.values() {
            trace_tagged(tracer, *value);
        }
        for value in table.key_snapshots.values() {
            trace_tagged(tracer, *value);
        }
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        let table = unsafe { &mut *self.table.get() };
        for value in table.data.values_mut() {
            relocate_tagged(relocator, value);
        }
        for value in table.key_snapshots.values_mut() {
            relocate_tagged(relocator, value);
        }
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

// ---------------------------------------------------------------------------
// GcLambda / GcMacro — closure types for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Lisp lambda (interpreted closure) managed by neovm-gc.
/// All slots are TaggedValue edges.
pub struct GcLambda {
    pub data: UnsafeCell<Vec<TaggedValue>>,
}

unsafe impl Trace for GcLambda {
    fn trace(&self, tracer: &mut dyn Tracer) {
        for slot in unsafe { &*self.data.get() } {
            trace_tagged(tracer, *slot);
        }
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        for slot in unsafe { &mut *self.data.get() } {
            relocate_tagged(relocator, slot);
        }
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

/// Lisp macro — same structure as lambda.
pub struct GcMacro {
    pub data: UnsafeCell<Vec<TaggedValue>>,
}

unsafe impl Trace for GcMacro {
    fn trace(&self, tracer: &mut dyn Tracer) {
        for slot in unsafe { &*self.data.get() } {
            trace_tagged(tracer, *slot);
        }
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        for slot in unsafe { &mut *self.data.get() } {
            relocate_tagged(relocator, slot);
        }
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

// ---------------------------------------------------------------------------
// GcByteCode — bytecode function for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Bytecode function managed by neovm-gc. The constants vector
/// contains TaggedValue edges.
pub struct GcByteCode {
    pub data: UnsafeCell<ByteCodeFunction>,
}

unsafe impl Trace for GcByteCode {
    fn trace(&self, tracer: &mut dyn Tracer) {
        let data = unsafe { &*self.data.get() };
        for val in &data.constants {
            trace_tagged(tracer, *val);
        }
        if let Some(env) = data.env {
            trace_tagged(tracer, env);
        }
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        let data = unsafe { &mut *self.data.get() };
        for val in &mut data.constants {
            relocate_tagged(relocator, val);
        }
        if let Some(ref mut env) = data.env {
            relocate_tagged(relocator, env);
        }
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

// ---------------------------------------------------------------------------
// GcRecord — record for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Record managed by neovm-gc (vector-like with type tag in slot 0).
pub struct GcRecord {
    pub items: UnsafeCell<Vec<TaggedValue>>,
}

unsafe impl Trace for GcRecord {
    fn trace(&self, tracer: &mut dyn Tracer) {
        for item in unsafe { &*self.items.get() } {
            trace_tagged(tracer, *item);
        }
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        for item in unsafe { &mut *self.items.get() } {
            relocate_tagged(relocator, item);
        }
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

// ---------------------------------------------------------------------------
// GcOverlay — overlay for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Buffer overlay managed by neovm-gc. The plist contains TaggedValue edges.
pub struct GcOverlay {
    pub data: UnsafeCell<OverlayData>,
}

unsafe impl Trace for GcOverlay {
    fn trace(&self, tracer: &mut dyn Tracer) {
        let data = unsafe { &*self.data.get() };
        trace_tagged(tracer, data.plist);
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        let data = unsafe { &mut *self.data.get() };
        relocate_tagged(relocator, &mut data.plist);
    }

    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }
}

// ---------------------------------------------------------------------------
// Leaf types — no GC edges
// ---------------------------------------------------------------------------

/// Buffer marker managed by neovm-gc. No TaggedValue edges.
pub struct GcMarker {
    pub data: MarkerData,
}

unsafe impl Trace for GcMarker {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy { MovePolicy::Movable }
}

/// Bignum managed by neovm-gc. No TaggedValue edges.
pub struct GcBignum {
    pub value: rug::Integer,
}

unsafe impl Trace for GcBignum {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy { MovePolicy::Movable }
    fn layout_kind() -> LayoutKind { LayoutKind::External }
}

/// Buffer reference managed by neovm-gc. No TaggedValue edges.
pub struct GcBuffer {
    pub id: crate::buffer::BufferId,
}

unsafe impl Trace for GcBuffer {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy { MovePolicy::Movable }
}

/// Window reference managed by neovm-gc. No TaggedValue edges.
pub struct GcWindow {
    pub id: u64,
}

unsafe impl Trace for GcWindow {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy { MovePolicy::Movable }
}

/// Frame reference managed by neovm-gc. No TaggedValue edges.
pub struct GcFrame {
    pub id: u64,
}

unsafe impl Trace for GcFrame {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy { MovePolicy::Movable }
}

/// Timer reference managed by neovm-gc. No TaggedValue edges.
pub struct GcTimer {
    pub id: u64,
}

unsafe impl Trace for GcTimer {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy { MovePolicy::Movable }
}

/// Built-in function managed by neovm-gc. No TaggedValue edges.
pub struct GcSubr {
    pub name: crate::emacs_core::intern::NameId,
    pub function: Option<super::header::SubrFn>,
    pub min_args: u16,
    pub max_args: Option<u16>,
    pub dispatch_kind: super::header::SubrDispatchKind,
}

unsafe impl Trace for GcSubr {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy { MovePolicy::Pinned }
}
