//! Indentation builtins for the Elisp interpreter.
//!
//! Implements stub versions of Emacs indentation primitives:
//! - `current-indentation`, `indent-to`, `current-column`, `move-to-column`
//! - `indent-line-to`, `indent-rigidly`, `newline-and-indent`,
//!   `tab-to-tab-stop`, `delete-indentation`
//!
//! Variables: `tab-width`, `indent-tabs-mode`, `standard-indent`, `tab-stop-list`

use super::buffer::{extract_cons_fixnums, point_char_pos};
use super::error::{EvalResult, Flow, signal};
use super::symbol::Obarray;
use super::value::*;
use super::xdisp::line_number_digit_width;
use crate::buffer::{
    Buffer, CharLen, CharPos0, EmacsByteLen, EmacsBytePos, EmacsByteRange, TextExtent,
};
use crate::buffer::{BufferId, LispCharPos1};
use crate::emacs_core::error::LispCondition;
use crate::emacs_core::error::{
    expect_args, expect_args_range, expect_fixnum, expect_max_args, expect_min_args,
};
use crate::emacs_core::value::ValueKind;
use crate::heap_types::LispString;
use super::xdisp::LineWrap;
use crate::window::{DisplayRowSnapshot, Window, WindowDisplaySnapshot, WindowId};
use std::cell::Cell;
use std::collections::VecDeque;

fn next_visible_line_start(
    eval: &mut super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    pos: EmacsBytePos,
    screen_width: usize,
    wrap: LineWrap,
) -> Result<Option<ScreenLineStep>, Flow> {
    let Some(step) = next_screen_line_start_from(eval, buffer_id, pos, screen_width, wrap)?
    else {
        return Ok(None);
    };
    Ok(Some(step))
}

#[derive(Clone, Copy, Debug)]
struct ScreenLineStep {
    next: EmacsBytePos,
    counts_line: bool,
}

fn previous_logical_line_start(
    buf: &crate::buffer::buffer::Buffer,
    pos: EmacsBytePos,
) -> Option<EmacsBytePos> {
    let begv = buf.accessible_emacs_byte_region().start();
    let line_start = line_start_at_or_before(buf, pos);
    if line_start <= begv {
        return None;
    }
    Some(line_start_at_or_before(
        buf,
        line_start.saturating_sub_len(EmacsByteLen::new(1)),
    ))
}

/// Move backward by several displayed rows without restarting at `point-min`
/// for every row.
///
/// GNU's `move_it_by_lines` first backs up across approximately the requested
/// number of logical lines, then scans forward once to correct for wrapping
/// and display properties.  Do the same here.  If invisible logical lines
/// leave too few displayed rows, grow the backward search geometrically; the
/// total forward work remains linear in the traversed buffer region.
fn previous_screen_line_target(
    eval: &mut super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    pos: EmacsBytePos,
    screen_width: usize,
    wrap: LineWrap,
    count: usize,
) -> Result<(EmacsBytePos, i64), Flow> {
    if count == 0 {
        return Ok((pos, 0));
    }
    let point_min = match eval.buffers.get(buffer_id) {
        Some(buf) => buf.accessible_emacs_byte_region().start(),
        None => return Ok((pos, 0)),
    };
    let current = current_screen_line_start_with_truncation(
        eval,
        buffer_id,
        pos,
        screen_width,
        wrap,
    )?
    .unwrap_or(point_min);
    if current <= point_min {
        return Ok((current, 0));
    }

    let mut anchor = eval
        .buffers
        .get(buffer_id)
        .map(|buf| line_start_at_or_before(buf, current))
        .unwrap_or(point_min);
    let mut logical_lines_to_back = count.max(1);

    loop {
        if let Some(buf) = eval.buffers.get(buffer_id) {
            for _ in 0..logical_lines_to_back {
                let Some(previous) = previous_logical_line_start(buf, anchor) else {
                    anchor = point_min;
                    break;
                };
                anchor = previous;
            }
        }

        let anchor_is_invisible = crate::emacs_core::xdisp::zero_width_invisible_run_end_byte(
            eval,
            buffer_id,
            anchor.get(),
        )?
        .is_some_and(|next_visible| next_visible > anchor.get());
        if anchor_is_invisible {
            if anchor <= point_min {
                return Ok((current, 0));
            }
            logical_lines_to_back = logical_lines_to_back.saturating_mul(2);
            continue;
        }

        let mut recent_rows = VecDeque::with_capacity(count.saturating_add(1));
        recent_rows.push_back(anchor);
        let mut cursor = anchor;
        while cursor < current {
            let Some(step) =
                next_screen_line_start_from(eval, buffer_id, cursor, screen_width, wrap)?
            else {
                break;
            };
            if step.next <= cursor || step.next > current {
                break;
            }
            cursor = step.next;
            if step.counts_line {
                recent_rows.push_back(cursor);
                if recent_rows.len() > count.saturating_add(1) {
                    recent_rows.pop_front();
                }
            }
        }

        if recent_rows.back().copied() == Some(current) && recent_rows.len() > count {
            return Ok((recent_rows[recent_rows.len() - count - 1], -(count as i64)));
        }
        if anchor <= point_min {
            let moved = recent_rows.len().saturating_sub(1);
            return Ok((
                recent_rows.front().copied().unwrap_or(point_min),
                -(moved as i64),
            ));
        }
        logical_lines_to_back = logical_lines_to_back.saturating_mul(2);
    }
}

fn vertical_motion_screen_width(eval: &mut super::eval::Context, window: Option<Value>) -> usize {
    let window_arg = window.unwrap_or(Value::NIL);
    super::window_cmds::builtin_window_body_width(eval, vec![window_arg, Value::NIL])
        .ok()
        .and_then(|v| v.as_fixnum())
        .filter(|n| *n > 1)
        .map(|n| (n as usize).saturating_sub(1))
        .unwrap_or(79)
        .max(1)
}

fn line_start_at_or_before(buf: &crate::buffer::buffer::Buffer, pos: EmacsBytePos) -> EmacsBytePos {
    let accessible = buf.accessible_emacs_byte_region();
    let begv = accessible.start();
    let mut bol = accessible.clamp(pos);
    while bol > begv
        && buf.emacs_byte_at_pos(bol.saturating_sub_len(EmacsByteLen::new(1))) != Some(b'\n')
    {
        bol = bol.saturating_sub_len(EmacsByteLen::new(1));
    }
    bol
}

fn current_screen_line_start_with_truncation(
    eval: &mut super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    pos: EmacsBytePos,
    screen_width: usize,
    wrap: LineWrap,
) -> Result<Option<EmacsBytePos>, Flow> {
    let Some(buf) = eval.buffers.get(buffer_id) else {
        return Ok(None);
    };
    let accessible = buf.accessible_emacs_byte_region();
    let target = accessible.clamp(pos);
    let mut current = line_start_at_or_before(buf, target);

    // GNU `vmotion` backs across invisible newline characters before it
    // computes the current display row. A folded group of logical lines can
    // therefore share one visual row with the text on either side of it. A
    // scan anchored only at the target's logical BOL loses that relationship
    // and reports the first line after a fold as a new display row.
    while current > accessible.start() {
        let preceding = current.saturating_sub_len(EmacsByteLen::new(1));
        let preceding_lisp_pos = eval
            .buffers
            .get(buffer_id)
            .map(|buf| buf.emacs_byte_pos_to_lisp_char_pos(preceding).as_i64())
            .unwrap_or(1);
        if crate::emacs_core::xdisp::invisible_status_for_value(
            eval,
            Value::fixnum(preceding_lisp_pos),
        )? == 0
        {
            break;
        }
        let Some(previous) = eval
            .buffers
            .get(buffer_id)
            .and_then(|buf| previous_logical_line_start(buf, current))
        else {
            current = accessible.start();
            break;
        };
        current = previous;
    }

    loop {
        let Some(step) =
            next_screen_line_start_from(eval, buffer_id, current, screen_width, wrap)?
        else {
            return Ok(Some(current));
        };
        if step.next <= target && step.counts_line {
            current = step.next;
        } else {
            return Ok(Some(current));
        }
    }
}

/// GNU `char_can_wrap_after` (src/xdisp.c:599-617) with the default
/// `word-wrap-by-category` nil: a wrap MAY follow a whitespace glyph.
///
/// GNU asks the iterator (`IT_DISPLAYING_WHITESPACE`), i.e. the glyph actually
/// being displayed; this scanner runs only when no realized row is available,
/// so it asks the buffer character instead.
fn char_can_wrap_after(code: u32) -> bool {
    code == b' ' as u32 || code == b'\t' as u32
}

/// GNU `char_can_wrap_before` (src/xdisp.c:577-596) with the default
/// `word-wrap-by-category` nil: a wrap MAY precede any non-whitespace glyph.
///
/// GNU's own comment is the reason for the asymmetry: "You cannot wrap before
/// a space or tab because that way you'll have space and tab at the beginning
/// of next line."
fn char_can_wrap_before(code: u32) -> bool {
    !char_can_wrap_after(code)
}

/// Where a [`LineWrap::WordWrap`] row is allowed to break, as GNU's `wrap_it`
/// records it (src/xdisp.c:10280-10300).
///
/// GNU saves a wrap point when it reaches a glyph that both FOLLOWS a
/// wrappable glyph (`may_wrap`) and CAN BE WRAPPED BEFORE; when the row then
/// overflows it restores that saved point.  With no saved point the row falls
/// back to breaking at the edge, which is exactly `LineWrap::WindowWrap` --
/// GNU: `if (it->line_wrap != WORD_WRAP || wrap_it.sp < 0)` (src/xdisp.c:10612).
#[derive(Clone, Copy, Debug)]
struct WordWrapPoint {
    /// True once the previous glyph on this row allowed a wrap after it.
    may_wrap: bool,
    /// The last saved wrap position -- GNU's `wrap_it`.
    saved: Option<EmacsBytePos>,
}

impl WordWrapPoint {
    /// GNU starts every row with `may_wrap` false and no saved point.
    fn new() -> Self {
        Self {
            may_wrap: false,
            saved: None,
        }
    }

    /// Observe the glyph at `pos` before it is placed, updating the saved wrap
    /// point the way GNU does inside `move_it_in_display_line_to`.
    fn observe(&mut self, pos: EmacsBytePos, row_start: EmacsBytePos, code: u32) {
        if self.may_wrap && char_can_wrap_before(code) && pos > row_start {
            self.saved = Some(pos);
        }
        self.may_wrap = char_can_wrap_after(code);
    }

    /// Where the row breaks when the glyph at `overflow_at` does not fit.
    fn break_at(self, overflow_at: EmacsBytePos) -> EmacsBytePos {
        self.saved.unwrap_or(overflow_at)
    }
}

fn next_screen_line_start_from(
    eval: &mut super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    start: EmacsBytePos,
    screen_width: usize,
    wrap: LineWrap,
) -> Result<Option<ScreenLineStep>, Flow> {
    let point_max = match eval.buffers.get(buffer_id) {
        Some(buf) => buf.accessible_emacs_byte_region().end(),
        None => return Ok(None),
    };
    let mut scan = start;
    let mut column = 0usize;
    let mut word_wrap = WordWrapPoint::new();
    // One stop cache for this forward screen-line scan: display/invisible/
    // composition/overlay probes run only at a stop, not per char (GNU
    // it->stop_charpos). Fresh per call, so it never observes a mid-scan edit.
    let stop_cache = DisplayStopCache::new();

    // Where a row that overflows at BYTE actually breaks, per wrap method.
    let break_position = |word_wrap: WordWrapPoint, overflow_at: EmacsBytePos| match wrap {
        LineWrap::WordWrap => word_wrap.break_at(overflow_at),
        LineWrap::WindowWrap | LineWrap::Truncate => overflow_at,
    };

    while scan < point_max {
        #[cfg(test)]
        SCREEN_LINE_SCAN_STEPS.with(|steps| steps.set(steps.get().saturating_add(1)));
        let Some(advance) = display_advance_at(eval, buffer_id, scan.get(), column, &stop_cache)?
        else {
            return Ok(None);
        };
        if advance.next_byte <= scan.get() {
            return Ok(None);
        }
        let next = EmacsBytePos::new(advance.next_byte);
        if advance.hard_newline {
            return Ok(Some(ScreenLineStep {
                next,
                counts_line: true,
            }));
        }
        if wrap == LineWrap::WordWrap
            && let Some(code) = eval
                .buffers
                .get(buffer_id)
                .and_then(|buf| buf.char_code_after_emacs_byte_pos(scan))
        {
            word_wrap.observe(scan, start, code);
        }
        if wrap.truncates() && column.saturating_add(advance.width) > screen_width {
            return Ok(Some(truncated_logical_line_step(eval, buffer_id, scan)?));
        }
        if advance.unbreakable_wide
            && scan > start
            && column.saturating_add(advance.width) > screen_width
        {
            let next = break_position(word_wrap, scan);
            return Ok(Some(ScreenLineStep {
                next,
                counts_line: true,
            }));
        }
        scan = next;
        column = column.saturating_add(advance.width);
        if column >= screen_width {
            if wrap.truncates() {
                return Ok(Some(truncated_logical_line_step(eval, buffer_id, scan)?));
            }
            // GNU inspects the glyph FIRST and only then discovers that it
            // overflows the row (src/xdisp.c:10280 runs before PRODUCE_GLYPHS),
            // so the overflowing glyph can itself be the saved wrap point --
            // that is how "delta epsilon zeta eta |theta" breaks before
            // `theta' rather than before `eta'.  `scan' is one past the last
            // glyph that fit, i.e. exactly at the glyph that does not.
            if wrap == LineWrap::WordWrap
                && let Some(code) = eval
                    .buffers
                    .get(buffer_id)
                    .and_then(|buf| buf.char_code_after_emacs_byte_pos(scan))
            {
                word_wrap.observe(scan, start, code);
            }
            // Under WORD_WRAP the row ends at the last saved wrap point, and
            // the glyphs between it and here belong to the NEXT row.
            let next = break_position(word_wrap, scan);
            return Ok(Some(ScreenLineStep {
                next,
                // A wrap only begins a screen line if something is left to put
                // on it. Filling the width with the buffer's last character
                // moves point to the end without occupying a following line,
                // and GNU counts screen lines actually moved over -- its batch
                // path stops `compute_motion' at ZV, so the count stays zero
                // there even though point moves. Counting it would also report
                // a line that redisplay never draws.
                counts_line: next < point_max,
            }));
        }
    }

    if start < point_max {
        Ok(Some(ScreenLineStep {
            next: point_max,
            counts_line: false,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
thread_local! {
    static SCREEN_LINE_SCAN_STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_screen_line_scan_steps_for_test() {
    SCREEN_LINE_SCAN_STEPS.with(|steps| steps.set(0));
}

#[cfg(test)]
pub(crate) fn screen_line_scan_steps_for_test() -> usize {
    SCREEN_LINE_SCAN_STEPS.with(std::cell::Cell::get)
}

fn truncated_logical_line_step(
    eval: &super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    from: EmacsBytePos,
) -> Result<ScreenLineStep, Flow> {
    let Some(buf) = eval.buffers.get(buffer_id) else {
        return Ok(ScreenLineStep {
            next: from,
            counts_line: false,
        });
    };
    let point_max = buf.accessible_emacs_byte_region().end();
    let mut scan = from;
    while scan < point_max {
        if buf.emacs_byte_at_pos(scan) == Some(b'\n') {
            return Ok(ScreenLineStep {
                next: scan.add_len(EmacsByteLen::new(1)),
                counts_line: true,
            });
        }
        let char_len = buf
            .char_after_emacs_byte_len(scan)
            .map(|len| len.max(EmacsByteLen::new(1)))
            .unwrap_or(EmacsByteLen::new(1));
        scan = scan.add_len(char_len);
    }
    Ok(ScreenLineStep {
        next: point_max,
        counts_line: false,
    })
}

#[derive(Clone, Copy, Debug)]
struct LiveSnapshotVerticalMotion {
    target: LispCharPos1,
    moved: i64,
}

fn live_vertical_motion_snapshot(
    eval: &super::eval::Context,
    window: Option<Value>,
    current_buffer: BufferId,
) -> Option<&WindowDisplaySnapshot> {
    let window = window.filter(|value| !value.is_nil());
    let (frame_id, window_id) = if let Some(window) = window {
        let window_id = WindowId(window.as_window_id()?);
        let frame_id = eval.frames.find_window_frame_id(window_id)?;
        (frame_id, window_id)
    } else {
        let frame = eval.frames.selected_frame()?;
        (frame.id, frame.selected_window)
    };
    eval.fresh_window_display_snapshot(frame_id, window_id, current_buffer)
}

fn snapshot_text_rows(snapshot: &WindowDisplaySnapshot) -> Vec<&DisplayRowSnapshot> {
    let mut rows: Vec<_> = snapshot
        .rows
        .iter()
        .filter(|row| row.start_buffer_pos.is_some() && row.end_buffer_pos.is_some())
        .collect();
    rows.sort_by_key(|row| row.row);
    rows
}

fn snapshot_row_index_for_pos(rows: &[&DisplayRowSnapshot], pos: LispCharPos1) -> Option<usize> {
    rows.iter().position(|row| {
        let Some(start) = row.start_buffer_pos else {
            return false;
        };
        let Some(end) = row.end_buffer_pos else {
            return false;
        };
        start <= pos && pos <= end
    })
}

/// One place on a screen row where GNU's goal-column walk can come to rest.
///
/// GNU reaches a `vertical-motion` goal column with
/// `move_it_in_display_line (&it, ZV, first_x + to_x, MOVE_TO_X)`
/// (src/indent.c:2540), and `move_it_in_display_line_to` has TWO ways to stop:
/// at a glyph that reaches the goal x, or -- when the goal is past everything
/// the row draws -- where the DISPLAY LINE itself ends. Naming both as stops
/// keeps that second exit from being an afterthought: a row's end is a
/// position in its own right, and on a newline-terminated row it is the
/// newline, which sits one column past the last glyph because it draws none.
#[derive(Clone, Copy)]
struct RowGoalStop {
    col: i64,
    x: i64,
    pos: LispCharPos1,
}

impl RowGoalStop {
    /// Ordering key for "the LAST stop that does not pass the goal column".
    ///
    /// GNU's `MOVE_TO_X` walk places a glyph only while it still fits before
    /// the goal, and backs up to `x_before_this_char` as soon as one would
    /// pass it (src/xdisp.c:10385-10400), so the answer is the greatest stop
    /// column that is `<= goal` -- never the nearer stop beyond it.  Measured
    /// under GNU Emacs 31.0.90 on a 24-column window whose row starts with a
    /// TAB: goal columns 1 through 7 all answer the TAB's own position at
    /// column 0, and only goal 8 reaches the glyph after it.
    fn reach_key(self, target_col: i64) -> (i64, i64, i64, i64) {
        let reached = self.col <= target_col;
        // Among reachable stops take the greatest column; among unreachable
        // ones (a goal before the row's first stop) take the smallest.
        let order = if reached { self.col } else { -self.col };
        (i64::from(reached), order, self.x, self.pos.as_i64())
    }
}

/// Every stop the goal-column walk may land on for one row: the drawn glyphs,
/// then the row's own end boundary.
fn row_goal_stops(
    snapshot: &WindowDisplaySnapshot,
    row: &DisplayRowSnapshot,
) -> impl Iterator<Item = RowGoalStop> {
    let glyphs = snapshot
        .points
        .iter()
        .filter(|point| point.row == row.row)
        .map(|point| RowGoalStop {
            col: point.col,
            x: point.x,
            pos: point.buffer_pos,
        });
    let row_end = row.end_buffer_pos.map(|pos| RowGoalStop {
        col: row.end_col,
        x: row.end_x,
        pos,
    });
    glyphs.chain(row_end)
}

fn snapshot_target_pos_on_row(
    snapshot: &WindowDisplaySnapshot,
    row: &DisplayRowSnapshot,
    cols: Option<i64>,
) -> Option<LispCharPos1> {
    let Some(target_col) = cols.map(|col| col.max(0)) else {
        return row.start_buffer_pos;
    };
    row_goal_stops(snapshot, row)
        .max_by_key(|stop| stop.reach_key(target_col))
        .map(|stop| stop.pos)
        .or(row.start_buffer_pos)
}

fn vertical_motion_from_live_snapshot(
    eval: &super::eval::Context,
    window: Option<Value>,
    current_buffer: BufferId,
    point: LispCharPos1,
    cols: Option<i64>,
    lines: i64,
) -> Option<LiveSnapshotVerticalMotion> {
    if eval.noninteractive() {
        return None;
    }
    let snapshot = live_vertical_motion_snapshot(eval, window, current_buffer)?;
    let rows = snapshot_text_rows(snapshot);
    let current_idx = snapshot_row_index_for_pos(&rows, point)?;
    let target_idx = current_idx as i64 + lines;
    if !(0..rows.len() as i64).contains(&target_idx) {
        return None;
    }
    let target_idx = target_idx as usize;
    let target = snapshot_target_pos_on_row(snapshot, rows[target_idx], cols)?;
    Some(LiveSnapshotVerticalMotion {
        target,
        moved: target_idx as i64 - current_idx as i64,
    })
}

/// Register the Lisp primitives owned by GNU `indent.c`.
pub(crate) fn syms_of_indent(ctx: &mut super::eval::Context) {
    ctx.defsubr("current-indentation", builtin_current_indentation, 0, None);
    ctx.defsubr_interactive(
        "indent-to",
        builtin_indent_to,
        1,
        Some(2),
        super::interactive::BuiltinInteractiveSpec::String("NIndent to column: "),
    );
    ctx.defsubr("current-column", builtin_current_column, 0, Some(0));
    ctx.defsubr_interactive(
        "move-to-column",
        builtin_move_to_column,
        1,
        Some(2),
        super::interactive::BuiltinInteractiveSpec::String("NMove to column: "),
    );
    ctx.defsubr(
        "line-number-display-width",
        builtin_line_number_display_width,
        0,
        None,
    );
    super::builtins::register_builtin_requires_eval_state(
        ctx,
        "vertical-motion",
        builtin_vertical_motion,
        1,
        Some(3),
    );
    ctx.defsubr("compute-motion", builtin_compute_motion, 7, Some(7));
}

/// `(vertical-motion LINES &optional WINDOW CUR-COL)` -> integer
///
/// Move point to the start of the screen line LINES lines down (or up if
/// negative).  Returns the number of lines actually moved.
///
/// In GNU Emacs this uses the full display engine to handle word-wrap,
/// display properties, etc.  In live frames, use the last redisplay snapshot
/// for visible rows; otherwise approximate with buffer scanning.
pub(crate) fn builtin_vertical_motion(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("vertical-motion", &args, 1, 3)?;
    // First arg can be LINES (integer) or (COLS . LINES) cons pair.
    // When (COLS . LINES), move LINES then position at column COLS.
    let (cols, lines): (Option<i64>, i64) = match args[0].kind() {
        ValueKind::Fixnum(n) => (None, n),
        ValueKind::Cons => {
            let car = args[0].cons_car();
            let cdr = args[0].cons_cdr();
            let cols_val = match car.kind() {
                ValueKind::Fixnum(n) => Some(n),
                ValueKind::Float => Some(car.xfloat() as i64),
                _ => None,
            };
            let lines_val = match cdr.kind() {
                ValueKind::Fixnum(n) => n,
                _ => {
                    return Err(signal(
                        LispCondition::WrongTypeArgument,
                        vec![Value::symbol("fixnump"), cdr],
                    ));
                }
            };
            (cols_val, lines_val)
        }
        _ => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("fixnump"), args[0]],
            ));
        }
    };
    // Validate optional WINDOW arg.
    if let Some(window) = args.get(1)
        && !window.is_nil()
        && !window.is_window()
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("window-live-p"), *window],
        ));
    }

    let Some(current_id) = eval.buffers.current_buffer_id() else {
        return Ok(Value::fixnum(0));
    };
    let Some(buf) = eval.buffers.get(current_id) else {
        return Ok(Value::fixnum(0));
    };
    let accessible = buf.accessible_emacs_byte_region();
    let pt = accessible.clamp(buf.point_emacs_byte_pos());
    let pt_lisp = buf.emacs_byte_pos_to_lisp_char_pos(pt);
    let begv = accessible.start();

    if cols.is_none() {
        let (target, moved) =
            screen_line_motion_target(eval, current_id, pt, args.get(1).copied(), lines)?;
        let _ = eval.buffers.goto_buffer_emacs_byte_pos(current_id, target);
        return Ok(Value::fixnum(moved));
    }

    if let Some(snapshot_motion) = vertical_motion_from_live_snapshot(
        eval,
        args.get(1).copied(),
        current_id,
        pt_lisp,
        cols,
        lines,
    ) {
        let target = eval
            .buffers
            .get(current_id)
            .map(|buf| buf.lisp_pos_to_emacs_byte_pos(snapshot_motion.target))
            .unwrap_or(pt);
        let _ = eval.buffers.goto_buffer_emacs_byte_pos(current_id, target);
        return Ok(Value::fixnum(snapshot_motion.moved));
    }

    let screen_width = vertical_motion_screen_width(eval, args.get(1).copied());
    let wrap = super::window_cmds::window_line_wrap_for_motion(eval, args.get(1).copied(), current_id);

    if lines == 0 && cols.is_none() {
        // Move to beginning of current screen line.
        let bol = current_screen_line_start_with_truncation(
            eval,
            current_id,
            pt,
            screen_width,
            wrap,
        )?
        .unwrap_or(begv);
        let _ = eval.buffers.goto_buffer_emacs_byte_pos(current_id, bol);
        return Ok(Value::fixnum(0));
    }

    let mut pos = current_screen_line_start_with_truncation(
        eval,
        current_id,
        pt,
        screen_width,
        wrap,
    )?
    .unwrap_or(pt);
    let mut moved: i64 = 0;

    if lines > 0 {
        for _ in 0..lines {
            let Some(step) =
                next_visible_line_start(eval, current_id, pos, screen_width, wrap)?
            else {
                break;
            };
            pos = step.next;
            if step.counts_line {
                moved += 1;
            }
            if !step.counts_line {
                break;
            }
        }
    } else if lines < 0 {
        (pos, moved) = previous_screen_line_target(
            eval,
            current_id,
            pos,
            screen_width,
            wrap,
            (-lines) as usize,
        )?;
    } else {
        // lines == 0 but cols is Some: stay on current screen line.
        pos = current_screen_line_start_with_truncation(
            eval,
            current_id,
            pt,
            screen_width,
            wrap,
        )?
        .unwrap_or(begv);
    }

    // Now pos is at beginning of target line.
    // If COLS was specified, advance to that column.
    let _ = eval.buffers.goto_buffer_emacs_byte_pos(current_id, pos);
    if let Some(target_col) = cols {
        // GNU positions to COLS through the display engine, which needs a live
        // window with glyph matrices. Under `noninteractive` (batch) there is no
        // display, so GNU's `vertical-motion` leaves point at the beginning of
        // the line rather than at COLS. Mirror that so display-driven Lisp such
        // as `shr-fill-line` sees GNU's line breaking.
        if !eval.noninteractive() {
            let target = goal_column_target_on_screen_line(
                eval,
                current_id,
                pos,
                screen_width,
                target_col.max(0),
            )?;
            let _ = eval.buffers.goto_buffer_emacs_byte_pos(current_id, target);
        }
    }
    Ok(Value::fixnum(moved))
}

/// GNU `move_it_in_display_line (&it, ZV, first_x + to_x, MOVE_TO_X)`
/// (src/indent.c:2540) over one screen row, for the fallback scanner.
///
/// The walk stops at the LAST position on the row whose display column does
/// not pass the goal, and never leaves the row: GNU's
/// `move_it_in_display_line_to` also stops where the display line itself ends
/// (`it->last_visible_x`), so a goal past everything the row draws answers the
/// row's end rather than running on into the next one.
///
/// Measured under GNU Emacs 31.0.90 on a 24-column window (body 24, so the
/// reachable columns are 0..=23 in BOTH a wrapped and a truncated window):
///
/// * a row of `x` characters answers column N for every goal N <= 23, and
///   saturates at column 23 for every goal above it -- truncated or wrapped;
/// * a row starting with a TAB answers the TAB's own position for goals 1..7
///   and only reaches the glyph after it at goal 8.
///
/// That last case is why `move-to-column` cannot stand in here: GNU stops
/// BEFORE the glyph that would pass the goal, while `move-to-column` moves
/// past a TAB to the column where it ends.
fn goal_column_target_on_screen_line(
    eval: &mut super::eval::Context,
    buffer_id: BufferId,
    row_start: EmacsBytePos,
    screen_width: usize,
    goal_col: i64,
) -> Result<EmacsBytePos, Flow> {
    let point_max = match eval.buffers.get(buffer_id) {
        Some(buf) => buf.accessible_emacs_byte_region().end(),
        None => return Ok(row_start),
    };
    let goal = goal_col.max(0) as usize;
    let stop_cache = DisplayStopCache::new();
    let mut scan = row_start;
    let mut column = 0usize;
    let mut reached = row_start;

    while scan < point_max {
        // A stop past the row's right edge is not on this row at all.
        if column > screen_width {
            break;
        }
        if column <= goal {
            reached = scan;
        } else {
            break;
        }
        let Some(advance) = display_advance_at(eval, buffer_id, scan.get(), column, &stop_cache)?
        else {
            break;
        };
        if advance.next_byte <= scan.get() || advance.hard_newline {
            // The newline is the row's last stop; nothing after it is on it.
            break;
        }
        scan = EmacsBytePos::new(advance.next_byte);
        column = column.saturating_add(advance.width);
    }
    Ok(reached)
}

/// Result of a display-property-aware screen-line scan.
///
/// `target` preserves GNU `vertical-motion`'s end-of-buffer behavior, while
/// `last_occupied_target` records the last row that actually counted as
/// motion. Pixel-distance consumers need the latter when an unterminated
/// final row exhausts the scan.
pub(crate) struct ScreenLineMotionScan {
    pub(crate) target: EmacsBytePos,
    pub(crate) moved: i64,
    pub(crate) last_occupied_target: EmacsBytePos,
}

/// Resolve screen-line motion without changing point.
///
/// This is the shared display-motion seam used by both `vertical-motion` and
/// window scrolling. It prefers the last live redisplay snapshot and falls
/// back to the display-property-aware scanner for motion beyond the currently
/// visible rows.
pub(crate) fn screen_line_motion_target(
    eval: &mut super::eval::Context,
    current_buffer: BufferId,
    point: EmacsBytePos,
    window: Option<Value>,
    lines: i64,
) -> Result<(EmacsBytePos, i64), Flow> {
    let Some(buf) = eval.buffers.get(current_buffer) else {
        return Ok((point, 0));
    };
    let accessible = buf.accessible_emacs_byte_region();
    let point = accessible.clamp(point);
    let point_lisp = buf.emacs_byte_pos_to_lisp_char_pos(point);

    if let Some(snapshot_motion) =
        vertical_motion_from_live_snapshot(eval, window, current_buffer, point_lisp, None, lines)
    {
        let target = eval
            .buffers
            .get(current_buffer)
            .map(|buf| buf.lisp_pos_to_emacs_byte_pos(snapshot_motion.target))
            .unwrap_or(point);
        return Ok((target, snapshot_motion.moved));
    }

    scan_screen_line_motion_target(eval, current_buffer, point, window, lines)
        .map(|motion| (motion.target, motion.moved))
}

/// Scan screen lines without consulting retained redisplay geometry.
///
/// Callers use this when they have already consumed a live snapshot (and may
/// have reached its edge), or when that snapshot is stale for current layout
/// state. Keeping the scanner independently callable prevents a fallback from
/// accidentally re-entering the same retained rows.
pub(crate) fn scan_screen_line_motion_target(
    eval: &mut super::eval::Context,
    current_buffer: BufferId,
    point: EmacsBytePos,
    window: Option<Value>,
    lines: i64,
) -> Result<ScreenLineMotionScan, Flow> {
    let Some(buf) = eval.buffers.get(current_buffer) else {
        return Ok(ScreenLineMotionScan {
            target: point,
            moved: 0,
            last_occupied_target: point,
        });
    };
    let point = buf.accessible_emacs_byte_region().clamp(point);

    let screen_width = vertical_motion_screen_width(eval, window);
    let wrap = super::window_cmds::window_line_wrap_for_motion(eval, window, current_buffer);
    let mut target = current_screen_line_start_with_truncation(
        eval,
        current_buffer,
        point,
        screen_width,
        wrap,
    )?
    .unwrap_or(point);
    let mut moved = 0_i64;
    let mut last_occupied_target = target;

    if lines > 0 {
        for _ in 0..lines {
            let Some(step) =
                next_visible_line_start(eval, current_buffer, target, screen_width, wrap)?
            else {
                break;
            };
            target = step.next;
            if step.counts_line {
                moved += 1;
                last_occupied_target = target;
            } else {
                break;
            }
        }
    } else if lines < 0 {
        (target, moved) = previous_screen_line_target(
            eval,
            current_buffer,
            target,
            screen_width,
            wrap,
            (-lines) as usize,
        )?;
        last_occupied_target = target;
    }

    Ok(ScreenLineMotionScan {
        target,
        moved,
        last_occupied_target,
    })
}

// ---------------------------------------------------------------------------
// last_known_column cache (GNU src/indent.c:40-51, 323-342)
// ---------------------------------------------------------------------------
//
// GNU caches the column of point so a `current-column' that immediately follows
// an operation which already computed it (e.g. `indent-to') returns the cached
// value without rescanning the line.  This is not merely an optimization: it is
// observable, because the cached value reflects the `tab-width' in effect *when
// the column was computed*, even if `tab-width' has since changed (e.g. a
// dynamic `let' has been unwound).  Rescanning would use the current
// `tab-width' and produce a different answer (oracle test cx122).
//
// GNU keeps a single global tied to the current buffer; we additionally key on
// the buffer id so a buffer switch invalidates the cache, and on the buffer's
// modification tick so any edit invalidates it (GNU compares `MODIFF').

#[derive(Clone, Copy)]
struct LastKnownColumn {
    buffer_id: u64,
    point: EmacsBytePos,
    modiff: i64,
    column: usize,
}

thread_local! {
    static LAST_KNOWN_COLUMN: Cell<Option<LastKnownColumn>> = const { Cell::new(None) };
}

/// Record the column of point after it has just been computed.
fn set_last_known_column(buffer_id: u64, point: EmacsBytePos, modiff: i64, column: usize) {
    LAST_KNOWN_COLUMN.with(|slot| {
        slot.set(Some(LastKnownColumn {
            buffer_id,
            point,
            modiff,
            column,
        }))
    });
}

/// Return the cached column if it is still valid for (buffer, point, modiff).
///
/// Unlike GNU's explicit `invalidate_current_column`, the (buffer, point,
/// modiff) key is self-invalidating: any point movement or buffer edit changes
/// the key and forces a fresh scan, so no separate invalidation hook is needed.
fn cached_current_column(buffer_id: u64, point: EmacsBytePos, modiff: i64) -> Option<usize> {
    LAST_KNOWN_COLUMN.with(|slot| {
        slot.get().and_then(|c| {
            (c.buffer_id == buffer_id && c.point == point && c.modiff == modiff).then_some(c.column)
        })
    })
}

// ---------------------------------------------------------------------------
// Argument helpers (local to this module)
// ---------------------------------------------------------------------------

fn expect_fixnump(val: &Value) -> Result<i64, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("fixnump"), *val],
        )),
    }
}

fn expect_wholenump(val: &Value) -> Result<usize, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) if n >= 0 => Ok(n as usize),
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("wholenump"), *val],
        )),
    }
}

pub(crate) fn dynamic_buffer_or_global_symbol_value(
    obarray: &Obarray,
    _dynamic: &[OrderedRuntimeBindingMap],
    buf: Option<&Buffer>,
    name: &str,
) -> Option<Value> {
    // Phase 10D: BUFFER_OBJFWD slots (always-local AND conditional)
    // store the live value in `buf.slots[offset]`. After
    // `set-default` propagation, conditional slots whose
    // local-flags bit is clear still reflect the latest global
    // default in their per-buffer slot, so reading the slot
    // directly is correct in both cases. `get_buffer_local`
    // returns None for conditional slots with the bit clear,
    // which would otherwise lose the live value here.
    if let Some(buf) = buf
        && let Some(info) = crate::buffer::buffer::lookup_buffer_slot(name)
    {
        return Some(buf.slots[info.offset.index()]);
    }
    if let Some(buf) = buf
        && let Some(value) = buf.get_buffer_local(name)
    {
        return Some(value);
    }
    obarray.symbol_value(name).copied()
}

fn tab_width_in_state(
    obarray: &Obarray,
    dynamic: &[OrderedRuntimeBindingMap],
    buf: Option<&Buffer>,
) -> usize {
    match dynamic_buffer_or_global_symbol_value(obarray, dynamic, buf, "tab-width") {
        Some(v) if v.is_fixnum() && v.as_fixnum().unwrap() > 0 => v.as_fixnum().unwrap() as usize,
        Some(v) if v.is_char() && (v.as_char().unwrap() as u32) > 0 => {
            v.as_char().unwrap() as usize
        }
        _ => 8,
    }
}

/// Current buffer's `tab-width', used by `char-width' for a TAB character.
///
/// GNU `CHARACTER_WIDTH` (buffer.h) returns `SANE_TAB_WIDTH (current_buffer)`
/// for `\t', i.e. the buffer-local `tab-width' clamped to 1..1000.  This is
/// the column width `char-width' reports for a tab, and what
/// `internal_self_insert' uses to decide how much to overwrite.
pub(crate) fn current_buffer_tab_width(ctx: &crate::emacs_core::eval::Context) -> usize {
    let buf = ctx.buffers.current_buffer();
    let width = tab_width_in_state(&ctx.obarray, &[], buf);
    // GNU SANE_TAB_WIDTH clamps to 1..=1000.
    width.clamp(1, 1000)
}

fn indent_tabs_mode_in_state(
    obarray: &Obarray,
    dynamic: &[OrderedRuntimeBindingMap],
    buf: Option<&Buffer>,
) -> bool {
    dynamic_buffer_or_global_symbol_value(obarray, dynamic, buf, "indent-tabs-mode")
        .is_none_or(|value| value.is_truthy())
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn buffer_read_only_active(eval: &super::eval::Context, buf: &Buffer) -> bool {
    super::editfns::buffer_read_only_active_in_state(&eval.obarray, &[], buf)
}

#[derive(Clone, Copy)]
struct DecodedUnit {
    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    start: usize,
    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    end: usize,
    code: u32,
    width: usize,
}

#[derive(Clone, Copy, Debug)]
struct ColumnScan {
    byte_pos: EmacsBytePos,
    column: usize,
    previous_byte_pos: EmacsBytePos,
    previous_column: usize,
    previous_code: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DisplayAdvance {
    pub(crate) next_byte: usize,
    pub(crate) width: usize,
    pub(crate) hard_newline: bool,
    pub(crate) unbreakable_wide: bool,
}

fn line_bounds(buf: &Buffer, point: EmacsBytePos) -> EmacsByteRange {
    let accessible = buf.accessible_emacs_byte_region();
    let begv = accessible.start();
    let zv = accessible.end();
    let pt = accessible.clamp(point);
    let one_byte = EmacsByteLen::new(1);

    let mut bol = pt;
    while bol > begv {
        let prev = bol.saturating_sub_len(one_byte);
        if buf.emacs_byte_at_pos(prev) == Some(b'\n') {
            break;
        }
        bol = prev;
    }

    let mut eol = pt;
    while eol < zv && buf.emacs_byte_at_pos(eol) != Some(b'\n') {
        eol = eol.add_len(one_byte);
    }

    EmacsByteRange::new(bol, eol)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn next_column(column: usize, ch: char, tab_width: usize) -> usize {
    if ch == '\t' {
        let tab = tab_width.max(1);
        column + (tab - (column % tab))
    } else {
        column + crate::encoding::char_width(ch)
    }
}

fn next_column_for_code(column: usize, code: u32, width: usize, tab_width: usize) -> usize {
    if code == b'\t' as u32 {
        let tab = tab_width.max(1);
        column + (tab - (column % tab))
    } else {
        column + width
    }
}

fn raw_unibyte_display_width(byte: u8) -> usize {
    if !(0o40..0o177).contains(&byte) { 4 } else { 1 }
}

/// Upper bound (in chars) on how far ahead a single stop computation looks for
/// the next display-relevant property change. Mirrors GNU's
/// `TEXT_PROP_DISTANCE_LIMIT` (src/xdisp.c): it caps the interval walk so the
/// cost of recomputing a stop stays bounded even in a buffer with no display
/// properties at all, at the price of an occasional redundant recompute.
const DISPLAY_STOP_CHAR_CAP: usize = 100;

/// Per-scan cache of the next byte position at which a display-relevant property
/// (`invisible`, `display`, `composition`, or any overlay) could change, so the
/// per-char display scanner runs the three expensive property probes only at a
/// stop and skips them in between. This mirrors GNU's `it->stop_charpos` /
/// `compute_stop_pos` (src/xdisp.c) and reuses the same discipline as
/// `SyntaxPropRange` in syntax.rs: construct one fresh per forward scan so it
/// never observes a mid-scan edit (screen-line motion is read-only; font-lock
/// and syntax-propertize ran earlier).
pub(crate) struct DisplayStopCache {
    recompute_at: std::cell::Cell<usize>,
}

impl DisplayStopCache {
    pub(crate) fn new() -> Self {
        // recompute_at = 0 forces the first call (at any byte >= 0) to probe.
        Self {
            recompute_at: std::cell::Cell::new(0),
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Counts how many times `display_advance_at` recomputes a stop (i.e. runs
    /// the three property probes). For plain text this must be O(scan / cap),
    /// not O(scan); a test asserts the ratio to guard against regressing to
    /// per-char probing.
    static DISPLAY_STOP_RECOMPUTES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_display_stop_recomputes_for_test() {
    DISPLAY_STOP_RECOMPUTES.with(|n| n.set(0));
}

#[cfg(test)]
pub(crate) fn display_stop_recomputes_for_test() -> usize {
    DISPLAY_STOP_RECOMPUTES.with(std::cell::Cell::get)
}

/// Byte position of the next stop after `byte`: the earliest place at which
/// `invisible`, `display`, or `composition` could change value, folding in the
/// coarse next-overlay-boundary (GNU `next_overlay_change`) and bounded by
/// `DISPLAY_STOP_CHAR_CAP` and the accessible end. The result is strictly
/// greater than `byte`, so the scan always makes progress before re-probing.
/// The display-relevant properties `display_advance_at` probes per stop. This is
/// the SINGLE source of truth binding together the three things that MUST agree:
/// the STOP-branch probes, the watched keys `next_display_stop` scans for, and
/// the fast-path validator. Adding a variant is a compiler-forced exhaustiveness
/// error in `key()` and `probe()`, and `ALL` carries it into every consumer, so
/// a new display-relevant probe cannot silently desync from the stop it computes
/// (the footgun of three hand-synced lists this replaces).
#[derive(Clone, Copy)]
enum DisplayStopProp {
    Invisible,
    Display,
    Composition,
}

impl DisplayStopProp {
    const ALL: [DisplayStopProp; 3] = [Self::Invisible, Self::Display, Self::Composition];

    /// The text-property name whose value-change bounds this property's run
    /// (what `next_display_stop` watches).
    fn key(self) -> &'static str {
        match self {
            Self::Invisible => "invisible",
            Self::Display => "display",
            Self::Composition => "composition",
        }
    }

    /// Probe this property at `byte`, returning the coalesced run as
    /// `(display_width, run_end_byte)` when it is present and its run ends
    /// strictly after `byte` (run end clamped to `end`); None otherwise.
    fn probe(
        self,
        ctx: &mut super::eval::Context,
        buffer_id: crate::buffer::BufferId,
        byte: usize,
        column: usize,
        end: usize,
    ) -> Result<Option<(usize, usize)>, Flow> {
        let hit =
            match self {
                Self::Invisible => {
                    super::xdisp::zero_width_invisible_run_end_byte(ctx, buffer_id, byte)?
                        .filter(|&next_visible| next_visible > byte)
                        .map(|next_visible| (0usize, next_visible))
                }
                Self::Display => display_run_at(ctx, buffer_id, byte, column)
                    .filter(|&(_, run_end)| run_end > byte),
                Self::Composition => composition_run_at(ctx, buffer_id, byte)
                    .filter(|&(_, comp_end)| comp_end > byte),
            };
        Ok(hit.map(|(width, run_end)| (width, run_end.min(end))))
    }
}

fn next_display_stop(
    ctx: &super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    byte: usize,
    end: usize,
) -> usize {
    let Some(buf) = ctx.buffers.get(buffer_id) else {
        return end;
    };
    let char_pos = buf.emacs_byte_pos_to_char_pos_clamped(EmacsBytePos::new(byte));
    let cap_char = CharPos0::new(char_pos.get().saturating_add(DISPLAY_STOP_CHAR_CAP));
    // Watched keys derive from the SAME source as the probes (DisplayStopProp),
    // so a new probe automatically becomes a watched key here.
    let watched = DisplayStopProp::ALL.map(|prop| Value::symbol(prop.key()));
    let change_char = buf.next_watched_property_change_at_char_pos(char_pos, cap_char, &watched);
    let mut stop = buf.char_pos_to_emacs_byte_pos_clamped(change_char).get();
    if let Some(overlay_boundary) = buf
        .overlays
        .next_boundary_after_emacs_byte_pos(EmacsBytePos::new(byte))
    {
        stop = stop.min(overlay_boundary.get());
    }
    stop.min(end).max(byte.saturating_add(1))
}

pub(crate) fn display_advance_at(
    ctx: &mut super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    byte: usize,
    column: usize,
    stop: &DisplayStopCache,
) -> Result<Option<DisplayAdvance>, Flow> {
    let (end, tab_width, display_table) = {
        let buf = match ctx.buffers.get(buffer_id) {
            Some(buf) => buf,
            None => return Ok(None),
        };
        let display_table = dynamic_buffer_or_global_symbol_value(
            &ctx.obarray,
            &[],
            Some(buf),
            "buffer-display-table",
        )
        .filter(|v| !v.is_nil())
        .or_else(|| ctx.obarray.symbol_value("standard-display-table").copied())
        .filter(|v| !v.is_nil());
        (
            buf.accessible_emacs_byte_region().end().get(),
            tab_width_in_state(&ctx.obarray, &[], Some(buf)),
            display_table,
        )
    };
    if byte >= end {
        return Ok(None);
    }

    if byte >= stop.recompute_at.get() {
        #[cfg(test)]
        DISPLAY_STOP_RECOMPUTES.with(|n| n.set(n.get().saturating_add(1)));
        // STOP: probe each display-relevant property (DisplayStopProp) in order.
        // On a hit, return the coalesced run and arm the cache to re-probe at its
        // end (GNU sets it->stop_charpos to the run end so handle_stop refires).
        for prop in DisplayStopProp::ALL {
            if let Some((width, run_end)) = prop.probe(ctx, buffer_id, byte, column, end)? {
                stop.recompute_at.set(run_end);
                return Ok(Some(DisplayAdvance {
                    next_byte: run_end,
                    width,
                    hard_newline: false,
                    unbreakable_wide: false,
                }));
            }
        }

        // None present here: cache the span over which none can appear so the
        // following chars skip these probes entirely, then fall through to the
        // ordinary per-char width branch.
        stop.recompute_at
            .set(next_display_stop(ctx, buffer_id, byte, end));
    } else {
        // FAST PATH: below the cached stop, no watched property can be present,
        // so skip the probes. The validator (opt-in via the `display-stop-validate`
        // feature, mirroring GNU's MARKER_DEBUG) re-runs every probe and asserts
        // none fire, catching a stale/over-far stop -- but it is OFF by default so
        // ordinary debug/test builds do not pay the per-char probes the cache
        // exists to skip. The DisplayStopProp enum already keeps the probe/key set
        // compiler-consistent, which is the always-on guard.
        #[cfg(feature = "display-stop-validate")]
        {
            let recompute_at = stop.recompute_at.get();
            for prop in DisplayStopProp::ALL {
                debug_assert!(
                    prop.probe(ctx, buffer_id, byte, column, end)?.is_none(),
                    "DisplayStopCache skipped a {} run at byte {byte} < stop {recompute_at}",
                    prop.key(),
                );
            }
        }
    }

    let (code, char_len, width) = {
        let buf = match ctx.buffers.get(buffer_id) {
            Some(buf) => buf,
            None => return Ok(None),
        };
        let scan_pos = EmacsBytePos::new(byte);
        let Some(code) = buf.char_code_after_emacs_byte_pos(scan_pos) else {
            return Ok(None);
        };
        let char_len = buf
            .char_after_emacs_byte_len(scan_pos)
            .map(|len| len.max(EmacsByteLen::new(1)))
            .unwrap_or(EmacsByteLen::new(1));
        let width = buffer_char_display_width(buf, scan_pos, code);
        (code, char_len, width)
    };

    let next_byte = byte.saturating_add(char_len.get()).min(end);
    if code == b'\n' as u32 {
        return Ok(Some(DisplayAdvance {
            next_byte,
            width: 0,
            hard_newline: true,
            unbreakable_wide: false,
        }));
    }

    let next_column = match display_table
        .as_ref()
        .and_then(|dt| display_table_glyph_width(dt, code))
    {
        Some(glyph_width) => column.saturating_add(glyph_width),
        None => next_column_for_code(column, code, width, tab_width),
    };

    Ok(Some(DisplayAdvance {
        next_byte,
        width: next_column.saturating_sub(column),
        hard_newline: false,
        unbreakable_wide: code > 0x7f && width > 1 && char_len.get() > 1,
    }))
}

/// Compute the column width of a `(space ...)` display spec, mirroring GNU's
/// `check_display_width` (src/indent.c) — the column-only subset of the display
/// engine's spec evaluation. `col` is the current column at the spec (needed for
/// `:align-to`); the char-at-pos width factor for `:relative-width` is applied by
/// the caller. Returns the width in canonical columns, or None when the spec
/// carries no width-bearing keyword (in which case GNU lets the underlying
/// character display at its own width).
///
/// GNU's exact precedence (indent.c:506-520):
///   * `:width N` or `:relative-width N` — N a FIXNUM in [0, INT_MAX] -> width N.
///   * a FLOAT `:relative-width` -> round(F).  (A float `:width` is NOT honored:
///     GNU only inspects the *last* `plist_get` result for the float branch, and
///     for `:width` that result was overwritten by the `:relative-width` lookup.)
///   * `:align-to COL` — COL a FIXNUM in [col, col+INT_MAX] -> width COL - col.
///   * a FLOAT `:align-to` in [col, ...] -> round(COL) - col.
fn space_spec_width(plist: Value, col: usize) -> Option<usize> {
    let qcwidth = Value::symbol(":width");
    let qcrel = Value::symbol(":relative-width");
    let qcalign = Value::symbol(":align-to");

    // GNU's `align_to_max` upper bound for `:align-to` is `col + INT_MAX`
    // (indent.c:501-504); `:width`/`:relative-width` use plain `INT_MAX`.
    let int_max = i64::from(i32::MAX);
    let align_to_max = (col as i64).saturating_add(int_max);

    // `:width N` (fixnum), else `:relative-width N` (fixnum). GNU's `||` leaves
    // `prop` holding the `:relative-width` value when `:width` is absent/non-fixnum.
    let width_prop = super::plist::plist_get(plist, &qcwidth).unwrap_or(Value::NIL);
    if let Some(n) = ranged_fixnum(width_prop, 0, int_max) {
        return Some(n as usize);
    }
    let rel_prop = super::plist::plist_get(plist, &qcrel).unwrap_or(Value::NIL);
    if let Some(n) = ranged_fixnum(rel_prop, 0, int_max) {
        return Some(n as usize);
    }
    // Float branch reads the *last* probed value, which is `:relative-width`.
    if let Some(f) = rel_prop.as_float()
        && (0.0..=(i32::MAX as f64)).contains(&f)
    {
        return Some((f + 0.5) as usize);
    }
    // `:align-to COL`: width = COL - col.
    let align_prop = super::plist::plist_get(plist, &qcalign).unwrap_or(Value::NIL);
    if let Some(n) = ranged_fixnum(align_prop, col as i64, align_to_max) {
        return Some((n as usize).saturating_sub(col));
    }
    if let Some(f) = align_prop.as_float()
        && f >= col as f64
        && f <= align_to_max as f64
    {
        return Some(((f + 0.5) as usize).saturating_sub(col));
    }
    None
}

/// GNU `RANGED_FIXNUMP (lo, x, hi)` — `x` is a fixnum in `[lo, hi]`.
fn ranged_fixnum(value: Value, lo: i64, hi: i64) -> Option<i64> {
    let n = value.as_fixnum()?;
    if n >= lo && n <= hi { Some(n) } else { None }
}

/// If buffer position `byte` carries a `display` property (text property OR
/// overlay — GNU consults both via `get_char_property_and_overlay`) whose value
/// replaces the covered text for layout, return `(display_width, run_end_byte)`.
/// Mirrors GNU's `check_display_width` (src/indent.c): a `display` STRING lays
/// out at its `string-width`; a `(space ...)` spec at the width computed by
/// `space_spec_width`. `column` is the current column at `byte`, needed for
/// `(space :align-to ...)`. Image/slice specs are measured by GNU through the
/// display iterator; on TTY/batch frames they fall back to a one-column
/// placeholder while still replacing the whole property range.
fn display_run_at(
    ctx: &super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    byte: usize,
    column: usize,
) -> Option<(usize, usize)> {
    let buf = ctx.buffers.get(buffer_id)?;
    let charpos0 = buf.emacs_byte_pos_to_char_pos_clamped(EmacsBytePos::new(byte));
    let charpos1 = charpos0.get() as i64 + 1;

    // GNU `get_char_property_and_overlay (pos, Qdisplay, ...)`: returns the
    // `display` value and, when it came from an overlay, the overlay itself.
    let (display, overlay) = super::textprop::buffer_overlay_property_at_byte_pos(
        &ctx.obarray,
        &ctx.buffers,
        buf,
        byte,
        Value::symbol("display"),
        None,
    )
    .map(|(v, ov)| (v, Some(ov)))
    .or_else(|| {
        let v = super::textprop::builtin_get_text_property_in_state(
            &ctx.obarray,
            &ctx.buffers,
            vec![Value::fixnum(charpos1), Value::symbol("display")],
        )
        .ok()?;
        if v.is_nil() { None } else { Some((v, None)) }
    })?;

    let is_space_spec = display.is_cons() && display.cons_car() == Value::symbol("space");

    // Compute the spec's column width (GNU `check_display_width`'s `width`).
    let mut width = if let Some(disp_str) = display.as_lisp_string() {
        // `display` STRING -> its display columns.
        lisp_string_display_columns(disp_str)
    } else if is_space_spec {
        // `(space ...)` spec -> evaluate the `:width`/`:relative-width`/`:align-to`
        // keywords. A spec with no width-bearing keyword leaves the char at its
        // own width.
        space_spec_width(display.cons_cdr(), column)?
    } else if display_image_or_slice_spec_p(display) {
        // GNU `check_display_width` measures image/slice display specs via the
        // display iterator. On a TTY/batch frame, bare image and slice specs
        // render as a single-column placeholder.
        1
    } else {
        // other display specs do not contribute a computable column width here.
        return None;
    };

    // `:relative-width` is multiplied by the column width of the covered char
    // (GNU multiplies by `MULTIBYTE_BYTES_WIDTH` of the char at POS).
    if is_space_spec
        && super::plist::plist_get(display.cons_cdr(), &Value::symbol(":relative-width"))
            .is_some_and(|v| !v.is_nil())
    {
        let scan_pos = EmacsBytePos::new(byte);
        if let Some(code) = buf.char_code_after_emacs_byte_pos(scan_pos) {
            let char_w = buffer_char_display_width(buf, scan_pos, code);
            width = width.saturating_mul(char_w);
        }
    }

    // End of the run: overlay-end for overlay `display`, else the text-property
    // range end (GNU `OVERLAY_END` vs `get_property_and_range`).
    let run_end_byte = if let Some(ov) = overlay {
        buf.overlays
            .overlay_end_emacs_byte_pos(ov)
            .map(|p| p.get())
            .unwrap_or_else(|| buf.accessible_emacs_byte_region().end().get())
    } else {
        let run_end_char1 = super::textprop::builtin_next_single_property_change_in_state(
            &ctx.obarray,
            &ctx.buffers,
            vec![Value::fixnum(charpos1), Value::symbol("display")],
        )
        .ok()
        .and_then(|v| match v.kind() {
            ValueKind::Fixnum(n) => Some(n),
            _ => None,
        })
        .unwrap_or_else(|| buf.accessible_char_region().end().get() as i64 + 1);
        buf.char_pos_to_emacs_byte_pos_clamped(CharPos0::new((run_end_char1 - 1).max(0) as usize))
            .get()
    };
    Some((width, run_end_byte))
}

fn display_image_or_slice_spec_p(spec: Value) -> bool {
    if !spec.is_cons() {
        return false;
    }
    let car = spec.cons_car();
    car.is_symbol_named("image") || car.is_symbol_named("slice")
}

/// If display table `dt` remaps character `code` to a glyph vector, return the
/// total display width of that glyph sequence (each glyph's character at its
/// own width). Returns None when there is no glyph-vector entry for `code`.
fn display_table_glyph_width(dt: &Value, code: u32) -> Option<usize> {
    // POINT lookup, not `char_table_ref_and_range`: this only needs the glyph
    // vector at `code`, never the surrounding range. The range variant
    // (`ct_effective_value_span_at`, extended cell-by-cell) was ~15% of Doom
    // scroll CPU here -- pure waste, since the span it computes is discarded.
    // `ct_ref` resolves the identical effective value (same local/default/
    // parent descent) and adds an ASCII fast-cache the range variant lacks.
    let entry = super::chartable::ct_ref(dt, i64::from(code));
    let glyphs = entry.as_vector_data()?;
    let mut total = 0usize;
    for glyph in glyphs.iter() {
        let w = match glyph.kind() {
            // A glyph code packs the character in the low 22 bits (face above).
            ValueKind::Fixnum(n) => char::from_u32((n & 0x3F_FFFF) as u32)
                .map(crate::encoding::char_width)
                .unwrap_or(1),
            _ => 1,
        };
        total += w;
    }
    Some(total)
}

/// If a `composition` property begins at buffer byte `byte`, return
/// `(composed_width, run_end_byte)` — the composed glyphs' display width and the
/// byte position just past the composed characters. Returns None otherwise.
fn composition_run_at(
    ctx: &super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    byte: usize,
) -> Option<(usize, usize)> {
    let buf = ctx.buffers.get(buffer_id)?;
    let charpos0 = buf.emacs_byte_pos_to_char_pos_clamped(EmacsBytePos::new(byte));
    let charpos1 = charpos0.get() as i64 + 1;
    let (width, length) = super::composite::composition_width_at(ctx, charpos1)?;
    let end_charpos0 = charpos0.get() + length.max(0) as usize;
    let end_byte = buf
        .char_pos_to_emacs_byte_pos_clamped(CharPos0::new(end_charpos0))
        .get();
    Some((width.max(0) as usize, end_byte))
}

/// Total display columns of a Lisp string (sum of per-character display widths).
fn lisp_string_display_columns(text: &LispString) -> usize {
    let mut total = 0usize;
    if text.is_multibyte() {
        let bytes = text.as_bytes();
        let mut pos = 0usize;
        while pos < bytes.len() {
            let (code, len) = crate::emacs_core::emacs_char::string_char(&bytes[pos..]);
            total += char::from_u32(code)
                .map(crate::encoding::char_width)
                .unwrap_or(1);
            pos += len;
        }
    } else {
        for &b in text.as_bytes() {
            total += raw_unibyte_display_width(b);
        }
    }
    total
}

fn buffer_char_display_width(buf: &Buffer, byte_pos: EmacsBytePos, code: u32) -> usize {
    if !buf.get_multibyte() {
        return buf
            .emacs_byte_at_pos(byte_pos)
            .map(raw_unibyte_display_width)
            .unwrap_or(1);
    }
    if crate::emacs_core::emacs_char::char_byte8_p(code) {
        4
    } else if let Some(ch) = char::from_u32(code) {
        crate::encoding::char_width(ch)
    } else {
        1
    }
}

fn current_buffer_line_bounds(
    ctx: &super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    point: EmacsBytePos,
) -> Result<EmacsByteRange, Flow> {
    let buf = ctx
        .buffers
        .get(buffer_id)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    Ok(line_bounds(buf, point))
}

/// Byte position of the next change of char-property `prop` (text property
/// or overlay) strictly after `byte`, clamped to `limit`.  GNU's
/// `scan_for_column` re-probes `invisible` only when `scan == next_boundary`
/// (`skip_invisible`, indent.c); neomacs extends the same memo to the
/// `display` and `composition` probes, so a property-free line costs one
/// boundary lookup per property instead of three full lookups per character.
fn next_char_property_boundary_byte(
    ctx: &super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    byte: usize,
    prop: Value,
    limit: usize,
) -> usize {
    let Some(buf) = ctx.buffers.get(buffer_id) else {
        return limit;
    };
    // GNU next-single-char-property-change = the nearer of the text
    // property's next change and the next overlay boundary; both stores are
    // byte-keyed, so no char<->byte round trips (they dominated the first
    // cut of this memo).
    let limit_pos = EmacsBytePos::new(limit);
    let pos = EmacsBytePos::new(byte);
    let text_next = buf
        .text_props_next_single_change_after_emacs_byte_pos_bounded(pos, prop, limit_pos)
        .map(|p| p.get())
        .unwrap_or(limit);
    let overlay_next = buf
        .overlays
        .next_boundary_after_until_emacs_byte_pos(pos, limit_pos)
        .map(|p| p.get())
        .unwrap_or(limit);
    let next = text_next.min(overlay_next).min(limit);
    if next > byte { next } else { limit }
}

fn scan_for_column(
    ctx: &mut super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    end_byte: Option<EmacsBytePos>,
    goal_column: Option<usize>,
) -> Result<ColumnScan, Flow> {
    let (mut scan, line_end, tab_width) = {
        let buf = ctx
            .buffers
            .get(buffer_id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        // Anchor the line at the target byte position when one is given so the
        // column is measured from *that* position's beginning-of-line, not the
        // buffer's current point.  For the two in-buffer callers
        // (`current-column`, `move-to-column`) `end_byte`, when `Some`, is point
        // itself, so this is behavior-preserving; it additionally lets callers
        // (auto-hscroll) measure the column at an arbitrary position such as a
        // non-selected window's `pointm`.
        let anchor = end_byte.unwrap_or_else(|| buf.point_emacs_byte_pos());
        let line = line_bounds(buf, anchor);
        (
            line.start().get(),
            line.end().get(),
            tab_width_in_state(&ctx.obarray, &[], Some(buf)),
        )
    };
    let end = end_byte
        .map(|pos| pos.get())
        .unwrap_or(line_end)
        .min(line_end);
    let goal = goal_column.unwrap_or(usize::MAX);
    let mut column = 0usize;
    let mut previous_byte_pos = scan;
    let mut previous_column = 0usize;
    let mut previous_code = None;

    // The active display table (buffer-display-table, else standard-display-table)
    // remaps individual characters to glyph sequences; consulted per char below.
    let display_table = {
        let buf = ctx.buffers.get(buffer_id);
        dynamic_buffer_or_global_symbol_value(&ctx.obarray, &[], buf, "buffer-display-table")
            .filter(|v| !v.is_nil())
            .or_else(|| ctx.obarray.symbol_value("standard-display-table").copied())
            .filter(|v| !v.is_nil())
    };

    // Property probes are re-run only at the next change boundary of their
    // property (GNU `skip_invisible`'s `next_boundary`, extended to all
    // three); between boundaries the answer cannot change.
    let invisible_sym = Value::symbol("invisible");
    let display_sym = Value::symbol("display");
    let composition_sym = Value::symbol("composition");
    let mut next_invisible_probe = scan;
    let mut next_display_probe = scan;
    let mut next_composition_probe = scan;

    while scan < end {
        if scan >= next_invisible_probe {
            if let Some(next_visible) =
                super::xdisp::zero_width_invisible_run_end_byte(ctx, buffer_id, scan)?
                && next_visible > scan
            {
                scan = next_visible.min(end);
                if scan >= end {
                    break;
                }
                next_invisible_probe = scan;
                continue;
            }
            next_invisible_probe =
                next_char_property_boundary_byte(ctx, buffer_id, scan, invisible_sym, end);
        }

        if column >= goal {
            break;
        }

        // A `display` property (text property or overlay) whose value is a string
        // or a `(space ...)` spec replaces the covered text for layout (GNU's
        // `current_column_1` / `Fmove_to_column` consult `display` specs via
        // `check_display_width`). Advance by the spec's display width over the
        // whole property/overlay run, atomically (no splitting a display run).
        if scan >= next_display_probe {
            if let Some((disp_width, run_end_byte)) = display_run_at(ctx, buffer_id, scan, column)
                && run_end_byte > scan
            {
                previous_byte_pos = scan;
                previous_column = column;
                previous_code = None;
                column = column.saturating_add(disp_width);
                scan = run_end_byte.min(end);
                next_display_probe = scan;
                continue;
            }
            next_display_probe =
                next_char_property_boundary_byte(ctx, buffer_id, scan, display_sym, end);
        }

        // A `composition` property lays its covered characters out as the
        // composed glyphs (GNU's display scan via get_composition_id), so the
        // run advances by the glyphs' width over the composed character count.
        if scan >= next_composition_probe {
            if let Some((comp_width, comp_end)) = composition_run_at(ctx, buffer_id, scan)
                && comp_end > scan
            {
                previous_byte_pos = scan;
                previous_column = column;
                previous_code = None;
                column = column.saturating_add(comp_width);
                scan = comp_end.min(end);
                next_composition_probe = scan;
                continue;
            }
            next_composition_probe =
                next_char_property_boundary_byte(ctx, buffer_id, scan, composition_sym, end);
        }

        let (code, char_len, width) = {
            let buf = ctx
                .buffers
                .get(buffer_id)
                .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
            let scan_pos = EmacsBytePos::new(scan);
            let Some(code) = buf.char_code_after_emacs_byte_pos(scan_pos) else {
                break;
            };
            let char_len = buf
                .char_after_emacs_byte_len(scan_pos)
                .map(|len| len.max(EmacsByteLen::new(1)))
                .unwrap_or(EmacsByteLen::new(1));
            let width = buffer_char_display_width(buf, scan_pos, code);
            (code, char_len, width)
        };

        if code == b'\n' as u32 {
            break;
        }

        previous_byte_pos = scan;
        previous_column = column;
        previous_code = Some(code);
        // A display-table entry remaps the character to a glyph sequence,
        // overriding its normal width (and tab expansion).
        column = match display_table
            .as_ref()
            .and_then(|dt| display_table_glyph_width(dt, code))
        {
            Some(glyph_width) => column.saturating_add(glyph_width),
            None => next_column_for_code(column, code, width, tab_width),
        };
        scan += char_len.get();
    }

    Ok(ColumnScan {
        byte_pos: EmacsBytePos::new(scan),
        column,
        previous_byte_pos: EmacsBytePos::new(previous_byte_pos),
        previous_column,
        previous_code,
    })
}

fn decode_lisp_string_units(text: &LispString) -> Vec<DecodedUnit> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    if text.is_multibyte() {
        let mut pos = 0usize;
        while pos < bytes.len() {
            let start = pos;
            let (code, len) = crate::emacs_core::emacs_char::string_char(&bytes[pos..]);
            pos += len;
            let width = if crate::emacs_core::emacs_char::char_byte8_p(code) {
                4
            } else if let Some(ch) = char::from_u32(code) {
                crate::encoding::char_width(ch)
            } else {
                1
            };
            out.push(DecodedUnit {
                start,
                end: pos,
                code,
                width,
            });
        }
        return out;
    }

    for (idx, &byte) in bytes.iter().enumerate() {
        let width = if byte < 0x80 {
            crate::encoding::char_width(byte as char)
        } else {
            4
        };
        out.push(DecodedUnit {
            start: idx,
            end: idx + 1,
            code: byte as u32,
            width,
        });
    }
    out
}

fn column_for_lisp_string(prefix: &LispString, tab_width: usize) -> usize {
    let mut column = 0usize;
    for unit in decode_lisp_string_units(prefix) {
        column = next_column_for_code(column, unit.code, unit.width, tab_width);
    }
    column
}

fn spaces_to_column(column: usize, target: usize) -> String {
    " ".repeat(target.saturating_sub(column))
}

/// Insert indentation with GNU `Finsert_char(..., INHERIT=t)` semantics.
///
/// Keeping the property policy out of the signature makes it impossible for
/// an indentation caller to select plain insertion accidentally.  The closed
/// `InsertPiecePropertyMode` choice remains explicit at the lower insertion
/// boundary, independently of marker placement.
fn insert_inheriting_indentation(
    ctx: &mut crate::emacs_core::eval::Context,
    indentation: String,
) -> Result<(), Flow> {
    if indentation.is_empty() {
        return Ok(());
    }
    debug_assert!(indentation.bytes().all(|byte| matches!(byte, b' ' | b'\t')));

    let current_id = ctx
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let insert_pos = ctx
        .buffers
        .get(current_id)
        .map(Buffer::point_emacs_byte_pos)
        .unwrap_or(EmacsBytePos::ZERO);
    let indentation_len = indentation.len();
    let change = super::editfns::text_change_for_empty_insertion_at_emacs_byte_pos(
        &ctx.buffers,
        current_id,
        insert_pos,
        TextExtent::new(
            CharLen::new(indentation_len),
            EmacsByteLen::new(indentation_len),
        ),
    )?;
    super::editfns::signal_before_text_change(ctx, change)?;
    super::builtins::insert_string_value_in_current_buffer(
        &ctx.obarray,
        &[],
        &mut ctx.buffers,
        Value::string(indentation),
        super::builtins::InsertPieceMarkerPlacement::AfterMarkers,
        super::builtins::InsertPiecePropertyMode::InheritAdjoining,
    )?;
    super::editfns::signal_after_text_change(ctx, change)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared-runtime indentation builtins
// ---------------------------------------------------------------------------

/// (current-indentation) -> integer
///
/// Return indentation columns for the current line.
pub(crate) fn builtin_current_indentation(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("current-indentation", &args, 0)?;
    let Some(buf) = &ctx.buffers.current_buffer() else {
        return Ok(Value::fixnum(0));
    };

    let tabw = tab_width_in_state(&ctx.obarray, &[], Some(buf));
    let line_range = line_bounds(buf, buf.point_emacs_byte_pos());
    let line = buf.buffer_substring_lisp_string_range(line_range);

    let mut column = 0usize;
    for unit in decode_lisp_string_units(&line) {
        if unit.code == b' ' as u32 || unit.code == b'\t' as u32 {
            column = next_column_for_code(column, unit.code, unit.width, tabw);
        } else {
            break;
        }
    }

    Ok(Value::fixnum(column as i64))
}

/// (current-column) -> integer
///
/// Return the display column at point on the current line.
pub(crate) fn builtin_current_column(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("current-column", &args, 0)?;
    let Some(current_id) = ctx.buffers.current_buffer_id() else {
        return Ok(Value::fixnum(0));
    };
    let (point, modiff) = {
        let Some(buf) = ctx.buffers.get(current_id) else {
            return Ok(Value::fixnum(0));
        };
        (
            buf.accessible_emacs_byte_region()
                .clamp(buf.point_emacs_byte_pos()),
            buf.modified_tick(),
        )
    };
    // GNU `Fcurrent_column`/`current_column' (src/indent.c:298-342) returns the
    // cached `last_known_column' when point and MODIFF are unchanged, so the
    // column reflects the `tab-width' in effect when it was last computed.
    if let Some(column) = cached_current_column(current_id.0, point, modiff) {
        return Ok(Value::fixnum(column as i64));
    }
    let scan = scan_for_column(ctx, current_id, Some(point), None)?;
    set_last_known_column(current_id.0, point, modiff, scan.column);
    Ok(Value::fixnum(scan.column as i64))
}

/// Display column (`current-column`-equivalent) of an explicit byte position in
/// a buffer, measured from that position's beginning-of-line.
///
/// Unlike `current-column`, this does not consult the current buffer or point:
/// it is the column primitive auto-hscroll (`hscroll`) needs to follow a
/// window's `pointm`, which for a non-selected window may differ from the
/// buffer's `pt`.  Tab- and char-width-aware, honoring `tab-width`, display
/// tables, `display`/`composition` properties, and invisibility, exactly as
/// `current-column` does (it shares `scan_for_column`).
pub(crate) fn display_column_at_emacs_byte_pos(
    ctx: &mut crate::emacs_core::eval::Context,
    buffer_id: crate::buffer::BufferId,
    pos: EmacsBytePos,
) -> Result<usize, Flow> {
    let clamped = {
        let Some(buf) = ctx.buffers.get(buffer_id) else {
            return Ok(0);
        };
        buf.accessible_emacs_byte_region().clamp(pos)
    };
    let scan = scan_for_column(ctx, buffer_id, Some(clamped), None)?;
    Ok(scan.column)
}

/// (move-to-column COLUMN &optional FORCE) -> COLUMN-REACHED
///
/// Move point on the current line according to display columns.
pub(crate) fn builtin_move_to_column(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("move-to-column", &args, 1)?;
    expect_max_args("move-to-column", &args, 2)?;
    let target = expect_wholenump(&args[0])?;
    let force_arg = args.get(1).copied().unwrap_or(Value::NIL);
    let force_non_nil = force_arg.is_truthy();
    let force_is_t = force_arg == Value::T;
    let Some(current_id) = ctx.buffers.current_buffer_id() else {
        return Ok(Value::fixnum(0));
    };
    let Some(buf) = ctx.buffers.get(current_id) else {
        return Ok(Value::fixnum(0));
    };
    let read_only = super::editfns::buffer_read_only_active_in_state(&ctx.obarray, &[], buf);
    let pt = buf
        .accessible_emacs_byte_region()
        .clamp(buf.point_emacs_byte_pos());
    let buffer_name = buf.name_value();

    if target == 0 {
        let line = current_buffer_line_bounds(ctx, current_id, pt)?;
        let _ = ctx
            .buffers
            .goto_buffer_emacs_byte_pos(current_id, line.start());
        return Ok(Value::fixnum(0));
    }

    let mut tab_split: Option<(EmacsBytePos, usize, usize)> = None;
    let scan = scan_for_column(ctx, current_id, None, Some(target))?;
    let dest_byte = scan.byte_pos;
    let mut reached = scan.column;

    if force_non_nil
        && scan.column > target
        && scan.previous_column < target
        && scan.previous_code == Some(b'\t' as u32)
        && scan.previous_byte_pos < scan.byte_pos
    {
        tab_split = Some((scan.previous_byte_pos, scan.previous_column, scan.column));
    }

    if let Some((tab_byte, col_before_tab, col_after_tab)) = tab_split {
        if read_only {
            return Err(signal(LispCondition::BufferReadOnly, vec![buffer_name]));
        }
        let _ = ctx.buffers.goto_buffer_emacs_byte_pos(current_id, tab_byte);
        let pad = spaces_to_column(col_before_tab, target);
        let insert_pos = tab_byte;
        let pad_len = pad.len();
        insert_inheriting_indentation(ctx, pad)?;
        let tab_after_pad = insert_pos.add_len(EmacsByteLen::new(pad_len));
        let delete_range = super::editfns::buffer_edit_range_for_byte_range_in_manager(
            &ctx.buffers,
            current_id,
            EmacsByteRange::from_start_len(tab_after_pad, EmacsByteLen::new(1)),
        )?;
        let delete_change = crate::buffer::TextChange::deletion(delete_range);
        super::editfns::signal_before_text_change(ctx, delete_change)?;
        let _ = ctx
            .buffers
            .delete_buffer_measured_region(current_id, delete_range);
        super::editfns::signal_after_text_change(ctx, delete_change)?;
        let goal_point = tab_after_pad;
        let _ = ctx
            .buffers
            .goto_buffer_emacs_byte_pos(current_id, goal_point);
        let _ = builtin_indent_to(ctx, vec![Value::fixnum(col_after_tab as i64), Value::NIL])?;
        let _ = ctx
            .buffers
            .goto_buffer_emacs_byte_pos(current_id, goal_point);
        return Ok(Value::fixnum(target as i64));
    }

    let _ = ctx
        .buffers
        .goto_buffer_emacs_byte_pos(current_id, dest_byte);

    if force_is_t && reached < target {
        if read_only {
            return Err(signal(LispCondition::BufferReadOnly, vec![buffer_name]));
        }
        // GNU Fmove_to_column delegates short-line padding to Findent_to
        // (src/indent.c:1176-1177), keeping indentation construction and
        // inheriting insertion in one path.
        let _ = builtin_indent_to(ctx, vec![Value::fixnum(target as i64), Value::NIL])?;
        reached = target;
    }

    Ok(Value::fixnum(reached as i64))
}

/// (indent-to COLUMN &optional MINIMUM) -> COLUMN
///
/// GNU Emacs `Findent_to` primitive from `src/indent.c`.
pub(crate) fn builtin_indent_to(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("indent-to", &args, 1)?;
    expect_max_args("indent-to", &args, 2)?;
    let column = expect_fixnump(&args[0])?.max(0) as usize;
    let minimum = if args.len() > 1 && !args[1].is_nil() {
        expect_fixnump(&args[1])?.max(0) as usize
    } else {
        0
    };

    let current_id = ctx
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let buf = ctx
        .buffers
        .current_buffer()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

    let pt = buf.point_emacs_byte_pos();
    let line = line_bounds(buf, pt);
    let line_prefix = buf.buffer_substring_lisp_string_range(EmacsByteRange::new(line.start(), pt));
    let tab_width = tab_width_in_state(&ctx.obarray, &[], Some(buf));

    let fromcol = column_for_lisp_string(&line_prefix, tab_width);

    let mincol = column.max(fromcol + minimum);
    if fromcol >= mincol {
        return Ok(Value::fixnum(mincol as i64));
    }

    if super::editfns::buffer_read_only_active_in_state(&ctx.obarray, &[], buf) {
        return Err(signal(
            LispCondition::BufferReadOnly,
            vec![buf.name_value()],
        ));
    }

    let use_tabs = indent_tabs_mode_in_state(&ctx.obarray, &[], Some(buf));

    let mut indent = String::new();
    let mut col = fromcol;

    if use_tabs {
        let tab = tab_width.max(1);
        while col < mincol {
            let next_tab = col + (tab - (col % tab));
            if next_tab <= mincol {
                indent.push('\t');
                col = next_tab;
            } else {
                break;
            }
        }
    }

    while col < mincol {
        indent.push(' ');
        col += 1;
    }

    insert_inheriting_indentation(ctx, indent)?;

    // GNU `Findent_to` caches the resulting column at the new point/MODIFF so a
    // following `current-column' returns it without rescanning (src/indent.c:
    // 831-833).  Record it for the same reason here.
    if let Some(buf) = ctx.buffers.get(current_id) {
        set_last_known_column(
            current_id.0,
            buf.point_emacs_byte_pos(),
            buf.modified_tick(),
            mincol,
        );
    }

    Ok(Value::fixnum(mincol as i64))
}

// ---------------------------------------------------------------------------
// Variable initialisation
// ---------------------------------------------------------------------------

/// Pre-populate the obarray with standard indentation variables.
///
/// Must be called during evaluator initialisation (after the obarray is created
/// but before any user code runs).
pub fn init_indent_vars(obarray: &mut super::symbol::Obarray) {
    // tab-width: default 8 (buffer-local in real Emacs, global default here)
    obarray.set_symbol_value("tab-width", Value::fixnum(8));
    obarray.make_special("tab-width");

    // standard-indent: default 4
    obarray.set_symbol_value("standard-indent", Value::fixnum(4));
    obarray.make_special("standard-indent");

    // tab-stop-list: default nil
    obarray.set_symbol_value("tab-stop-list", Value::NIL);
    obarray.make_special("tab-stop-list");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

pub(crate) fn builtin_compute_motion(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let obarray = &eval.obarray;
    let buffers = &eval.buffers;
    expect_args("compute-motion", &args, 7)?;

    let from = crate::emacs_core::position::fix_position_with_buffers(buffers, &args[0])?;
    if !args[1].is_cons() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("consp"), args[1]],
        ));
    }
    let to = crate::emacs_core::position::fix_position_with_buffers(buffers, &args[2])?;
    if !args[3].is_nil() && !args[3].is_cons() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("consp"), args[3]],
        ));
    }
    if !args[4].is_nil() {
        let _ = expect_fixnum(&args[4])?;
    }
    if !args[5].is_nil() && !args[5].is_cons() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("consp"), args[5]],
        ));
    }
    if !args[6].is_nil() && !args[6].is_window() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("window-live-p"), args[6]],
        ));
    }

    // Extract FROMPOS (HPOS . VPOS).
    let (from_hpos, from_vpos) = extract_cons_fixnums(args[1])?;

    // Extract TOPOS (HPOS . VPOS) or nil.
    let (to_hpos, to_vpos) = if args[3].is_nil() {
        (i64::MAX, i64::MAX)
    } else {
        extract_cons_fixnums(args[3])?
    };

    // Extract WIDTH.
    let width = if args[4].is_nil() {
        80i64 // default window width
    } else {
        expect_fixnum(&args[4])?
    };
    if !args[5].is_nil() {
        let (hscroll, tab_offset) = extract_cons_fixnums(args[5])?;
        if hscroll < 0 || tab_offset < 0 || tab_offset > i32::MAX as i64 {
            return Err(signal(
                LispCondition::ArgsOutOfRange,
                vec![args[5].cons_car(), args[5].cons_cdr()],
            ));
        }
    }

    // Get buffer text.
    let Some(buf) = buffers.current_buffer() else {
        return Ok(Value::list(vec![
            Value::fixnum(from),
            Value::fixnum(from_hpos),
            Value::fixnum(from_vpos),
            Value::fixnum(0),
            Value::NIL,
        ]));
    };
    let text = buf.full_text_string();
    let accessible = buf.accessible_emacs_byte_region();
    let tab_width = crate::buffer::buffer::lookup_buffer_slot("tab-width")
        .map(|info| buf.slots[info.offset.index()])
        .or_else(|| buf.get_buffer_local("tab-width"))
        .or_else(|| obarray.symbol_value("tab-width").copied())
        .and_then(|value: Value| match value.kind() {
            ValueKind::Fixnum(n) if n > 0 => Some(n as usize),
            _ => None,
        })
        .unwrap_or(8);

    // Convert 1-based char positions to byte offsets.
    let point_min = buf.point_min_lisp_char_pos().as_i64();
    let point_max = buf.point_max_lisp_char_pos().as_i64();
    if from < point_min || from > point_max {
        return Err(signal(
            LispCondition::ArgsOutOfRange,
            vec![
                Value::fixnum(from),
                Value::fixnum(point_min),
                Value::fixnum(point_max),
            ],
        ));
    }
    if to < point_min || to > point_max {
        return Err(signal(
            LispCondition::ArgsOutOfRange,
            vec![
                Value::fixnum(to),
                Value::fixnum(point_min),
                Value::fixnum(point_max),
            ],
        ));
    }

    let from_pos = buf
        .lisp_pos_to_accessible_emacs_byte_pos(LispCharPos1::new(from))
        .get();
    let to_pos = buf
        .lisp_pos_to_accessible_emacs_byte_pos(LispCharPos1::new(to))
        .get();

    let mut hpos = from_hpos;
    let mut vpos = from_vpos;
    let mut prev_hpos = from_hpos;
    let mut contin = false;
    let mut pos = from_pos;

    let bytes = text.as_bytes();
    let tw = tab_width.max(1) as i64;

    while pos < to_pos {
        // Check TOPOS stop condition.
        if vpos > to_vpos || (vpos == to_vpos && hpos >= to_hpos) {
            break;
        }

        prev_hpos = hpos;
        let ch = if pos < bytes.len() {
            // Decode UTF-8 character.
            let b = bytes[pos];
            if b < 0x80 {
                pos += 1;
                b as char
            } else {
                let s = &text[pos..];
                let c = s.chars().next().unwrap_or('\u{FFFD}');
                pos += c.len_utf8();
                c
            }
        } else {
            break;
        };

        match ch {
            '\n' => {
                vpos += 1;
                hpos = 0;
                contin = false;
            }
            '\t' => {
                hpos += tw - (hpos % tw);
            }
            _ => {
                hpos += crate::encoding::char_width(ch) as i64;
            }
        }

        // Line continuation (wrapping).
        if hpos >= width && ch != '\n' {
            vpos += 1;
            contin = true;
            hpos -= width;
        }
    }

    // Convert byte pos back to 1-based char position.
    let final_charpos = point_char_pos(buf, EmacsBytePos::new(pos).min(accessible.end()));

    Ok(Value::list(vec![
        Value::fixnum(final_charpos),
        Value::fixnum(hpos),
        Value::fixnum(vpos),
        Value::fixnum(prev_hpos),
        if contin { Value::T } else { Value::NIL },
    ]))
}

/// (line-number-display-width &optional ON-DISPLAY) -> integer
///
/// Return the selected window's line-number width.  GNU returns the digit
/// width without the two display padding columns when ON-DISPLAY is nil; with
/// non-nil ON-DISPLAY it returns the actual displayed gutter width in pixels,
/// except the symbol `columns' returns that gutter in canonical columns.
pub(crate) fn builtin_line_number_display_width(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("line-number-display-width", &args, 0, 1)?;
    let Some(frame) = eval.frames.selected_frame() else {
        return Ok(Value::fixnum(0));
    };
    let wid = frame.selected_window;
    let Some(window) = frame.find_window(wid) else {
        return Ok(Value::fixnum(0));
    };
    let Some(buffer_id) = window.buffer_id() else {
        return Ok(Value::fixnum(0));
    };
    let Some(buffer) = eval.buffers.get(buffer_id) else {
        return Ok(Value::fixnum(0));
    };
    let enabled = buffer
        .buffer_local_value("display-line-numbers")
        .is_some_and(|value| value.is_truthy());
    if !enabled {
        return Ok(Value::fixnum(0));
    }

    let char_width = frame.char_width.max(1.0).round() as i64;
    let char_height = frame.char_height.max(1.0).round() as i64;
    let visible_lines = match window {
        Window::Leaf { bounds, .. } => {
            ((bounds.height / char_height.max(1) as f32).floor() as i64).max(1)
        }
        Window::Internal { .. } => 1,
    };
    let digit_width = line_number_digit_width(buffer, visible_lines);
    let displayed_columns = digit_width + 2;

    match args.first() {
        Some(value) if value == &Value::symbol("columns") => {
            Ok(Value::make_float(displayed_columns as f64))
        }
        Some(value) if value.is_truthy() => Ok(Value::fixnum(displayed_columns * char_width)),
        _ => Ok(Value::fixnum(digit_width)),
    }
}

#[cfg(test)]
#[path = "indent_test.rs"]
mod tests;
