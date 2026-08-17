//! Undo system -- buffer undo/redo functionality.
//!
//! Provides Emacs-compatible undo functionality:
//! - `undo-boundary` -- insert an undo boundary marker
//! - `primitive-undo` -- undo entries from an undo list
//! - `undo` -- undo the last change in the current buffer

use super::error::{EvalResult, Flow, signal};
use super::value::*;
use crate::buffer::buffer::UndoBoundaryOutcome;
use crate::buffer::{Buffer, CharPos0, CharRange, EmacsBytePos, EmacsByteRange, LispCharPos1};
use crate::emacs_core::error::LispCondition;
use crate::emacs_core::error::{expect_args, expect_max_args, expect_min_args};
use strum::{EnumString, IntoStaticStr};

// ---------------------------------------------------------------------------
// Bootstrap variables
// ---------------------------------------------------------------------------

pub fn register_bootstrap_vars(obarray: &mut super::symbol::Obarray) {
    // GNU `syms_of_undo' (src/undo.c:437-457).
    obarray.define_int_variable("undo-limit", 160000);
    obarray.define_int_variable("undo-strong-limit", 240000);

    // `undo-outer-limit' (src/undo.c:459-474) defaults to 24000000, but
    // `--batch' replaces it with nil before anything runs
    // (src/emacs.c:1700-1707).  A bare Context is a batch evaluator, so nil is
    // the default here and the binary raises it for interactive sessions.
    obarray.set_symbol_value("undo-outer-limit", Value::NIL);
    obarray.make_special("undo-outer-limit");
    // `undo-outer-limit-function' (src/undo.c:476-485); lisp/simple.el sets it
    // to `undo-outer-limit-truncate' once that file is loaded.
    obarray.set_symbol_value("undo-outer-limit-function", Value::NIL);
    obarray.make_special("undo-outer-limit-function");
}

// ---------------------------------------------------------------------------
// Truncation at garbage collection
// ---------------------------------------------------------------------------

/// Read the truncation limits out of the bindings visible in the current
/// buffer, which is what GNU's `set_buffer_internal (b)` at the top of
/// `truncate_undo_list' (src/undo.c:296-306) arranges for.
impl crate::buffer::UndoLimitBindings for super::eval::Context {
    fn undo_limit(&self) -> Value {
        self.undo_truncation_variable("undo-limit")
    }

    fn undo_strong_limit(&self) -> Value {
        self.undo_truncation_variable("undo-strong-limit")
    }

    fn undo_outer_limit(&self) -> Value {
        self.undo_truncation_variable("undo-outer-limit")
    }

    fn undo_outer_limit_function(&self) -> Value {
        self.undo_truncation_variable("undo-outer-limit-function")
    }
}

thread_local! {
    /// Re-entrancy latch for [`compact_buffers_for_gc`].
    ///
    /// `undo-outer-limit-function' is Lisp and may call `garbage-collect'
    /// itself.  GNU cannot recurse here because its `garbage_collect' bails
    /// out on `garbage_collection_inhibited' (src/alloc.c:5789-5790) and
    /// `truncate_undo_list' holds that inhibition across the call
    /// (src/undo.c:296-298); Neomacs' explicit `garbage-collect' collects
    /// regardless, so the latch is what keeps one compaction pass from
    /// starting another.
    static COMPACTION_IN_PROGRESS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

struct CompactionLatch;

impl CompactionLatch {
    /// `None` when a compaction pass is already running on this thread.
    fn acquire() -> Option<Self> {
        COMPACTION_IN_PROGRESS.with(|flag| {
            if flag.get() {
                None
            } else {
                flag.set(true);
                Some(Self)
            }
        })
    }
}

impl Drop for CompactionLatch {
    fn drop(&mut self) {
        COMPACTION_IN_PROGRESS.with(|flag| flag.set(false));
    }
}

/// Shorten every live buffer's undo list, the way GNU's collector does.
///
/// GNU runs this as the first thing `garbage_collect' does, before any
/// marking: "Don't keep undo information around forever.  Do this early on, so
/// it is no problem if the user quits." (src/alloc.c:5796-5800).  The walk goes
/// through `compact_buffer' (src/buffer.c:1854-1885), which skips dead
/// buffers, indirect buffers, and buffers unchanged since the last compaction,
/// and refuses to hand a `t' undo list to `truncate_undo_list' because that
/// would turn undo back on.
///
/// Errors from `undo-outer-limit-function' are swallowed, as they are for the
/// finalizers and `post-gc-hook' this collector already runs: a collection
/// cannot propagate a signal to whatever the mutator was doing.
pub(crate) fn compact_buffers_for_gc(ctx: &mut super::eval::Context) {
    let Some(latch) = CompactionLatch::acquire() else {
        return;
    };
    let restore_to = ctx.buffers.current_buffer_id();
    for id in ctx.buffers.buffer_list() {
        compact_one_buffer_for_gc(ctx, id);
    }
    if let Some(id) = restore_to {
        ctx.restore_current_buffer_if_live(id);
    }
    drop(latch);
}

/// GNU `compact_buffer' (src/buffer.c:1854-1885), minus the gap shrinking that
/// has no counterpart in Neomacs' buffer text.
fn compact_one_buffer_for_gc(ctx: &mut super::eval::Context, id: crate::buffer::BufferId) {
    use crate::buffer::{UndoLimits, UndoRecording};

    let Some(buffer) = ctx.buffers.get(id) else {
        return; // killed while we walked the list
    };
    if buffer.base_buffer.is_some() {
        return; // indirect buffers share their base's text
    }
    let modified_tick = buffer.modified_tick();
    if buffer.undo_state.compacted_modified_tick() == modified_tick {
        return; // unchanged since the last compaction
    }
    // GNU stamps the buffer whatever the truncation decides, including for the
    // `t' and early-return paths (src/buffer.c:1884).
    buffer.undo_state.set_compacted_modified_tick(modified_tick);

    let undo_list = buffer.get_undo_list();
    if UndoRecording::of(&undo_list) == UndoRecording::Disabled {
        return;
    }

    // Everything from here reads the buffer's own variable bindings, so the
    // buffer has to be current -- the reason GNU calls `set_buffer_internal'.
    if ctx.set_current_buffer_unrecorded(id).is_err() {
        return;
    }
    let Some(mut limits) = UndoLimits::read(ctx) else {
        return;
    };

    let first_group_bytes = crate::buffer::undo_first_group_bytes(undo_list);
    if let Some(function) = limits.outer_limit_function_for(first_group_bytes) {
        let saved_roots = super::eval::save_scratch_gc_roots();
        super::eval::push_scratch_gc_root(function);
        let handled = ctx.with_gc_inhibited(|eval| {
            eval.funcall_general(function, vec![Value::fixnum(first_group_bytes)])
        });
        super::eval::restore_scratch_gc_roots(saved_roots);
        if handled.is_ok_and(|answer| !answer.is_nil()) {
            // "The function is responsible for making any desired changes in
            // buffer-undo-list." (src/undo.c:362-368)
            return;
        }
        // GNU reads `undo_limit' and `undo_strong_limit' during the walk that
        // follows (src/undo.c:386-389), so a function that lowers them and
        // answers nil has its new values applied.  They are C globals, so it
        // reads them from whatever buffer the function left current -- both
        // halves measured under GNU 31.0.90: a function that lowers this
        // buffer's limits and stays truncates 21 entries to 2; the same
        // function ending in `(set-buffer "H")' leaves all 21, because H's
        // values are what the globals then hold.
        let Some(reread) = UndoLimits::read(ctx) else {
            return;
        };
        limits = reread;
    }

    // Re-read the list: the function may have replaced it before answering nil.
    let Some(undo_list) = ctx.buffers.get(id).map(|buffer| buffer.get_undo_list()) else {
        return; // the function killed the buffer
    };
    if UndoRecording::of(&undo_list) == UndoRecording::Disabled {
        return;
    }
    let truncated = crate::buffer::truncate_undo_list(undo_list, &limits);
    if let Some(buffer) = ctx.buffers.get_mut(id) {
        buffer.set_undo_list(truncated);
    }
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn expect_int(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integerp"), *value],
        )),
    }
}

fn expect_list_like(value: &Value) -> Result<(), Flow> {
    if value.is_nil() || value.is_cons() {
        Ok(())
    } else {
        Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("listp"), *value],
        ))
    }
}

fn char_pos1_to_char0(pos1: LispCharPos1) -> CharPos0 {
    pos1.to_char_pos()
}

fn char_pos1_to_byte_clamped(buf: &Buffer, pos1: LispCharPos1) -> EmacsBytePos {
    buf.char_pos_to_emacs_byte_pos_clamped(char_pos1_to_char0(pos1))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UndoLispPosition(i64);

impl UndoLispPosition {
    fn from_entry(raw: i64) -> Self {
        Self(raw)
    }

    fn absolute_from_signed_deletion(raw: i64) -> Self {
        Self(raw.abs())
    }

    fn to_lisp_char_pos(self) -> LispCharPos1 {
        LispCharPos1::new(self.0)
    }
}

fn accessible_lisp_char_bounds(buf: &Buffer) -> (LispCharPos1, LispCharPos1) {
    let accessible = buf.accessible_char_region();
    (accessible.start().to_lisp(), accessible.end().to_lisp())
}

fn lisp_char_position_is_visible(buf: &Buffer, pos: LispCharPos1) -> bool {
    let (point_min, point_max) = accessible_lisp_char_bounds(buf);
    point_min <= pos && pos <= point_max
}

fn ensure_undo_lisp_range_is_visible(
    buf: &Buffer,
    beg: LispCharPos1,
    end: LispCharPos1,
) -> Result<(), Flow> {
    let (point_min, point_max) = accessible_lisp_char_bounds(buf);
    if beg < point_min || end > point_max {
        return Err(signal(
            "error",
            vec![Value::string(
                "Changes to be undone are outside visible portion of buffer",
            )],
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum UndoEntryHead {
    Apply,
}

impl UndoEntryHead {
    fn from_lisp_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

// ---------------------------------------------------------------------------
// Pure builtins
// ---------------------------------------------------------------------------

/// (undo-boundary) -> nil
///
/// Context-dependent variant used during normal execution: inserts an
/// undo boundary into the current buffer's undo list.
pub(crate) fn builtin_undo_boundary(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("undo-boundary", &args, 0)?;
    let Some(current_id) = ctx.buffers.current_buffer_id() else {
        return Err(signal("error", vec![Value::string("No current buffer")]));
    };
    // GNU sets `undo-auto--last-boundary-cause' to `explicit' inside
    // `Fundo_boundary' (src/undo.c:277), after the early return for an
    // undo-disabled buffer, so a buffer that records nothing does not claim a
    // boundary either.  The boundary itself runs below the obarray, so the
    // assignment lives here, gated on the outcome it reports.
    if ctx.buffers.add_undo_boundary(current_id) == Some(UndoBoundaryOutcome::Recorded) {
        set_last_boundary_cause_explicit(ctx)?;
    }
    Ok(Value::NIL)
}

/// GNU `Fset (Qundo_auto__last_boundary_cause, Qexplicit)` (src/undo.c:277).
///
/// This goes through our `set' builtin rather than a direct obarray write
/// because GNU goes through `Fset': the variable is a `defvar-local' in
/// lisp/simple.el, so wherever a buffer-local binding exists the assignment
/// must land on THAT and not on the default.  Writing the default instead is
/// invisible until something has made the variable local, which is the ordinary
/// case -- `undo-auto--undoably-changed-buffers' processing localizes it.
/// Delegating also picks up alias resolution, the constant check and variable
/// watchers, all of which GNU gets for free from the same call.
pub(crate) fn set_last_boundary_cause_explicit(
    ctx: &mut super::eval::Context,
) -> Result<(), Flow> {
    super::builtins::symbols::builtin_set_2(
        ctx,
        Value::symbol("undo-auto--last-boundary-cause"),
        Value::symbol("explicit"),
    )?;
    Ok(())
}

/// (primitive-undo COUNT LIST) -> remainder of LIST
///
/// Undo COUNT undo-groups from LIST, applying each entry to the current
/// buffer.  Returns the unconsumed tail of LIST.
///
/// Matches GNU Emacs's `primitive-undo` (simple.el:3642-3777).
///
/// Entry types handled:
/// - Integer POS: `(goto-char POS)`
/// - `(BEG . END)` both ints: delete the region (undo an insertion)
/// - `(TEXT . POS)` string+int: insert TEXT at |POS| (undo a deletion)
/// - `(t . MODTIME)`: restore buffer-modified state
/// - `(nil PROP VAL BEG . END)`: restore text property
/// - `(MARKER . OFFSET)`: adjust marker (skipped)
/// - `(apply FUN . ARGS)`: call FUN with ARGS
pub(crate) fn builtin_primitive_undo(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("primitive-undo", &args, 2)?;

    let count = expect_int(&args[0])?;
    expect_list_like(&args[1])?;

    if count <= 0 {
        return Ok(args[1]);
    }

    let Some(buf_id) = ctx.buffers.current_buffer_id() else {
        return Err(signal("error", vec![Value::string("No current buffer")]));
    };

    // Save and set inhibit-read-only to t during undo.
    let saved_inhibit = ctx.obarray.symbol_value("inhibit-read-only").copied();
    ctx.obarray.set_symbol_value("inhibit-read-only", Value::T);

    let result = primitive_undo_inner(ctx, buf_id, count, args[1]);

    // Restore inhibit-read-only.
    match saved_inhibit {
        Some(v) => ctx.obarray.set_symbol_value("inhibit-read-only", v),
        None => ctx
            .obarray
            .set_symbol_value("inhibit-read-only", Value::NIL),
    }

    result
}

/// Inner loop: process COUNT undo groups from LIST, return unconsumed tail.
fn primitive_undo_inner(
    ctx: &mut super::eval::Context,
    buf_id: crate::buffer::BufferId,
    count: i64,
    mut list: Value,
) -> EvalResult {
    let mut groups_done = 0i64;

    while groups_done < count && list.is_cons() {
        // Skip leading nil boundaries.
        while list.is_cons() && list.cons_car().is_nil() {
            list = list.cons_cdr();
        }

        if !list.is_cons() {
            break;
        }

        // Process one undo group (entries until next nil or end).
        while list.is_cons() {
            let entry = list.cons_car();
            list = list.cons_cdr();

            if entry.is_nil() {
                // Hit boundary — end of this group.
                break;
            }

            // Integer POS: goto-char
            if let Some(pos1) = entry.as_fixnum() {
                if let Some(buf) = ctx.buffers.get(buf_id) {
                    let byte = char_pos1_to_byte_clamped(
                        buf,
                        UndoLispPosition::from_entry(pos1).to_lisp_char_pos(),
                    );
                    ctx.buffers.goto_buffer_emacs_byte_pos(buf_id, byte);
                }
                continue;
            }

            if !entry.is_cons() {
                // Unknown non-cons, non-int entry — skip.
                continue;
            }

            let car = entry.cons_car();
            let cdr = entry.cons_cdr();

            match (car.kind(), cdr.kind()) {
                // (BEG . END) both integers — undo an insertion by deleting.
                (ValueKind::Fixnum(beg1), ValueKind::Fixnum(end1)) => {
                    let beg_pos = UndoLispPosition::from_entry(beg1).to_lisp_char_pos();
                    let end_pos = UndoLispPosition::from_entry(end1).to_lisp_char_pos();
                    let delete_range = if let Some(buf) = ctx.buffers.get(buf_id) {
                        ensure_undo_lisp_range_is_visible(buf, beg_pos, end_pos)?;
                        Some(buf.edit_range_for_char_range(CharRange::new(
                            char_pos1_to_char0(beg_pos),
                            char_pos1_to_char0(end_pos),
                        )))
                    } else {
                        None
                    };
                    if let Some(range) = delete_range {
                        let _ = ctx.buffers.delete_buffer_measured_region(buf_id, range);
                    }
                }
                // (TEXT . POS) string + int — undo a deletion by re-inserting.
                (ValueKind::String, ValueKind::Fixnum(pos1)) => {
                    let apos1 =
                        UndoLispPosition::absolute_from_signed_deletion(pos1).to_lisp_char_pos();
                    if let Some(buf) = ctx.buffers.get(buf_id)
                        && !lisp_char_position_is_visible(buf, apos1)
                    {
                        return Err(signal(
                            "error",
                            vec![Value::string(
                                "Changes to be undone are outside visible portion of buffer",
                            )],
                        ));
                    }

                    let mut valid_marker_adjustments = Vec::new();
                    while list.is_cons() {
                        let marker_adj = list.cons_car();
                        if !marker_adj.is_cons() {
                            break;
                        }
                        let marker = marker_adj.cons_car();
                        let offset = marker_adj.cons_cdr();
                        let Some(offset) = offset.as_fixnum() else {
                            break;
                        };
                        if !marker.is_marker() {
                            break;
                        }

                        list = list.cons_cdr();
                        let marker_in_current_buffer =
                            super::marker::marker_logical_fields(&marker)
                                .is_some_and(|(buffer, _, _)| buffer == Some(buf_id));
                        let marker_at_undo_position =
                            super::marker::marker_position_as_int_with_buffers(
                                &ctx.buffers,
                                &marker,
                            )
                            .is_ok_and(|pos| pos == apos1.as_i64());
                        if marker_in_current_buffer && marker_at_undo_position {
                            valid_marker_adjustments.push((marker, offset));
                        }
                    }

                    if let Some(buf) = ctx.buffers.get(buf_id) {
                        let clamped = char_pos1_to_byte_clamped(buf, apos1);
                        ctx.buffers.goto_buffer_emacs_byte_pos(buf_id, clamped);
                        super::builtins::insert_string_value_in_current_buffer(
                            &ctx.obarray,
                            &[],
                            &mut ctx.buffers,
                            car,
                            super::builtins::InsertPieceMarkerPlacement::AfterMarkers,
                            super::builtins::InsertPiecePropertyMode::SourceOnly,
                        )?;
                        // If POS was negative, point should be at end of
                        // inserted text (which insert_into_buffer already does).
                        // If positive, move point back to start of insertion.
                        if pos1 > 0 {
                            ctx.buffers.goto_buffer_emacs_byte_pos(buf_id, clamped);
                        }
                    }

                    for (marker, offset) in valid_marker_adjustments {
                        let marker_still_live = super::marker::marker_logical_fields(&marker)
                            .is_some_and(|(buffer, _, _)| buffer.is_some());
                        if !marker_still_live {
                            continue;
                        }
                        let Ok(pos) = super::marker::marker_position_as_int_with_buffers(
                            &ctx.buffers,
                            &marker,
                        ) else {
                            continue;
                        };
                        super::marker::builtin_set_marker_in_buffers(
                            &mut ctx.buffers,
                            vec![
                                marker,
                                Value::fixnum(pos - offset),
                                Value::make_buffer(buf_id),
                            ],
                        )?;
                    }
                }
                // (t . MODTIME) — restore buffer-modified state.
                (ValueKind::T, ValueKind::Fixnum(modtime)) => {
                    if modtime == 0 {
                        // modtime 0 means mark buffer as unmodified.
                        let _ = ctx.buffers.set_buffer_modified_flag(buf_id, false);
                    }
                    // Non-zero modtimes would compare against file modtime;
                    // for now we just skip those.
                }
                // (nil . LEN) — undo a yank: delete LEN chars before point.
                (ValueKind::Nil, ValueKind::Fixnum(len1)) => {
                    let len = len1.max(0) as usize;
                    let delete_range = if let Some(buf) = ctx.buffers.get(buf_id) {
                        let point = buf.point_char_pos().get();
                        if point >= len {
                            let del_start_char = point - len;
                            let del_end_char = point;
                            let accessible = buf.accessible_char_region();
                            let del_start_char = CharPos0::new(del_start_char);
                            let del_end_char = CharPos0::new(del_end_char);
                            if accessible.contains_boundary(del_start_char)
                                && accessible.contains_boundary(del_end_char)
                            {
                                Some(buf.edit_range_for_char_range(CharRange::new(
                                    del_start_char,
                                    del_end_char,
                                )))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(range) = delete_range {
                        ctx.buffers
                            .goto_buffer_emacs_byte_pos(buf_id, range.byte_start());
                        let _ = ctx.buffers.delete_buffer_measured_region(buf_id, range);
                    }
                }
                // (nil PROP VAL BEG . END) — restore text property.
                (ValueKind::Nil, _) => {
                    // cdr is (PROP VAL BEG . END)
                    if cdr.is_cons() {
                        let prop = cdr.cons_car();
                        let rest1 = cdr.cons_cdr();
                        if rest1.is_cons() {
                            let val = rest1.cons_car();
                            let rest2 = rest1.cons_cdr();
                            if rest2.is_cons() {
                                let beg_val = rest2.cons_car();
                                let end_val = rest2.cons_cdr();
                                if let (Some(b), Some(e)) =
                                    (beg_val.as_fixnum(), end_val.as_fixnum())
                                {
                                    let beg_pos =
                                        UndoLispPosition::from_entry(b).to_lisp_char_pos();
                                    let end_pos =
                                        UndoLispPosition::from_entry(e).to_lisp_char_pos();
                                    if let Some(buf) = ctx.buffers.get(buf_id) {
                                        ensure_undo_lisp_range_is_visible(buf, beg_pos, end_pos)?;
                                        let accessible_start =
                                            buf.accessible_emacs_byte_region().start();
                                        let byte_beg = if b > 0 {
                                            char_pos1_to_byte_clamped(buf, beg_pos)
                                        } else {
                                            accessible_start
                                        };
                                        let byte_end = if e > 0 {
                                            char_pos1_to_byte_clamped(buf, end_pos)
                                        } else {
                                            accessible_start
                                        };
                                        let _ = ctx
                                            .buffers
                                            .put_buffer_text_property_in_emacs_byte_range(
                                                buf_id,
                                                EmacsByteRange::new(byte_beg, byte_end),
                                                prop,
                                                val,
                                            );
                                    }
                                }
                            }
                        }
                    }
                }
                // (apply FUN . ARGS) or
                // (apply DELTA START END FUN . ARGS) — call FUN with ARGS.
                // Mirrors GNU lisp/simple.el:3702-3722.
                _ if UndoEntryHead::from_lisp_value(&car) == Some(UndoEntryHead::Apply) => {
                    if cdr.is_cons() {
                        let fun_args_head = cdr.cons_car();
                        let (fun, fargs) = if fun_args_head.as_fixnum().is_some() {
                            // Long format: (apply DELTA START END FUN . ARGS).
                            // Validate that START..END is fully inside the
                            // visible portion of the buffer.
                            let rest1 = cdr.cons_cdr(); // (START END FUN . ARGS)
                            let mut start_v = Value::NIL;
                            let mut rest2 = Value::NIL;
                            if rest1.is_cons() {
                                start_v = rest1.cons_car();
                                rest2 = rest1.cons_cdr(); // (END FUN . ARGS)
                            }
                            let mut end_v = Value::NIL;
                            let mut rest3 = Value::NIL;
                            if rest2.is_cons() {
                                end_v = rest2.cons_car();
                                rest3 = rest2.cons_cdr(); // (FUN . ARGS)
                            }
                            if let (Some(start1), Some(end1)) =
                                (start_v.as_fixnum(), end_v.as_fixnum())
                            {
                                let start_pos =
                                    UndoLispPosition::from_entry(start1).to_lisp_char_pos();
                                let end_pos = UndoLispPosition::from_entry(end1).to_lisp_char_pos();
                                if let Some(buf) = ctx.buffers.get(buf_id) {
                                    ensure_undo_lisp_range_is_visible(buf, start_pos, end_pos)?;
                                }
                            }
                            if !rest3.is_cons() {
                                continue;
                            }
                            let fun = rest3.cons_car();
                            let mut fargs = Vec::new();
                            let mut cursor = rest3.cons_cdr();
                            while cursor.is_cons() {
                                fargs.push(cursor.cons_car());
                                cursor = cursor.cons_cdr();
                            }
                            (fun, fargs)
                        } else {
                            // Short format: (apply FUN . ARGS).
                            let fun = cdr.cons_car();
                            let mut fargs = Vec::new();
                            let mut cursor = cdr.cons_cdr();
                            while cursor.is_cons() {
                                fargs.push(cursor.cons_car());
                                cursor = cursor.cons_cdr();
                            }
                            (fun, fargs)
                        };
                        // Best-effort: ignore errors from undo apply calls.
                        let _ = ctx.funcall_general(fun, fargs);
                    }
                }
                // (MARKER . OFFSET) — unexpected marker adjustment without a
                // matching (TEXT . POS) entry.  GNU warns and still applies it
                // conservatively.
                (ValueKind::Veclike(VecLikeType::Marker), ValueKind::Fixnum(offset)) => {
                    let marker_still_live = super::marker::marker_logical_fields(&car)
                        .is_some_and(|(buffer, _, _)| buffer.is_some());
                    if marker_still_live {
                        let Ok(pos) =
                            super::marker::marker_position_as_int_with_buffers(&ctx.buffers, &car)
                        else {
                            continue;
                        };
                        super::marker::builtin_set_marker_in_buffers(
                            &mut ctx.buffers,
                            vec![car, Value::fixnum(pos - offset), Value::make_buffer(buf_id)],
                        )?;
                    }
                }
                _ => {
                    // Unknown entry type — skip.
                }
            }
        }
        groups_done += 1;
    }

    Ok(list)
}

// ---------------------------------------------------------------------------
// Eval-dependent builtins
// ---------------------------------------------------------------------------

/// (undo &optional ARG) -> nil
///
/// Undo the last change in the current buffer.
/// ARG is the number of undo commands to execute (default 1).
///
/// In a full implementation, this would:
/// 1. Get the current buffer's undo list
/// 2. Apply primitive-undo to reverse the specified number of actions
/// 3. Update buffer state accordingly
///
pub(crate) fn builtin_undo(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("undo", &args, 0)?;
    expect_max_args("undo", &args, 1)?;

    // If ARG is provided, verify it's an integer
    let mut count = 1i64;
    if let Some(arg) = args.first() {
        count = expect_int(arg)?;
    }

    let Some(current_id) = eval.buffers.current_buffer_id() else {
        return Err(signal("error", vec![Value::string("No current buffer")]));
    };
    let outcome = eval
        .buffers
        .undo_buffer(current_id, count)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

    if outcome.skipped_apply {
        return Ok(Value::string("Undo"));
    }

    if !outcome.applied_any {
        let msg = if outcome.had_any_records {
            "No further undo information"
        } else {
            "No undo information in this buffer"
        };
        return Err(signal(LispCondition::UserError, vec![Value::string(msg)]));
    }

    if outcome.had_boundary {
        Ok(Value::string("Undo"))
    } else {
        Err(signal(
            LispCondition::UserError,
            vec![Value::string("No further undo information")],
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "undo_test.rs"]
mod tests;
