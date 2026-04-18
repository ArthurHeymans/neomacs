//! Centralized tagged-heap mutation helpers.
//!
//! These functions are the single place to hook future generational or
//! incremental write barriers into the tagged runtime.

use crate::buffer::text_props::TextPropertyTable;
use crate::emacs_core::bytecode::ByteCodeFunction;
use crate::emacs_core::value::LispHashTable;
use crate::heap_types::{LispString, MarkerData, OverlayData};

use super::gc::{
    HeapWriteKind, gc_post_write_barrier, gc_post_write_barrier_bulk, note_heap_slot_write,
    note_heap_write,
};
use super::gc_trace_impls::{
    GcByteCode, GcHashTable, GcLambda, GcLispString, GcMacro, GcMarker, GcOverlay, GcRecord,
    GcVector,
};
use super::header::{ConsCell, VecLikeType};
use super::value::TaggedValue;

#[inline]
pub fn set_cons_car(cell: TaggedValue, value: TaggedValue) -> bool {
    if !cell.is_cons() {
        return false;
    }
    // Read old value BEFORE store (SATB needs it).
    let old_value = unsafe { (*cell.xcons_ptr()).car };
    note_heap_slot_write(cell, HeapWriteKind::ConsCar, 0, value);
    unsafe {
        (*(cell.xcons_ptr() as *mut ConsCell)).set_car(value);
    }
    gc_post_write_barrier(cell, 0, old_value, value);
    true
}

#[inline]
pub fn set_cons_cdr(cell: TaggedValue, value: TaggedValue) -> bool {
    if !cell.is_cons() {
        return false;
    }
    // Read old value BEFORE store (SATB needs it).
    let old_value = unsafe { (*cell.xcons_ptr()).cdr() };
    note_heap_slot_write(cell, HeapWriteKind::ConsCdr, 1, value);
    unsafe {
        (*(cell.xcons_ptr() as *mut ConsCell)).set_cdr(value);
    }
    gc_post_write_barrier(cell, 1, old_value, value);
    true
}

#[inline]
pub fn with_vector_data_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut Vec<TaggedValue>) -> R,
) -> Option<R> {
    if value.veclike_type()? != VecLikeType::Vector {
        return None;
    }
    note_heap_write(value, HeapWriteKind::VectorBulk);
    let ptr = value.as_veclike_ptr().unwrap() as *const GcVector;
    let result = f(unsafe { &mut *(*ptr).items.get() });
    gc_post_write_barrier_bulk(value);
    Some(result)
}

#[inline]
pub fn replace_vector_data(value: TaggedValue, items: Vec<TaggedValue>) -> bool {
    with_vector_data_mut(value, |data| *data = items).is_some()
}

#[inline]
pub fn set_vector_slot(value: TaggedValue, index: usize, item: TaggedValue) -> bool {
    if value.veclike_type() != Some(VecLikeType::Vector) {
        return false;
    }
    let ptr = value.as_veclike_ptr().unwrap() as *const GcVector;
    let data = unsafe { &mut *(*ptr).items.get() };
    let slot = match data.get_mut(index) {
        Some(slot) => slot,
        None => return false,
    };
    // Read old value BEFORE store (SATB needs it).
    let old_value = *slot;
    note_heap_slot_write(value, HeapWriteKind::VectorSlot, index, item);
    *slot = item;
    gc_post_write_barrier(value, index, old_value, item);
    true
}

#[inline]
pub fn with_record_data_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut Vec<TaggedValue>) -> R,
) -> Option<R> {
    if value.veclike_type()? != VecLikeType::Record {
        return None;
    }
    note_heap_write(value, HeapWriteKind::RecordBulk);
    let ptr = value.as_veclike_ptr().unwrap() as *const GcRecord;
    let result = f(unsafe { &mut *(*ptr).items.get() });
    gc_post_write_barrier_bulk(value);
    Some(result)
}

#[inline]
pub fn replace_record_data(value: TaggedValue, items: Vec<TaggedValue>) -> bool {
    with_record_data_mut(value, |data| *data = items).is_some()
}

#[inline]
pub fn set_record_slot(value: TaggedValue, index: usize, item: TaggedValue) -> bool {
    if value.veclike_type() != Some(VecLikeType::Record) {
        return false;
    }
    let ptr = value.as_veclike_ptr().unwrap() as *const GcRecord;
    let data = unsafe { &mut *(*ptr).items.get() };
    let slot = match data.get_mut(index) {
        Some(slot) => slot,
        None => return false,
    };
    // Read old value BEFORE store (SATB needs it).
    let old_value = *slot;
    note_heap_slot_write(value, HeapWriteKind::RecordSlot, index, item);
    *slot = item;
    gc_post_write_barrier(value, index, old_value, item);
    true
}

#[inline]
pub fn with_closure_slots_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut Vec<TaggedValue>) -> R,
) -> Option<R> {
    note_heap_write(value, HeapWriteKind::ClosureBulk);
    let result = match value.veclike_type()? {
        VecLikeType::Lambda => {
            let ptr = value.as_veclike_ptr().unwrap() as *const GcLambda;
            Some(f(unsafe { &mut *(*ptr).data.get() }))
        }
        VecLikeType::Macro => {
            let ptr = value.as_veclike_ptr().unwrap() as *const GcMacro;
            Some(f(unsafe { &mut *(*ptr).data.get() }))
        }
        _ => None,
    };
    if result.is_some() {
        gc_post_write_barrier_bulk(value);
    }
    result
}

#[inline]
pub fn replace_closure_slots(value: TaggedValue, slots: Vec<TaggedValue>) -> bool {
    with_closure_slots_mut(value, |data| *data = slots).is_some()
}

#[inline]
pub fn set_closure_slot(value: TaggedValue, index: usize, item: TaggedValue) -> bool {
    match value.veclike_type() {
        Some(VecLikeType::Lambda) => unsafe {
            let ptr = value.as_veclike_ptr().unwrap() as *const GcLambda;
            let data = &mut *(*ptr).data.get();
            let slot = match data.get_mut(index) {
                Some(slot) => slot,
                None => return false,
            };
            // Read old value BEFORE store (SATB needs it).
            let old_value = *slot;
            note_heap_slot_write(value, HeapWriteKind::ClosureSlot, index, item);
            *slot = item;
            gc_post_write_barrier(value, index, old_value, item);
            true
        },
        Some(VecLikeType::Macro) => unsafe {
            let ptr = value.as_veclike_ptr().unwrap() as *const GcMacro;
            let data = &mut *(*ptr).data.get();
            let slot = match data.get_mut(index) {
                Some(slot) => slot,
                None => return false,
            };
            // Read old value BEFORE store (SATB needs it).
            let old_value = *slot;
            note_heap_slot_write(value, HeapWriteKind::ClosureSlot, index, item);
            *slot = item;
            gc_post_write_barrier(value, index, old_value, item);
            true
        },
        _ => false,
    }
}

#[inline]
pub fn with_string_text_props_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut TextPropertyTable) -> R,
) -> Option<R> {
    let ptr = value.as_string_ptr()? as *mut GcLispString;
    note_heap_write(value, HeapWriteKind::StringTextProps);
    // text_props is wrapped in UnsafeCell so `Trace::relocate`
    // can rebuild it during STW evacuation (see gc_trace_impls.rs).
    let result = f(unsafe { &mut *(*ptr).text_props.get() });
    gc_post_write_barrier_bulk(value);
    Some(result)
}

#[inline]
pub fn with_lisp_string_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut LispString) -> R,
) -> Option<R> {
    let ptr = value.as_string_ptr()? as *mut GcLispString;
    note_heap_write(value, HeapWriteKind::StringData);
    // LispString does not contain TaggedValue fields — no GC edge barrier needed.
    Some(f(unsafe { &mut (*ptr).data }))
}

#[inline]
pub fn with_hash_table_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut LispHashTable) -> R,
) -> Option<R> {
    if value.veclike_type()? != VecLikeType::HashTable {
        return None;
    }
    note_heap_write(value, HeapWriteKind::HashTableData);
    let ptr = value.as_veclike_ptr().unwrap() as *const GcHashTable;
    let result = f(unsafe { &mut *(*ptr).table.get() });
    gc_post_write_barrier_bulk(value);
    Some(result)
}

#[inline]
pub fn with_bytecode_data_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut ByteCodeFunction) -> R,
) -> Option<R> {
    if value.veclike_type()? != VecLikeType::ByteCode {
        return None;
    }
    note_heap_write(value, HeapWriteKind::ByteCodeData);
    let ptr = value.as_veclike_ptr().unwrap() as *const GcByteCode;
    let result = f(unsafe { &mut *(*ptr).data.get() });
    gc_post_write_barrier_bulk(value);
    Some(result)
}

#[inline]
pub fn with_marker_data_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut MarkerData) -> R,
) -> Option<R> {
    if value.veclike_type()? != VecLikeType::Marker {
        return None;
    }
    note_heap_write(value, HeapWriteKind::MarkerData);
    let ptr = value.as_veclike_ptr().unwrap() as *mut GcMarker;
    // MarkerData does not contain TaggedValue fields — no GC edge barrier needed.
    Some(f(unsafe { &mut (*ptr).data }))
}

#[inline]
pub fn with_overlay_data_mut<R>(
    value: TaggedValue,
    f: impl FnOnce(&mut OverlayData) -> R,
) -> Option<R> {
    if value.veclike_type()? != VecLikeType::Overlay {
        return None;
    }
    note_heap_write(value, HeapWriteKind::OverlayData);
    let ptr = value.as_veclike_ptr().unwrap() as *const GcOverlay;
    let result = f(unsafe { &mut *(*ptr).data.get() });
    gc_post_write_barrier_bulk(value);
    Some(result)
}
