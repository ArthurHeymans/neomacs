//! Centralized tagged-heap mutation helpers.
//!
//! These functions are the single place to hook future generational or
//! incremental write barriers into the tagged runtime.

use crate::buffer::text_props::TextPropertyTable;
use crate::emacs_core::bytecode::ByteCodeFunction;
use crate::emacs_core::value::LispHashTable;
use crate::gc_trace::GcTrace;
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
fn record_bulk_tagged_edges(
    owner: TaggedValue,
    values: impl IntoIterator<Item = TaggedValue>,
    start_index: usize,
) {
    for (offset, value) in values.into_iter().enumerate() {
        gc_post_write_barrier(owner, start_index + offset, TaggedValue::NIL, value);
    }
}

#[inline]
fn record_bulk_text_property_edges(owner: TaggedValue, table: &mut TextPropertyTable) {
    let mut slot = 0usize;
    table.trace_roots_mut(&mut |value| {
        gc_post_write_barrier(owner, slot, TaggedValue::NIL, *value);
        slot += 1;
    });
}

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
    let items = unsafe { &*(*ptr).items.get() };
    record_bulk_tagged_edges(value, items.iter().copied(), 0);
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
    let items = unsafe { &*(*ptr).items.get() };
    record_bulk_tagged_edges(value, items.iter().copied(), 0);
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
        match value.veclike_type() {
            Some(VecLikeType::Lambda) => unsafe {
                let ptr = value.as_veclike_ptr().unwrap() as *const GcLambda;
                record_bulk_tagged_edges(value, (&*(*ptr).data.get()).iter().copied(), 0);
            },
            Some(VecLikeType::Macro) => unsafe {
                let ptr = value.as_veclike_ptr().unwrap() as *const GcMacro;
                record_bulk_tagged_edges(value, (&*(*ptr).data.get()).iter().copied(), 0);
            },
            _ => {}
        }
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
    let table = unsafe { &mut *(*ptr).text_props.get() };
    record_bulk_text_property_edges(value, table);
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
    let table = unsafe { &*(*ptr).table.get() };
    record_bulk_tagged_edges(value, table.data.values().copied(), 0);
    record_bulk_tagged_edges(
        value,
        table.key_snapshots.values().copied(),
        table.data.len(),
    );
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
    let data = unsafe { &*(*ptr).data.get() };
    record_bulk_tagged_edges(value, data.constants.iter().copied(), 0);
    let mut next_index = data.constants.len();
    if let Some(env) = data.env {
        gc_post_write_barrier(value, next_index, TaggedValue::NIL, env);
        next_index += 1;
    }
    if let Some(doc) = data.doc_form {
        gc_post_write_barrier(value, next_index, TaggedValue::NIL, doc);
        next_index += 1;
    }
    if let Some(interactive) = data.interactive {
        gc_post_write_barrier(value, next_index, TaggedValue::NIL, interactive);
    }
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
    let data = unsafe { &*(*ptr).data.get() };
    record_bulk_tagged_edges(value, [data.plist], 0);
    Some(result)
}
