//! Indentation builtins for the Elisp interpreter.
//!
//! Implements stub versions of Emacs indentation primitives:
//! - `current-indentation`, `indent-to`, `current-column`, `move-to-column`
//! - `indent-line-to`, `indent-rigidly`, `newline-and-indent`,
//!   `tab-to-tab-stop`, `delete-indentation`
//!
//! Variables: `tab-width`, `indent-tabs-mode`, `standard-indent`, `tab-stop-list`

use super::error::{EvalResult, Flow, signal};
use super::intern::intern;
use super::symbol::Obarray;
use super::value::*;
use crate::buffer::{
    Buffer, BufferManager, CharLen, CharPos0, EmacsByteLen, EmacsBytePos, EmacsByteRange,
    TextExtent,
};
use crate::emacs_core::value::ValueKind;
use crate::heap_types::LispString;

// ---------------------------------------------------------------------------
// Argument helpers (local to this module)
// ---------------------------------------------------------------------------

fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_min_args(name: &str, args: &[Value], min: usize) -> Result<(), Flow> {
    if args.len() < min {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_max_args(name: &str, args: &[Value], max: usize) -> Result<(), Flow> {
    if args.len() > max {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_fixnump(val: &Value) -> Result<i64, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("fixnump"), *val],
        )),
    }
}

fn expect_wholenump(val: &Value) -> Result<usize, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) if n >= 0 => Ok(n as usize),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("wholenump"), *val],
        )),
    }
}

fn dynamic_buffer_or_global_symbol_value(
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

fn indent_tabs_mode_in_state(
    obarray: &Obarray,
    dynamic: &[OrderedRuntimeBindingMap],
    buf: Option<&Buffer>,
) -> bool {
    dynamic_buffer_or_global_symbol_value(obarray, dynamic, buf, "indent-tabs-mode")
        .is_none_or(|value| value.is_truthy())
}

fn buffer_read_only_active(eval: &super::eval::Context, buf: &Buffer) -> bool {
    super::editfns::buffer_read_only_active_in_state(&eval.obarray, &[], buf)
}

#[derive(Clone, Copy)]
struct DecodedUnit {
    start: usize,
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
    if byte < 0o40 || byte >= 0o177 { 4 } else { 1 }
}

/// If the buffer position `byte` carries a `display` text property whose value
/// is a string, return `(display_width, run_end_byte)` where `display_width` is
/// the string's total display columns and `run_end_byte` is the end of the
/// `display`-property run. Used by the column engine so display-string text
/// lays out at the replacement string's width, not the underlying text's.
fn display_string_run_at(
    ctx: &super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    byte: usize,
) -> Option<(usize, usize)> {
    let buf = ctx.buffers.get(buffer_id)?;
    let charpos0 = buf.emacs_byte_pos_to_char_pos_clamped(EmacsBytePos::new(byte));
    let charpos1 = charpos0.get() as i64 + 1;
    let display = super::textprop::builtin_get_text_property_in_state(
        &ctx.obarray,
        &ctx.buffers,
        vec![Value::fixnum(charpos1), Value::symbol("display")],
    )
    .ok()?;
    let disp_str = display.as_lisp_string()?;
    let width: usize = lisp_string_display_columns(disp_str);
    // End of the `display`-property run (nil result means end of accessible text).
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
    let run_end_byte = buf
        .char_pos_to_emacs_byte_pos_clamped(CharPos0::new((run_end_char1 - 1).max(0) as usize))
        .get();
    Some((width, run_end_byte))
}

/// If display table `dt` remaps character `code` to a glyph vector, return the
/// total display width of that glyph sequence (each glyph's character at its
/// own width). Returns None when there is no glyph-vector entry for `code`.
fn display_table_glyph_width(dt: &Value, code: u32) -> Option<usize> {
    let entry = super::chartable::char_table_ref_and_range(dt, i64::from(code))
        .ok()?
        .0;
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
        let line = line_bounds(buf, buf.point_emacs_byte_pos());
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

    while scan < end {
        if let Some(next_visible) =
            super::xdisp::zero_width_invisible_run_end_byte(ctx, buffer_id, scan)?
        {
            if next_visible > scan {
                scan = next_visible.min(end);
                if scan >= end {
                    break;
                }
                continue;
            }
        }

        if column >= goal {
            break;
        }

        // A `display` text property whose value is a string replaces the covered
        // text with that string for layout (GNU's `current_column_1` /
        // `Fmove_to_column` consult `display` specs). Advance by the string's
        // display width over the whole property run, atomically.
        if let Some((disp_width, run_end_byte)) = display_string_run_at(ctx, buffer_id, scan) {
            if run_end_byte > scan {
                previous_byte_pos = scan;
                previous_column = column;
                previous_code = None;
                column = column.saturating_add(disp_width);
                scan = run_end_byte.min(end);
                continue;
            }
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

fn column_for_prefix(prefix: &str, tab_width: usize) -> usize {
    let mut column = 0usize;
    for ch in prefix.chars() {
        column = next_column(column, ch, tab_width);
    }
    column
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

fn indent_to_column_string(
    mut column: usize,
    target: usize,
    tab_width: usize,
    indent_tabs_mode: bool,
) -> String {
    let mut out = String::new();
    let tab = tab_width.max(1);

    if indent_tabs_mode {
        let ntabs = target / tab - column / tab;
        for _ in 0..ntabs {
            out.push('\t');
        }
        if ntabs > 0 {
            column = (target / tab) * tab;
        }
    }

    while column < target {
        out.push(' ');
        column += 1;
    }

    out
}

#[inline]
fn is_horizontal_space(ch: char) -> bool {
    ch == ' ' || ch == '\t'
}

fn delete_horizontal_space_at_point(
    eval: &mut super::eval::Context,
    backward_only: bool,
) -> Result<(), Flow> {
    let buf = eval
        .buffers
        .current_buffer()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

    let accessible = buf.accessible_emacs_byte_region();
    let pmin = accessible.start();
    let pmax = accessible.end();
    let pt = buf.point_emacs_byte_pos();

    let mut left = pt;
    while left > pmin {
        let Some(ch) = buf.char_before_emacs_byte_pos(left) else {
            break;
        };
        if !is_horizontal_space(ch) {
            break;
        }
        let Some(char_len) = buf.char_before_emacs_byte_len(left) else {
            break;
        };
        left = left.saturating_sub_len(char_len.max(EmacsByteLen::new(1)));
    }

    let mut right = pt;
    if !backward_only {
        while right < pmax {
            let Some(ch) = buf.char_after_emacs_byte_pos(right) else {
                break;
            };
            if !is_horizontal_space(ch) {
                break;
            }
            let Some(char_len) = buf.char_after_emacs_byte_len(right) else {
                break;
            };
            right = right.add_len(char_len.max(EmacsByteLen::new(1)));
        }
    }

    if left == right {
        return Ok(());
    }

    if buffer_read_only_active(eval, buf) {
        return Err(signal("buffer-read-only", vec![buf.name_value()]));
    }

    let current_id = eval
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let delete_range = super::editfns::buffer_edit_range_for_byte_range_in_manager(
        &eval.buffers,
        current_id,
        EmacsByteRange::new(left, right),
    )?;
    let change = crate::buffer::TextChange::deletion(delete_range);
    super::editfns::signal_before_text_change(eval, change)?;
    let _ = eval
        .buffers
        .delete_buffer_measured_region(current_id, delete_range);
    super::editfns::signal_after_text_change(eval, change)?;
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
    let point = {
        let Some(buf) = ctx.buffers.get(current_id) else {
            return Ok(Value::fixnum(0));
        };
        buf.accessible_emacs_byte_region()
            .clamp(buf.point_emacs_byte_pos())
    };
    let scan = scan_for_column(ctx, current_id, Some(point), None)?;
    Ok(Value::fixnum(scan.column as i64))
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
    let tabw = tab_width_in_state(&ctx.obarray, &[], Some(buf));
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
            return Err(signal("buffer-read-only", vec![buffer_name]));
        }
        let _ = ctx.buffers.goto_buffer_emacs_byte_pos(current_id, tab_byte);
        let pad = spaces_to_column(col_before_tab, target);
        let insert_pos = tab_byte;
        let pad_len = pad.len();
        let pad_change = super::editfns::text_change_for_empty_insertion_at_emacs_byte_pos(
            &ctx.buffers,
            current_id,
            insert_pos,
            TextExtent::new(CharLen::new(pad_len), EmacsByteLen::new(pad_len)),
        )?;
        super::editfns::signal_before_text_change(ctx, pad_change)?;
        let _ = ctx.buffers.insert_into_buffer(current_id, &pad);
        super::editfns::signal_after_text_change(ctx, pad_change)?;
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
            return Err(signal("buffer-read-only", vec![buffer_name]));
        }
        let use_tabs = indent_tabs_mode_in_state(&ctx.obarray, &[], ctx.buffers.get(current_id));
        let pad = indent_to_column_string(reached, target, tabw, use_tabs);
        let insert_pos = ctx
            .buffers
            .get(current_id)
            .map(|b| b.point_emacs_byte_pos())
            .unwrap_or(EmacsBytePos::ZERO);
        let pad_len = pad.len();
        let change = super::editfns::text_change_for_empty_insertion_at_emacs_byte_pos(
            &ctx.buffers,
            current_id,
            insert_pos,
            TextExtent::new(CharLen::new(pad_len), EmacsByteLen::new(pad_len)),
        )?;
        super::editfns::signal_before_text_change(ctx, change)?;
        let _ = ctx.buffers.insert_into_buffer(current_id, &pad);
        super::editfns::signal_after_text_change(ctx, change)?;
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
        return Err(signal("buffer-read-only", vec![buf.name_value()]));
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

    let insert_pos = ctx
        .buffers
        .get(current_id)
        .map(|b| b.point_emacs_byte_pos())
        .unwrap_or(EmacsBytePos::ZERO);
    let indent_len = indent.len();
    if indent_len > 0 {
        let change = super::editfns::text_change_for_empty_insertion_at_emacs_byte_pos(
            &ctx.buffers,
            current_id,
            insert_pos,
            TextExtent::new(CharLen::new(indent_len), EmacsByteLen::new(indent_len)),
        )?;
        super::editfns::signal_before_text_change(ctx, change)?;
        super::builtins::insert_string_value_in_current_buffer(
            &ctx.obarray,
            &[],
            &mut ctx.buffers,
            Value::string(indent),
            false,
            true,
        )?;
        super::editfns::signal_after_text_change(ctx, change)?;
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

    // indent-tabs-mode: default t
    obarray.set_symbol_value("indent-tabs-mode", Value::T);
    obarray.make_special("indent-tabs-mode");

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
#[cfg(test)]
#[path = "indent_test.rs"]
mod tests;
