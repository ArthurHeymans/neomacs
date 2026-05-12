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
use crate::buffer::{Buffer, BufferManager};
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
        return Some(buf.slots[info.offset]);
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
    byte_pos: usize,
    column: usize,
    previous_byte_pos: usize,
    previous_column: usize,
    previous_code: Option<u32>,
}

fn line_bounds(buf: &Buffer, point: usize) -> (usize, usize) {
    let begv = buf.begv_byte;
    let zv = buf.zv_byte;
    let pt = point.clamp(begv, zv);

    let mut bol = pt;
    while bol > begv && buf.text.emacs_byte_at(bol - 1) != Some(b'\n') {
        bol -= 1;
    }

    let mut eol = pt;
    while eol < zv && buf.text.emacs_byte_at(eol) != Some(b'\n') {
        eol += 1;
    }

    (bol, eol)
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

fn buffer_char_display_width(buf: &Buffer, byte_pos: usize, code: u32) -> usize {
    if !buf.get_multibyte() {
        return buf
            .text
            .emacs_byte_at(byte_pos)
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
    point: usize,
) -> Result<(usize, usize), Flow> {
    let buf = ctx
        .buffers
        .get(buffer_id)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    Ok(line_bounds(buf, point))
}

fn scan_for_column(
    ctx: &mut super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    end_byte: usize,
    goal_column: Option<usize>,
) -> Result<ColumnScan, Flow> {
    let (mut scan, line_end, tab_width) = {
        let buf = ctx
            .buffers
            .get(buffer_id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let (bol, eol) = line_bounds(buf, buf.pt_byte);
        (bol, eol, tab_width_in_state(&ctx.obarray, &[], Some(buf)))
    };
    let end = end_byte.min(line_end);
    let goal = goal_column.unwrap_or(usize::MAX);
    let mut column = 0usize;
    let mut previous_byte_pos = scan;
    let mut previous_column = 0usize;
    let mut previous_code = None;

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

        let (code, char_len, width) = {
            let buf = ctx
                .buffers
                .get(buffer_id)
                .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
            let Some(code) = buf.char_code_after(scan) else {
                break;
            };
            let char_len = buf.char_after_emacs_len(scan).unwrap_or(1).max(1);
            let width = buffer_char_display_width(buf, scan, code);
            (code, char_len, width)
        };

        if code == b'\n' as u32 {
            break;
        }

        previous_byte_pos = scan;
        previous_column = column;
        previous_code = Some(code);
        column = next_column_for_code(column, code, width, tab_width);
        scan += char_len;
    }

    Ok(ColumnScan {
        byte_pos: scan,
        column,
        previous_byte_pos,
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

fn padding_to_column(mut column: usize, target: usize, tab_width: usize) -> String {
    let mut out = String::new();
    let tab = tab_width.max(1);
    while column < target {
        let next_tab = column + (tab - (column % tab));
        if next_tab <= target && next_tab > column + 1 {
            out.push('\t');
            column = next_tab;
        } else {
            out.push(' ');
            column += 1;
        }
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

    let pmin = buf.point_min();
    let pmax = buf.point_max();
    let pt = buf.point();

    let mut left = pt;
    while left > pmin {
        let Some(ch) = buf.char_before(left) else {
            break;
        };
        if !is_horizontal_space(ch) {
            break;
        }
        left -= ch.len_utf8();
    }

    let mut right = pt;
    if !backward_only {
        while right < pmax {
            let Some(ch) = buf.char_after(right) else {
                break;
            };
            if !is_horizontal_space(ch) {
                break;
            }
            right += ch.len_utf8();
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
    let old_len = super::editfns::current_buffer_byte_span_char_len(eval, left, right);
    super::editfns::signal_before_change(eval, left, right)?;
    let _ = eval.buffers.delete_buffer_region(current_id, left, right);
    super::editfns::signal_after_change(eval, left, left, old_len)?;
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
    let (bol, eol) = line_bounds(buf, buf.pt_byte);
    let line = buf.buffer_substring_lisp_string(bol, eol);

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
        buf.pt_byte.clamp(buf.begv_byte, buf.zv_byte)
    };
    let scan = scan_for_column(ctx, current_id, point, None)?;
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
    let pt = buf.pt_byte.clamp(buf.begv_byte, buf.zv_byte);
    let buffer_name = buf.name_value();

    if target == 0 {
        let (bol, _) = current_buffer_line_bounds(ctx, current_id, pt)?;
        let _ = ctx.buffers.goto_buffer_byte(current_id, bol);
        return Ok(Value::fixnum(0));
    }

    let mut tab_split: Option<(usize, usize, usize)> = None;
    let scan = scan_for_column(ctx, current_id, usize::MAX, Some(target))?;
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
        let _ = ctx.buffers.goto_buffer_byte(current_id, tab_byte);
        let pad = padding_to_column(col_before_tab, target, tabw);
        let insert_pos = tab_byte;
        let pad_len = pad.len();
        super::editfns::signal_before_change(ctx, insert_pos, insert_pos)?;
        let _ = ctx.buffers.insert_into_buffer(current_id, &pad);
        super::editfns::signal_after_change(ctx, insert_pos, insert_pos + pad_len, 0)?;
        let tab_after_pad = insert_pos + pad_len;
        super::editfns::signal_before_change(ctx, tab_after_pad, tab_after_pad + 1)?;
        let _ = ctx
            .buffers
            .delete_buffer_region(current_id, tab_after_pad, tab_after_pad + 1);
        super::editfns::signal_after_change(ctx, tab_after_pad, tab_after_pad, 1)?;
        let goal_point = tab_after_pad;
        let _ = ctx.buffers.goto_buffer_byte(current_id, goal_point);
        let _ = builtin_indent_to(ctx, vec![Value::fixnum(col_after_tab as i64), Value::NIL])?;
        let _ = ctx.buffers.goto_buffer_byte(current_id, goal_point);
        return Ok(Value::fixnum(target as i64));
    }

    let _ = ctx.buffers.goto_buffer_byte(current_id, dest_byte);

    if force_is_t && reached < target {
        if read_only {
            return Err(signal("buffer-read-only", vec![buffer_name]));
        }
        let pad = padding_to_column(reached, target, tabw);
        let insert_pos = ctx.buffers.get(current_id).map(|b| b.pt_byte).unwrap_or(0);
        let pad_len = pad.len();
        super::editfns::signal_before_change(ctx, insert_pos, insert_pos)?;
        let _ = ctx.buffers.insert_into_buffer(current_id, &pad);
        super::editfns::signal_after_change(ctx, insert_pos, insert_pos + pad_len, 0)?;
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

    let pt = buf.point();
    let pmin = buf.point_min();
    let (bol, _) = line_bounds(buf, pt);
    let line_prefix = buf.buffer_substring_lisp_string(bol, pt);
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

    let insert_pos = ctx.buffers.get(current_id).map(|b| b.pt_byte).unwrap_or(0);
    let indent_len = indent.len();
    if indent_len > 0 {
        super::editfns::signal_before_change(ctx, insert_pos, insert_pos)?;
        let _ = ctx.buffers.insert_into_buffer(current_id, &indent);
        super::editfns::signal_after_change(ctx, insert_pos, insert_pos + indent_len, 0)?;
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
