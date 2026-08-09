//! Undo system for buffers — GNU Emacs–compatible Lisp list approach.
//!
//! The undo list is stored as a direct Lisp `Value` in the buffer-local
//! property `buffer-undo-list`.  This module provides helper functions
//! that manipulate that `Value` list, matching GNU Emacs's undo.c:
//!
//! - `t` means undo is disabled
//! - `nil` means undo is enabled with an empty list
//! - Records are cons-ed onto the FRONT (most recent first)
//!
//! Entry types:
//! - `(BEG . END)` — insertion (1-indexed positions)
//! - `(TEXT . POS)` — deletion (TEXT is string, POS is 1-indexed,
//!   negative if point was at end of deleted region)
//! - `POS` (integer) — cursor position (1-indexed)
//! - `(t . MODTIME)` — first-change marker
//! - `nil` — undo boundary

use crate::emacs_core::eval::{
    push_scratch_gc_root, restore_scratch_gc_roots, save_scratch_gc_roots,
};
use crate::emacs_core::value::{Value, ValueKind};
use crate::heap_types::LispString;

use super::{CharLen, CharPos0, CharRange};

fn prepend_undo_entry(undo_list: &mut Value, entry: Value) {
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(*undo_list);
    push_scratch_gc_root(entry);
    *undo_list = Value::cons(entry, *undo_list);
    restore_scratch_gc_roots(saved);
}

/// Returns `true` when `buffer-undo-list` is `t` (undo disabled).
pub fn undo_list_is_disabled(undo_list: &Value) -> bool {
    undo_list.is_t()
}

/// Record that text was inserted at character position `beg` with character
/// length `len`.  Positions stored in the list are 1-indexed.
///
/// If we are right at an undo boundary (head is nil or list is empty)
/// and `pt` != `beg`, a cursor-position entry is recorded first.
///
/// Consecutive adjacent inserts are merged when the head entry is an
/// insert whose END equals `beg+1` (the 1-indexed start of the new
/// insert).
pub fn undo_list_record_insert(
    undo_list: &mut Value,
    beg: CharPos0,
    len: CharLen,
    point_before_command_or_undo: Option<CharPos0>,
) {
    // GNU `record_insert` (undo.c) returns early only for a disabled undo
    // list; a zero-length insertion still conses `(BEG . BEG)`.  That record
    // is load-bearing, because `record_insert` coalesces into the newest
    // record only when that record is an insertion ending where the new one
    // begins — a zero-length record breaks the chain between two adjacent
    // change runs.
    if undo_list_is_disabled(undo_list) {
        return;
    }

    let at_boundary = undo_list.is_nil() || (undo_list.is_cons() && undo_list.cons_car().is_nil());
    if let Some(point) = point_before_command_or_undo
        && at_boundary
        && point != beg
    {
        undo_list_record_point(undo_list, point);
    }

    let beg1 = beg.to_lisp().as_i64();
    let end1 = beg.add_len(len).to_lisp().as_i64();

    // Try to merge with the head entry if it's an adjacent insert.
    if undo_list.is_cons() {
        let head = undo_list.cons_car();
        if head.is_cons() {
            let car = head.cons_car();
            let cdr = head.cons_cdr();
            if let (Some(prev_beg), Some(prev_end)) = (car.as_fixnum(), cdr.as_fixnum()) {
                if prev_end == beg1 {
                    // Merge: extend the existing insert entry.
                    head.set_cdr(Value::fixnum(prev_end + len.get() as i64));
                    return;
                }
                // Check if insert is at the beginning of the previous range
                if prev_beg == end1 {
                    head.set_car(Value::fixnum(beg1));
                    return;
                }
                let _ = (prev_beg, prev_end); // suppress unused warnings
            }
        }
    }

    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(*undo_list);
    let entry = Value::cons(Value::fixnum(beg1), Value::fixnum(end1));
    push_scratch_gc_root(entry);
    *undo_list = Value::cons(entry, *undo_list);
    restore_scratch_gc_roots(saved);
}

/// Record the point-position undo entry that GNU `record_point (beg)` conses
/// at the start of `record_delete`/`record_insert`.
///
/// GNU records `point_before_last_command_or_undo` (point as it was at the
/// beginning of the command) — *not* the current point — and only when the
/// undo list is currently at a boundary and that saved point differs from
/// `beg` (the start of the change).  This must run *before* any
/// marker-adjustment entries are consed so the resulting undo list keeps the
/// point entry ahead of them (GNU bug 16818 ordering).
pub fn undo_list_record_point_for_change(
    undo_list: &mut Value,
    beg: CharPos0,
    point_before_command_or_undo: Option<CharPos0>,
) {
    if undo_list_is_disabled(undo_list) {
        return;
    }
    let at_boundary = undo_list.is_nil() || (undo_list.is_cons() && undo_list.cons_car().is_nil());
    if let Some(point) = point_before_command_or_undo
        && at_boundary
        && point != beg
    {
        undo_list_record_point(undo_list, point);
    }
}

/// Record a deletion.  `beg` is the 0-indexed character position, `text` is
/// the deleted string, `pt` is the 0-indexed cursor character position at
/// the time of deletion.
///
/// The stored position is 1-indexed and negative when `pt` was at the
/// END of the deleted region (i.e. `pt == beg + SCHARS (text)`).
///
/// When `record_point` is `false` the caller has already recorded the
/// point-position entry (GNU records it *before* marker adjustments, so the
/// marker-adjustment path records it itself and suppresses it here).
pub fn undo_list_record_delete_with_point(
    undo_list: &mut Value,
    beg: CharPos0,
    text: LispString,
    pt: CharPos0,
    point_before_command_or_undo: Option<CharPos0>,
    record_point: bool,
) {
    // GNU `record_delete` (undo.c) returns early only for a disabled undo
    // list; it never tests the string's length, so a zero-length deletion
    // still conses `("" . POS)`.  See [`undo_list_record_insert`] for why an
    // empty record matters.
    if undo_list_is_disabled(undo_list) {
        return;
    }

    if record_point {
        undo_list_record_point_for_change(undo_list, beg, point_before_command_or_undo);
    }

    let pos1 = beg.to_lisp().as_i64();
    let stored_pos = if pt == beg.add_len(CharLen::new(text.schars())) {
        -pos1
    } else {
        pos1
    };

    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(*undo_list);
    let text = Value::heap_string(text);
    push_scratch_gc_root(text);
    let entry = Value::cons(text, Value::fixnum(stored_pos));
    push_scratch_gc_root(entry);
    *undo_list = Value::cons(entry, *undo_list);
    restore_scratch_gc_roots(saved);
}

/// Record a deletion, recording the point-position entry first (the common
/// case with no marker adjustments).  See [`undo_list_record_delete_with_point`].
pub fn undo_list_record_delete(
    undo_list: &mut Value,
    beg: CharPos0,
    text: LispString,
    pt: CharPos0,
    point_before_command_or_undo: Option<CharPos0>,
) {
    undo_list_record_delete_with_point(
        undo_list,
        beg,
        text,
        pt,
        point_before_command_or_undo,
        true,
    );
}

/// Record a marker adjustment immediately before a deletion record.
///
/// GNU `record_marker_adjustments` conses `(MARKER . ADJUSTMENT)` entries
/// before `record_delete` conses `(TEXT . POS)`, so the final undo list has
/// the deletion first followed by its marker adjustments.
pub fn undo_list_record_marker_adjustment(undo_list: &mut Value, marker: Value, adjustment: i64) {
    if undo_list_is_disabled(undo_list) || adjustment == 0 {
        return;
    }

    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(*undo_list);
    push_scratch_gc_root(marker);
    let entry = Value::cons(marker, Value::fixnum(adjustment));
    push_scratch_gc_root(entry);
    *undo_list = Value::cons(entry, *undo_list);
    restore_scratch_gc_roots(saved);
}

/// Record the cursor position (0-indexed `pt`) as a 1-indexed integer.
/// Skips if the most recent entry is the same position.
pub fn undo_list_record_point(undo_list: &mut Value, pt: CharPos0) {
    if undo_list_is_disabled(undo_list) {
        return;
    }
    let pt1 = Value::fixnum(pt.to_lisp().as_i64());

    // Don't record consecutive identical positions.
    if undo_list.is_cons() {
        let head = undo_list.cons_car();
        if head == pt1 {
            return;
        }
    }

    prepend_undo_entry(undo_list, pt1);
}

/// Record a text-property change: `(nil PROP VAL BEG . END)`.
///
/// `prop` is the property name (symbol), `val` is the OLD value before
/// the change (so that undoing restores it), `beg` and `end` are
/// 0-indexed character positions; they are stored as 1-indexed integers.
pub fn undo_list_record_property_change(
    undo_list: &mut Value,
    prop: Value,
    val: Value,
    range: CharRange,
) {
    if undo_list_is_disabled(undo_list) || range.is_empty() {
        return;
    }
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(*undo_list);
    push_scratch_gc_root(prop);
    push_scratch_gc_root(val);
    let beg1 = Value::fixnum(range.start().to_lisp().as_i64());
    let end1 = Value::fixnum(range.end().to_lisp().as_i64());
    // Build (nil PROP VAL BEG . END)
    let inner = Value::cons(beg1, end1);
    push_scratch_gc_root(inner);
    let inner = Value::cons(val, inner);
    push_scratch_gc_root(inner);
    let inner = Value::cons(prop, inner);
    push_scratch_gc_root(inner);
    let entry = Value::cons(Value::NIL, inner);
    push_scratch_gc_root(entry);
    *undo_list = Value::cons(entry, *undo_list);
    restore_scratch_gc_roots(saved);
}

/// Record the first-change sentinel `(t . 0)`.
pub fn undo_list_record_first_change(undo_list: &mut Value) {
    if undo_list_is_disabled(undo_list) {
        return;
    }
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(*undo_list);
    let entry = Value::cons(Value::T, Value::fixnum(0));
    push_scratch_gc_root(entry);
    *undo_list = Value::cons(entry, *undo_list);
    restore_scratch_gc_roots(saved);
}

/// Return true if LIST contains a GNU first-change sentinel `(t . MODTIME)`.
pub fn undo_list_contains_first_change(undo_list: &Value) -> bool {
    let mut cursor = *undo_list;
    while cursor.is_cons() {
        let entry = cursor.cons_car();
        if entry.is_cons() && entry.cons_car().is_t() {
            return true;
        }
        cursor = cursor.cons_cdr();
    }
    false
}

/// Insert an undo boundary (`nil`).  Skips if the list is empty/nil or
/// already starts with a nil boundary.
pub fn undo_list_boundary(undo_list: &mut Value) {
    if undo_list_is_disabled(undo_list) {
        return;
    }
    // Don't add boundary to empty list or if head is already nil.
    if undo_list.is_nil() {
        return;
    }
    if undo_list.is_cons() && undo_list.cons_car().is_nil() {
        return;
    }
    prepend_undo_entry(undo_list, Value::NIL);
}

/// Pop one undo group from the front of the list.
///
/// Skips leading nil boundaries, then collects entries until the next
/// nil boundary (or end of list).  Returns the collected entries in
/// the order they were popped (most recent first).
///
/// Mutates `undo_list` in place to remove the consumed entries.
pub fn undo_list_pop_group(undo_list: &mut Value) -> Vec<Value> {
    // Skip leading boundaries.
    while undo_list.is_cons() && undo_list.cons_car().is_nil() {
        *undo_list = undo_list.cons_cdr();
    }

    let mut group = Vec::new();
    while undo_list.is_cons() {
        let head = undo_list.cons_car();
        if head.is_nil() {
            // Hit the next boundary — stop.
            break;
        }
        group.push(head);
        *undo_list = undo_list.cons_cdr();
    }
    group
}

/// Check whether the undo list is non-empty (has actual records, not
/// just nil).
pub fn undo_list_is_empty(undo_list: &Value) -> bool {
    undo_list.is_nil()
}

/// Check whether the undo list contains at least one nil boundary.
pub fn undo_list_contains_boundary(undo_list: &Value) -> bool {
    let mut cursor = *undo_list;
    while cursor.is_cons() {
        if cursor.cons_car().is_nil() {
            return true;
        }
        cursor = cursor.cons_cdr();
    }
    false
}

/// Check whether the most recent entry is a nil boundary.
pub fn undo_list_has_trailing_boundary(undo_list: &Value) -> bool {
    undo_list.is_cons() && undo_list.cons_car().is_nil()
}

/// Estimate the byte size of one undo entry for truncation purposes.
/// Each cons cell counts as 16 bytes; strings count their byte length.
fn undo_entry_size(entry: &Value) -> usize {
    match entry.kind() {
        ValueKind::Nil => 0,
        ValueKind::Fixnum(_) => 8,
        ValueKind::String => entry.as_lisp_string().map(|s| s.sbytes()).unwrap_or(8),
        _ if entry.is_cons() => {
            let car = entry.cons_car();
            let cdr = entry.cons_cdr();
            let car_size = match car.kind() {
                ValueKind::String => car.as_lisp_string().map(|s| s.sbytes()).unwrap_or(8),
                _ => 8,
            };
            let cdr_size = match cdr.kind() {
                ValueKind::String => cdr.as_lisp_string().map(|s| s.sbytes()).unwrap_or(8),
                _ => 8,
            };
            16 + car_size + cdr_size
        }
        _ => 8,
    }
}

/// Truncate an undo list to stay within size limits.
///
/// Walks the list counting approximate byte size.  After exceeding
/// `undo_limit`, looks for the next nil boundary to truncate at.
/// After exceeding `undo_strong_limit`, truncates immediately.
///
/// Returns the truncated list.
pub fn truncate_undo_list(undo_list: Value, undo_limit: usize, undo_strong_limit: usize) -> Value {
    if undo_list_is_disabled(&undo_list) || undo_list.is_nil() {
        return undo_list;
    }

    let mut total_size: usize = 0;
    let mut past_limit = false;
    let mut scan = undo_list;

    while scan.is_cons() {
        let entry = scan.cons_car();
        total_size += undo_entry_size(&entry) + 16; // 16 for the cons cell itself

        if total_size > undo_strong_limit {
            // Immediate truncation: cut here.
            scan.set_cdr(Value::NIL);
            return undo_list;
        }

        if total_size > undo_limit {
            past_limit = true;
        }

        if past_limit && entry.is_nil() {
            // Found a boundary past the limit — truncate after this boundary.
            scan.set_cdr(Value::NIL);
            return undo_list;
        }

        scan = scan.cons_cdr();
    }

    // Never exceeded any limit — return as-is.
    undo_list
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[path = "undo_test.rs"]
mod tests;
