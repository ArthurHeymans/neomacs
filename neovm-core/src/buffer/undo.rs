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

/// The two states GNU's `buffer-undo-list` slot can be in.
///
/// The slot holds either the symbol `t` -- undo turned off -- or a list of
/// undo records, which may be nil for "on, but nothing recorded yet".  GNU
/// tests this domain by hand at every decision point (`EQ (..., Qt)` in
/// `record_insert` src/undo.c:91, `Fbuffer_enable_undo` src/buffer.c:1846,
/// `compact_buffer` src/buffer.c:1869).  Naming it once makes each caller's
/// branch exhaustive at compile time, so a new state cannot be forgotten and
/// "on with an empty history" cannot be confused with "off".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoRecording {
    /// `buffer-undo-list` is `t`: changes are not recorded.
    Disabled,
    /// `buffer-undo-list` is a list of records, possibly empty (nil).
    Enabled,
}

impl UndoRecording {
    /// Classify a `buffer-undo-list` value.
    pub fn of(undo_list: &Value) -> Self {
        if undo_list.is_t() {
            Self::Disabled
        } else {
            Self::Enabled
        }
    }
}

/// Returns `true` when `buffer-undo-list` is `t` (undo disabled).
pub fn undo_list_is_disabled(undo_list: &Value) -> bool {
    matches!(UndoRecording::of(undo_list), UndoRecording::Disabled)
}

/// True when `buffer-undo-list` currently sits at an undo boundary: it is
/// empty, or its newest entry is the `nil` boundary.
///
/// GNU `record_point` (`src/undo.c:59-61`) reads exactly this, and reads it
/// *before* `record_first_change` may cons `(t . TIME)` onto the list.  The
/// answer is therefore a fact about the list as the command found it, which
/// is why it is read once by [`crate::buffer::Buffer::undo_prepare_change`]
/// rather than re-derived by each recorder.
pub fn undo_list_at_boundary(undo_list: &Value) -> bool {
    undo_list.is_nil() || (undo_list.is_cons() && undo_list.cons_car().is_nil())
}

/// Record that text was inserted at character position `beg` with character
/// length `len`.  Positions stored in the list are 1-indexed.
///
/// The caller must already have run the GNU `record_point` prologue (see
/// [`crate::buffer::Buffer::undo_prepare_change`]); this records only the
/// insertion itself, exactly like GNU's `record_insert` body after its
/// `record_point (beg)` call.
///
/// Consecutive adjacent inserts are merged when the head entry is an
/// insert whose END equals `beg+1` (the 1-indexed start of the new
/// insert), and only then: an insert that ends where the head entry begins
/// stays its own record, exactly as in GNU `record_insert`.
pub fn undo_list_record_insert(undo_list: &mut Value, beg: CharPos0, len: CharLen) {
    // GNU `record_insert` (undo.c) returns early only for a disabled undo
    // list; a zero-length insertion still conses `(BEG . BEG)`.  That record
    // is load-bearing, because `record_insert` coalesces into the newest
    // record only when that record is an insertion ending where the new one
    // begins — a zero-length record breaks the chain between two adjacent
    // change runs.
    if undo_list_is_disabled(undo_list) {
        return;
    }

    let beg1 = beg.to_lisp().as_i64();
    let end1 = beg.add_len(len).to_lisp().as_i64();

    // GNU `record_insert` (undo.c:98-112) coalesces in exactly ONE direction:
    // into a newest record that is an insertion ENDING where this insertion
    // BEGINS.  There is deliberately no reverse rule.  `primitive-undo` replays
    // the records newest-first and each deletion reshapes the buffer the later
    // records are read against, so two insertions made back-to-front --
    // the ordinary shape when a client applies a server's edit list in reverse
    // to keep earlier positions valid -- must stay two records.  Merging them
    // would claim the untouched text between them as newly inserted and delete
    // it on undo.
    if undo_list.is_cons() {
        let head = undo_list.cons_car();
        if head.is_cons() {
            let car = head.cons_car();
            let cdr = head.cons_cdr();
            if let (Some(_prev_beg), Some(prev_end)) = (car.as_fixnum(), cdr.as_fixnum())
                && prev_end == beg1
            {
                // Merge: extend the existing insert entry.
                head.set_cdr(Value::fixnum(prev_end + len.get() as i64));
                return;
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

/// Record a deletion.  `beg` is the 0-indexed character position, `text` is
/// the deleted string, `pt` is the 0-indexed cursor character position at
/// the time of deletion.
///
/// The stored position is 1-indexed and negative when `pt` was at the
/// END of the deleted region (i.e. `pt == beg + SCHARS (text)`).
///
/// The caller must already have run the GNU `record_point` prologue (see
/// [`crate::buffer::Buffer::undo_prepare_change`]), which is also what keeps
/// the point entry ahead of any `(MARKER . ADJUSTMENT)` entries (GNU bug
/// 16818 ordering).
pub fn undo_list_record_delete(
    undo_list: &mut Value,
    beg: CharPos0,
    text: LispString,
    pt: CharPos0,
) {
    // GNU `record_delete` (undo.c) returns early only for a disabled undo
    // list; it never tests the string's length, so a zero-length deletion
    // still conses `("" . POS)`.  See [`undo_list_record_insert`] for why an
    // empty record matters.
    if undo_list_is_disabled(undo_list) {
        return;
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

/// Record the first-change sentinel `(t . VISITED-FILE-MODTIME)`.
///
/// GNU `record_first_change` (`src/undo.c:209-223`) records the buffer's
/// visited-file modtime, not a placeholder:
///
/// ```c
/// bset_undo_list (current_buffer,
///                 Fcons (Fcons (Qt, buffer_visited_file_modtime (base_buffer)),
///                        BVAR (current_buffer, undo_list)));
/// ```
///
/// The datum is what makes the entry *mean* anything: `primitive-undo`'s
/// `(t . TIME)` arm (`lisp/simple.el:3669-3688`) clears the modified flag only
/// when `(time-equal-p time (visited-file-modtime))`, so that undoing back to
/// a save that has since been superseded on disk does NOT claim the buffer is
/// unmodified.  Recording a constant made every such comparison fail for a
/// file-visiting buffer, and `undo` back to the saved text left
/// `buffer-modified-p` t where GNU reports nil.
///
/// `visited_file_modtime` must be the value GNU's
/// `buffer_visited_file_modtime` would return for the buffer owning the undo
/// list -- see [`crate::buffer::Buffer::visited_file_modtime_value`].
pub fn undo_list_record_first_change(undo_list: &mut Value, visited_file_modtime: Value) {
    if undo_list_is_disabled(undo_list) {
        return;
    }
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(*undo_list);
    push_scratch_gc_root(visited_file_modtime);
    let entry = Value::cons(Value::T, visited_file_modtime);
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
