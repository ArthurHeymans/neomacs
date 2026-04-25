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

use neovm_gc::descriptor::{GcErased, LayoutKind, MovePolicy, Relocator, Trace, Tracer, TypeFlags};
use neovm_gc::root::Gc;

use super::header::VecLikeType;
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
        unsafe {
            *self.0.get() = val;
        }
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
        // Phase gamma: cons cells dominate allocation volume, so
        // routing them through the moving nursery is the bulk of
        // the pause-time win. trace() / relocate() visit both car
        // and cdr via GcSlot interior mutability; Context
        // roots + card-based remembered set + legacy remembered
        // set + relocate_thread_local_gc_roots together cover every
        // live Value slot that could point at a nursery cons after
        // evacuation.
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
        // Phase beta canary: GcFloat is a leaf (no heap edges), so
        // flipping it to Movable is the minimum-risk first test of
        // the end-to-end moving-nursery path.
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
    /// UnsafeCell so `Trace::relocate(&self, ...)` can rebuild
    /// each property interval's `HashMap<Value, Value>` in place
    /// after the collector moves a key/value Value out from
    /// under it. Mutation is safe because relocate only fires
    /// during STW.
    pub text_props: UnsafeCell<TextPropertyTable>,
}

unsafe impl Trace for GcLispString {
    fn trace(&self, tracer: &mut dyn Tracer) {
        let text_props = unsafe { &*self.text_props.get() };
        // Text properties contain TaggedValue edges in their
        // PropertyInterval HashMap<Value, Value> entries.
        for interval in text_props.intervals_snapshot() {
            for (key, val) in interval.ordered_properties() {
                trace_tagged(tracer, key);
                trace_tagged(tracer, *val);
            }
        }
    }

    fn relocate(&self, relocator: &mut dyn Relocator) {
        // Rebuild each interval's `HashMap<Value, Value>` via the
        // same drain-and-reinsert path used by the VM-side
        // `TextPropertyTable::trace_roots_mut`, but feed the
        // relocator instead of a generic visitor. This keeps
        // bucket indexes consistent with the post-evacuation
        // pointer values in the keys.
        let text_props = unsafe { &mut *self.text_props.get() };
        crate::gc_trace::GcTrace::trace_roots_mut(text_props, &mut |slot| {
            relocate_tagged(relocator, slot);
        });
    }

    fn move_policy() -> MovePolicy {
        // Phase epsilon: fresh strings typically have empty
        // text_props (most allocations come from `(make-string ...)`
        // or literal reads, which don't install properties), so
        // the Nursery fast path is a natural fit. The relocate()
        // path above handles the rare populated-properties case
        // by rebuilding each interval's HashMap after evacuation.
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
#[repr(C)]
pub struct GcVector {
    pub type_tag: VecLikeType,
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
        // Phase epsilon: vectors move through the nursery.
        // GcVector metadata (type_tag + Vec header) is fixed-size
        // and safe to evacuate; the Vec's backing store lives in
        // malloc so its pointer stays valid across the struct's
        // evacuation. All TaggedValue elements get rewritten via
        // relocate() above.
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
#[repr(C)]
pub struct GcHashTable {
    pub type_tag: VecLikeType,
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
        // eq-hash tables use `HashKey::Ptr(usize) = Value::bits()`
        // as their stored hash-key form. When a Movable keyed
        // object evacuates, the Ptr bits are stale AND the
        // HashMap's bucket index for the key is wrong relative
        // to the post-evacuation hash.
        //
        // Fix: drain both `data` and `key_snapshots`, walk each
        // HashKey and each Value through the relocator, then
        // reinsert into the same (now-empty but still-allocated)
        // HashMap. Drain reuses the existing bucket array, so no
        // allocation happens during GC; reinsert refills those
        // buckets against the post-evacuation hash.
        let table = unsafe { &mut *self.table.get() };
        rebuild_hashmap_with_relocator(&mut table.data, relocator);
        rebuild_hashmap_with_relocator(&mut table.key_snapshots, relocator);
        for key in &mut table.insertion_order {
            relocate_hashkey(key, relocator);
        }
    }

    fn move_policy() -> MovePolicy {
        // Phase epsilon: hash tables move through the nursery
        // now that `relocate` rebuilds their HashMap keys.
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

/// Drain a `HashMap<HashKey, Value>`, relocate both the key's
/// embedded pointer bits and the value's tagged pointer, then
/// reinsert so bucket indexes regenerate against post-evacuation
/// hashes.
fn rebuild_hashmap_with_relocator(
    map: &mut std::collections::HashMap<crate::emacs_core::value::HashKey, TaggedValue>,
    relocator: &mut dyn Relocator,
) {
    let taken = std::mem::take(map);
    let mut rebuilt = std::collections::HashMap::with_capacity(taken.len());
    for (mut k, mut v) in taken {
        relocate_hashkey(&mut k, relocator);
        relocate_tagged(relocator, &mut v);
        rebuilt.insert(k, v);
    }
    *map = rebuilt;
}

/// Rewrite heap-pointer bits nested inside a `HashKey`. Primitive
/// variants (Int, Float, Symbol, etc.) are no-ops; only `Ptr`,
/// `EqualCons`, and `EqualVec` carry storage that can go stale
/// across evacuation.
fn relocate_hashkey(key: &mut crate::emacs_core::value::HashKey, relocator: &mut dyn Relocator) {
    use crate::emacs_core::value::HashKey;
    match key {
        HashKey::Ptr(bits) => {
            let mut tv = TaggedValue(*bits);
            if tv.is_heap_object() {
                relocate_tagged(relocator, &mut tv);
                *bits = tv.0;
            }
        }
        HashKey::EqualCons(head, tail) => {
            relocate_hashkey(head.as_mut(), relocator);
            relocate_hashkey(tail.as_mut(), relocator);
        }
        HashKey::EqualVec(items) => {
            for item in items.iter_mut() {
                relocate_hashkey(item, relocator);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// GcLambda / GcMacro — closure types for neovm-gc allocation
// ---------------------------------------------------------------------------

/// Lisp lambda (interpreted closure) managed by neovm-gc.
/// All slots are TaggedValue edges.
#[repr(C)]
pub struct GcLambda {
    pub type_tag: VecLikeType,
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
        // Interpreted closures have the same "outer header + owned
        // Vec<TaggedValue>" shape as movable vectors and records:
        // evacuating the wrapper is memcpy-safe and relocate() rewrites
        // every embedded edge after the move.
        MovePolicy::Movable
    }

    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

/// Lisp macro — same structure as lambda.
#[repr(C)]
pub struct GcMacro {
    pub type_tag: VecLikeType,
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
        // Macros share the same representation and relocation behavior as
        // interpreted closures, so they can move through the nursery too.
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
#[repr(C)]
pub struct GcByteCode {
    pub type_tag: VecLikeType,
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
        if let Some(doc) = data.doc_form {
            trace_tagged(tracer, doc);
        }
        if let Some(interactive) = data.interactive {
            trace_tagged(tracer, interactive);
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
        if let Some(ref mut doc) = data.doc_form {
            relocate_tagged(relocator, doc);
        }
        if let Some(ref mut interactive) = data.interactive {
            relocate_tagged(relocator, interactive);
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
#[repr(C)]
pub struct GcRecord {
    pub type_tag: VecLikeType,
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
        // Phase epsilon: records have the same shape as vectors
        // (type_tag + Vec<TaggedValue>), so the same Movable
        // reasoning applies.
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
#[repr(C)]
pub struct GcOverlay {
    pub type_tag: VecLikeType,
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
#[repr(C)]
pub struct GcMarker {
    pub type_tag: VecLikeType,
    pub data: MarkerData,
}

unsafe impl Trace for GcMarker {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn finalize(&self) {
        let marker = self as *const GcMarker as *mut GcMarker;
        super::gc::with_tagged_heap(|heap| unsafe {
            heap.unlink_marker_from_registered_chains(marker);
        });
    }
    fn type_flags() -> TypeFlags {
        TypeFlags::FINALIZABLE
    }
    fn move_policy() -> MovePolicy {
        MovePolicy::Pinned
    }
}

/// Bignum managed by neovm-gc. No TaggedValue edges.
#[repr(C)]
pub struct GcBignum {
    pub type_tag: VecLikeType,
    pub value: rug::Integer,
}

unsafe impl Trace for GcBignum {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy {
        // Phase epsilon: bignums have no TaggedValue edges and
        // the rug::Integer wrapper is memcpy-safe -- its GMP
        // limb pointer is owned by malloc and survives the
        // struct's relocation.
        MovePolicy::Movable
    }
    fn layout_kind() -> LayoutKind {
        LayoutKind::External
    }
}

/// Symbol-with-position managed by neovm-gc. Two TaggedValue edges.
#[repr(C)]
pub struct GcSymbolWithPos {
    pub type_tag: VecLikeType,
    /// The bare symbol (TAG_SYMBOL).
    pub sym: TaggedValue,
    /// Source byte offset (fixnum).
    pub pos: TaggedValue,
}

unsafe impl Trace for GcSymbolWithPos {
    fn trace(&self, tracer: &mut dyn Tracer) {
        trace_tagged(tracer, self.sym);
        trace_tagged(tracer, self.pos);
    }
    fn relocate(&self, _relocator: &mut dyn Relocator) {
        // Safe no-op: GcSymbolWithPos.sym is always TAG_SYMBOL (not a
        // heap pointer) and .pos is always a fixnum. Neither can
        // ever be a heap-tagged value that the collector might move,
        // so there is nothing to relocate. If that invariant ever
        // changes (e.g. a future version allows a heap-tagged
        // position for detailed source maps), convert sym/pos to
        // UnsafeCell<TaggedValue> and mirror trace() here.
    }
    fn move_policy() -> MovePolicy {
        // Phase epsilon: symbol-with-pos has no heap edges
        // (sym is TAG_SYMBOL, pos is a fixnum) so the memcpy
        // evacuation is safe. The Pinned status was only there
        // because phase alpha did not need to flip it.
        MovePolicy::Movable
    }
}

/// Buffer reference managed by neovm-gc. No TaggedValue edges.
#[repr(C)]
pub struct GcBuffer {
    pub type_tag: VecLikeType,
    pub id: crate::buffer::BufferId,
}

unsafe impl Trace for GcBuffer {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }
}

/// Window reference managed by neovm-gc. No TaggedValue edges.
#[repr(C)]
pub struct GcWindow {
    pub type_tag: VecLikeType,
    pub id: u64,
}

unsafe impl Trace for GcWindow {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }
}

/// Frame reference managed by neovm-gc. No TaggedValue edges.
#[repr(C)]
pub struct GcFrame {
    pub type_tag: VecLikeType,
    pub id: u64,
}

unsafe impl Trace for GcFrame {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }
}

/// Timer reference managed by neovm-gc. No TaggedValue edges.
#[repr(C)]
pub struct GcTimer {
    pub type_tag: VecLikeType,
    pub id: u64,
}

unsafe impl Trace for GcTimer {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }
}

/// Built-in function managed by neovm-gc. No TaggedValue edges.
#[repr(C)]
pub struct GcSubr {
    pub type_tag: VecLikeType,
    pub name: crate::emacs_core::intern::NameId,
    pub function: Option<super::header::SubrFn>,
    pub min_args: u16,
    pub max_args: Option<u16>,
    pub dispatch_kind: super::header::SubrDispatchKind,
}

unsafe impl Trace for GcSubr {
    fn trace(&self, _tracer: &mut dyn Tracer) {}
    fn relocate(&self, _relocator: &mut dyn Relocator) {}
    fn move_policy() -> MovePolicy {
        MovePolicy::Movable
    }
}
