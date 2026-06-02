//! Buffer navigation, line operations, and mark/region management builtins.
//!
//! All functions here take `(eval: &mut Context, args: Vec<Value>) -> EvalResult`
//! and are dispatched from `builtins.rs` via `dispatch_builtin`.

use super::error::{EvalResult, Flow, signal};
use super::intern::intern;
use super::syntax::{SyntaxClass, SyntaxTable};
use super::textprop::{buffer_overlay_property_at_byte_pos, lookup_buffer_text_property};
use super::value::{Value, ValueKind, VecLikeType, lexenv_lookup};
use crate::buffer::BufferManager;
use malachite::integer::Integer;
use num_enum::{IntoPrimitive, TryFromPrimitive};

// ---------------------------------------------------------------------------
// Argument helpers (duplicated from builtins.rs — they are not `pub`)
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

fn expect_int(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

#[derive(Clone, Copy)]
struct LineCountArg {
    original: Value,
    count: i64,
    excessive: bool,
}

fn bignum_line_count(value: &Value) -> i64 {
    let n = value.as_bignum().expect("bignum kind");
    if n >= &Integer::from(0) {
        Value::MOST_POSITIVE_FIXNUM
    } else {
        Value::MOST_NEGATIVE_FIXNUM
    }
}

fn line_count_arg(value: &Value) -> Result<LineCountArg, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(LineCountArg {
            original: *value,
            count: n,
            excessive: false,
        }),
        ValueKind::Veclike(VecLikeType::Bignum) => Ok(LineCountArg {
            original: *value,
            count: bignum_line_count(value),
            excessive: true,
        }),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

fn optional_line_count_arg(args: &[Value], default: i64) -> Result<LineCountArg, Flow> {
    if args.is_empty() || args[0].is_nil() {
        Ok(LineCountArg {
            original: Value::fixnum(default),
            count: default,
            excessive: false,
        })
    } else {
        line_count_arg(&args[0])
    }
}

pub(crate) fn line_beginning_scan_count_arg(args: &[Value]) -> Result<i64, Flow> {
    if args.is_empty() || args[0].is_nil() {
        Ok(0)
    } else {
        Ok(line_count_arg(&args[0])?.count.saturating_sub(1))
    }
}

pub(crate) fn line_end_scan_count_arg(args: &[Value]) -> Result<i64, Flow> {
    if args.is_empty() || args[0].is_nil() {
        Ok(1)
    } else {
        Ok(line_count_arg(&args[0])?.count)
    }
}

fn line_count_result(arg: LineCountArg, shortage: i64) -> Value {
    if !arg.excessive {
        return Value::make_int(shortage);
    }

    let adjustment = shortage - arg.count;
    if let Some(big) = arg.original.as_bignum() {
        return Value::make_integer(big.clone() + Integer::from(adjustment));
    }
    Value::make_int(shortage)
}

/// Get a no-current-buffer signal flow.
fn no_buffer() -> Flow {
    signal("error", vec![Value::string("No current buffer")])
}

fn current_buffer_in_manager(buffers: &BufferManager) -> Result<&crate::buffer::Buffer, Flow> {
    buffers.current_buffer().ok_or_else(no_buffer)
}

fn dynamic_or_global_symbol_value(eval: &super::eval::Context, name: &str) -> Option<Value> {
    let name_id = intern(name);
    if eval.lexical_binding() && !eval.obarray.is_special(name) {
        if let Some(v) = lexenv_lookup(eval.lexenv, name_id) {
            return Some(v);
        }
    }

    if let Some(buf) = eval.buffers.current_buffer() {
        if let Some(v) = buf.get_buffer_local(name) {
            return Some(v);
        }
    }

    eval.obarray.symbol_value(name).cloned()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a 1-based Emacs char position to a 0-based byte position in the
/// current buffer.  Clamps to valid range.
fn char_pos_to_byte(buf: &crate::buffer::Buffer, pos: i64) -> usize {
    buf.lisp_pos_to_byte(pos)
}

/// Convert a 0-based byte position to a 1-based Emacs char position.
fn byte_to_char_pos(buf: &crate::buffer::Buffer, byte_pos: usize) -> i64 {
    buf.text.emacs_byte_to_char(byte_pos) as i64 + 1
}

fn clamp_byte_to_accessible(buf: &crate::buffer::Buffer, byte_pos: usize) -> usize {
    byte_pos.clamp(buf.point_min_byte(), buf.point_max_byte())
}

/// Return the full buffer text as raw Emacs bytes.
fn buffer_bytes(buf: &crate::buffer::Buffer) -> Vec<u8> {
    let mut out = Vec::new();
    buf.copy_emacs_bytes_to(0, buf.total_bytes(), &mut out);
    out
}

/// Count newlines in the Emacs-byte range [start, end).
fn count_newlines(text: &[u8], start: usize, end: usize) -> usize {
    let s = start.min(text.len());
    let e = end.max(start).min(text.len());
    text[s..e].iter().filter(|&&b| b == b'\n').count()
}

/// Like `move_by_lines` but confined to the narrowed region `[begv, zv)`.
fn move_by_lines_narrowed(
    text: &[u8],
    byte_pos: usize,
    n: i64,
    begv: usize,
    zv: usize,
) -> (usize, i64) {
    let zv = zv.min(text.len());
    let mut pos = byte_pos.clamp(begv, zv);
    let mut moved: i64 = 0;
    if n >= 0 {
        if n == 0 {
            return (line_beginning_byte_narrowed(text, pos, begv), 0);
        }
        for _ in 0..n {
            match text[pos..zv].iter().position(|&b| b == b'\n') {
                Some(offset) => {
                    pos += offset + 1;
                    moved += 1;
                }
                None => {
                    pos = zv;
                    break;
                }
            }
        }
    } else {
        for _ in 0..(-n) {
            let bol = line_beginning_byte_narrowed(text, pos, begv);
            if bol <= begv {
                pos = begv;
                break;
            }
            pos = line_beginning_byte_narrowed(text, bol - 1, begv);
            moved -= 1;
        }
    }
    (pos, moved)
}

/// Find the beginning of the line containing `byte_pos`, but not before `begv`.
fn line_beginning_byte_narrowed(text: &[u8], byte_pos: usize, begv: usize) -> usize {
    let pos = byte_pos.min(text.len());
    let start = begv.min(pos);
    match text[start..pos].iter().rposition(|&b| b == b'\n') {
        Some(offset) => start + offset + 1,
        None => start,
    }
}

/// Find the end of the line containing `byte_pos`, but not past `zv`.
fn line_end_byte_narrowed(text: &[u8], byte_pos: usize, zv: usize) -> usize {
    let pos = byte_pos.min(text.len());
    let end = zv.min(text.len());
    match text[pos..end].iter().position(|&b| b == b'\n') {
        Some(offset) => pos + offset,
        None => end,
    }
}

// ===========================================================================
// Point motion hooks and intangible support
// ===========================================================================

pub(crate) fn check_point_motion_hooks(
    eval: &mut super::eval::Context,
    old_byte: usize,
    new_byte: usize,
) -> Result<(), Flow> {
    if old_byte == new_byte {
        return Ok(());
    }
    let inhibit = eval
        .obarray
        .symbol_value("inhibit-point-motion-hooks")
        .cloned()
        .unwrap_or(Value::NIL);
    if inhibit.is_truthy() {
        return Ok(());
    }
    let current_id = match eval.buffers.current_buffer_id() {
        Some(id) => id,
        None => return Ok(()),
    };
    let (old_lisp, new_lisp, leave_before, leave_after, enter_before, enter_after) = {
        let buf = match eval.buffers.get(current_id) {
            Some(b) => b,
            None => return Ok(()),
        };
        let ol = buf.text.emacs_byte_to_char(old_byte) as i64 + 1;
        let nl = buf.text.emacs_byte_to_char(new_byte) as i64 + 1;
        let leave_before = point_motion_property(
            &eval.obarray,
            &eval.buffers,
            buf,
            old_byte,
            false,
            "point-left",
        );
        let leave_after = point_motion_property(
            &eval.obarray,
            &eval.buffers,
            buf,
            old_byte,
            true,
            "point-left",
        );
        let enter_before = point_motion_property(
            &eval.obarray,
            &eval.buffers,
            buf,
            new_byte,
            false,
            "point-entered",
        );
        let enter_after = point_motion_property(
            &eval.obarray,
            &eval.buffers,
            buf,
            new_byte,
            true,
            "point-entered",
        );
        (ol, nl, leave_before, leave_after, enter_before, enter_after)
    };

    if leave_before != enter_before && leave_before.is_truthy() {
        eval.apply(
            leave_before,
            vec![Value::fixnum(old_lisp), Value::fixnum(new_lisp)],
        )?;
    }
    if leave_after != enter_after && leave_after.is_truthy() {
        eval.apply(
            leave_after,
            vec![Value::fixnum(old_lisp), Value::fixnum(new_lisp)],
        )?;
    }
    if enter_before != leave_before && enter_before.is_truthy() {
        eval.apply(
            enter_before,
            vec![Value::fixnum(old_lisp), Value::fixnum(new_lisp)],
        )?;
    }
    if enter_after != leave_after && enter_after.is_truthy() {
        eval.apply(
            enter_after,
            vec![Value::fixnum(old_lisp), Value::fixnum(new_lisp)],
        )?;
    }
    Ok(())
}

fn point_motion_property(
    obarray: &super::symbol::Obarray,
    buffers: &BufferManager,
    buf: &crate::buffer::Buffer,
    point_byte: usize,
    after_point: bool,
    property: &str,
) -> Value {
    if after_point {
        if point_byte >= buf.zv_byte {
            return Value::NIL;
        }
        lookup_buffer_text_property(obarray, buffers, buf, point_byte, Value::symbol(property))
    } else {
        if point_byte <= buf.begv_byte {
            return Value::NIL;
        }
        lookup_buffer_text_property(
            obarray,
            buffers,
            buf,
            point_byte - 1,
            Value::symbol(property),
        )
    }
}

fn lookup_buffer_char_property(
    obarray: &super::symbol::Obarray,
    buffers: &BufferManager,
    buf: &crate::buffer::Buffer,
    byte_pos: usize,
    prop: Value,
) -> Value {
    if byte_pos >= buf.text.char_to_emacs_byte(buf.text.char_count()) {
        return Value::NIL;
    }
    if let Some((value, _overlay)) =
        buffer_overlay_property_at_byte_pos(obarray, buffers, buf, byte_pos, prop, None)
    {
        return value;
    }
    lookup_buffer_text_property(obarray, buffers, buf, byte_pos, prop)
}

fn next_char_property_change(buf: &crate::buffer::Buffer, byte_pos: usize) -> Option<usize> {
    let text_next = buf
        .text
        .text_props_next_change(byte_pos)
        .filter(|next| *next <= buf.zv_byte);
    let overlay_next = buf
        .overlays
        .next_boundary_after_until(byte_pos, buf.zv_byte);
    match (text_next, overlay_next) {
        (Some(text), Some(overlay)) => Some(text.min(overlay)),
        (Some(text), None) => Some(text),
        (None, Some(overlay)) => Some(overlay),
        (None, None) => None,
    }
}

fn previous_char_property_change(buf: &crate::buffer::Buffer, byte_pos: usize) -> Option<usize> {
    let text_prev = buf
        .text
        .text_props_previous_change(byte_pos)
        .filter(|prev| *prev >= buf.begv_byte);
    let overlay_prev = buf
        .overlays
        .previous_boundary_before_since(byte_pos, buf.begv_byte);
    match (text_prev, overlay_prev) {
        (Some(text), Some(overlay)) => Some(text.max(overlay)),
        (Some(text), None) => Some(text),
        (None, Some(overlay)) => Some(overlay),
        (None, None) => None,
    }
}

pub(crate) fn adjust_for_intangible(
    eval: &super::eval::Context,
    pos: usize,
    direction: i32,
) -> usize {
    let inhibit = eval
        .obarray
        .symbol_value("inhibit-point-motion-hooks")
        .cloned()
        .unwrap_or(Value::NIL);
    if inhibit.is_truthy() {
        return pos;
    }
    let current_id = match eval.buffers.current_buffer_id() {
        Some(id) => id,
        None => return pos,
    };
    let buf = match eval.buffers.get(current_id) {
        Some(b) => b,
        None => return pos,
    };
    let intangible = lookup_buffer_char_property(
        &eval.obarray,
        &eval.buffers,
        buf,
        pos,
        Value::symbol("intangible"),
    );
    if !intangible.is_truthy() {
        return pos;
    }
    let mut cursor = pos;
    if direction >= 0 {
        loop {
            match next_char_property_change(buf, cursor) {
                Some(next) => {
                    let prop = lookup_buffer_char_property(
                        &eval.obarray,
                        &eval.buffers,
                        buf,
                        next,
                        Value::symbol("intangible"),
                    );
                    cursor = next;
                    if prop != intangible {
                        break;
                    }
                }
                None => {
                    cursor = buf.zv_byte;
                    break;
                }
            }
        }
    } else {
        loop {
            match previous_char_property_change(buf, cursor) {
                Some(prev) => {
                    let check = prev.saturating_sub(1);
                    let prop = lookup_buffer_char_property(
                        &eval.obarray,
                        &eval.buffers,
                        buf,
                        check,
                        Value::symbol("intangible"),
                    );
                    cursor = prev;
                    if prop != intangible {
                        break;
                    }
                }
                None => {
                    cursor = buf.begv_byte;
                    break;
                }
            }
        }
    }
    cursor
}

// ===========================================================================
// Position predicates
// ===========================================================================

/// (bobp) -- at beginning of buffer?
pub(crate) fn builtin_bobp(ctx: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("bobp", &args, 0)?;
    let buf = current_buffer_in_manager(&ctx.buffers)?;
    Ok(Value::bool_val(buf.pt_byte == buf.begv_byte))
}

/// (eobp) -- at end of buffer?
pub(crate) fn builtin_eobp(ctx: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("eobp", &args, 0)?;
    let buf = current_buffer_in_manager(&ctx.buffers)?;
    Ok(Value::bool_val(buf.pt_byte == buf.zv_byte))
}

/// (bolp) -- at beginning of line?
pub(crate) fn builtin_bolp(ctx: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("bolp", &args, 0)?;
    let buf = current_buffer_in_manager(&ctx.buffers)?;
    if buf.pt_byte == buf.begv_byte {
        return Ok(Value::T);
    }
    Ok(Value::bool_val(
        buf.pt_byte == 0 || buf.char_code_before(buf.pt_byte) == Some('\n' as u32),
    ))
}

/// (eolp) -- at end of line?
pub(crate) fn builtin_eolp(ctx: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("eolp", &args, 0)?;
    let buf = current_buffer_in_manager(&ctx.buffers)?;
    if buf.pt_byte == buf.zv_byte {
        return Ok(Value::T);
    }
    match buf.char_code_after(buf.pt_byte) {
        Some(code) if code == '\n' as u32 => Ok(Value::T),
        _ => Ok(Value::NIL),
    }
}

// ===========================================================================
// Line operations
// ===========================================================================

/// (line-beginning-position &optional N)
/// Compute the unconstrained beginning-of-line position for the current
/// buffer's point after moving `n - 1` lines. Returns `(bol_charpos,
/// orig_charpos, lines_moved)` mirroring GNU's static `bol` helper
/// (editfns.c) plus the original PT used as anchor for field constraint.
pub(crate) fn pos_bol_compute(
    ctx: &super::eval::Context,
    scan_count: i64,
) -> Result<(i64, i64, i64), Flow> {
    let buf = current_buffer_in_manager(&ctx.buffers)?;
    let text = buffer_bytes(buf);
    let begv = buf.begv_byte;
    let zv = buf.zv_byte;
    let mut pos = buf.pt_byte;
    let mut moved: i64 = 0;
    if scan_count != 0 {
        let (new_pos, actual_moved) = move_by_lines_narrowed(&text, pos, scan_count, begv, zv);
        pos = new_pos;
        moved = actual_moved;
    }
    // GNU `bol` (editfns.c) asks `scan_newline_from_point` for N - 1 lines.
    // If a forward scan reaches ZV before finding enough newlines, the
    // returned position is ZV itself, not the beginning of the final
    // unterminated line containing ZV.  `delete-line` relies on this via
    // `(pos-bol 2)` to delete the last line of a buffer.
    let bol = if scan_count > 0 && moved != scan_count && pos == zv {
        zv
    } else {
        line_beginning_byte_narrowed(&text, pos, begv)
    };
    Ok((
        byte_to_char_pos(buf, bol),
        byte_to_char_pos(buf, buf.pt_byte),
        moved,
    ))
}

/// Compute the unconstrained end-of-line position for the current buffer's
/// point after moving `n - 1` lines. Returns `(eol_charpos, orig_charpos)`,
/// mirroring GNU's static `eol` helper (editfns.c).
pub(crate) fn pos_eol_compute(
    ctx: &super::eval::Context,
    scan_count: i64,
) -> Result<(i64, i64), Flow> {
    let buf = current_buffer_in_manager(&ctx.buffers)?;
    let text = buffer_bytes(buf);
    let begv = buf.begv_byte;
    let zv = buf.zv_byte;
    let mut pos = buf.pt_byte;
    let mut moved = 0;
    let delta = scan_count.saturating_sub(1);
    if delta != 0 {
        let (new_pos, actual_moved) = move_by_lines_narrowed(&text, pos, delta, begv, zv);
        pos = new_pos;
        moved = actual_moved;
    }
    let eol = if delta != 0 && moved != delta && pos == begv {
        begv
    } else {
        line_end_byte_narrowed(&text, pos, zv)
    };
    Ok((
        byte_to_char_pos(buf, eol),
        byte_to_char_pos(buf, buf.pt_byte),
    ))
}

pub(crate) fn builtin_line_beginning_position(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_max_args("line-beginning-position", &args, 1)?;
    let scan_count = line_beginning_scan_count_arg(&args)?;
    let (bol_charpos, orig_charpos, count) = pos_bol_compute(ctx, scan_count)?;
    // GNU `Fline_beginning_position` (editfns.c:700) constrains the result to
    // the current input field. ESCAPE-FROM-EDGE is t when any lines were
    // scanned (count != 0), nil otherwise; ONLY-IN-LINE is always t.
    crate::emacs_core::builtins::builtin_constrain_to_field(
        ctx,
        vec![
            Value::fixnum(bol_charpos),
            Value::fixnum(orig_charpos),
            if count != 0 { Value::T } else { Value::NIL },
            Value::T,
            Value::NIL,
        ],
    )
}

/// (line-end-position &optional N)
pub(crate) fn builtin_line_end_position(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_max_args("line-end-position", &args, 1)?;
    let scan_count = line_end_scan_count_arg(&args)?;
    let (eol_charpos, orig_charpos) = pos_eol_compute(ctx, scan_count)?;
    // GNU `Fline_end_position` (editfns.c:755): constrain to current input
    // field with ESCAPE-FROM-EDGE = nil and ONLY-IN-LINE = t.
    crate::emacs_core::builtins::builtin_constrain_to_field(
        ctx,
        vec![
            Value::fixnum(eol_charpos),
            Value::fixnum(orig_charpos),
            Value::NIL,
            Value::T,
            Value::NIL,
        ],
    )
}

/// (line-number-at-pos &optional POS ABSOLUTE)
pub(crate) fn builtin_line_number_at_pos(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let buf = eval.buffers.current_buffer().ok_or_else(no_buffer)?;
    let current_buffer_id = buf.id;
    let byte_pos = if args.is_empty() || args[0].is_nil() {
        buf.pt_byte
    } else {
        match args[0].kind() {
            ValueKind::Veclike(VecLikeType::Marker) => {
                let marker = args[0].as_marker_data().unwrap();
                if marker.buffer == Some(current_buffer_id) {
                    marker
                        .marker_id
                        .and_then(|marker_id| {
                            eval.buffers.marker_position(current_buffer_id, marker_id)
                        })
                        .unwrap_or_else(|| char_pos_to_byte(buf, marker.charpos as i64 + 1))
                } else {
                    char_pos_to_byte(buf, marker.charpos as i64 + 1)
                }
            }
            ValueKind::Fixnum(pos) => {
                let beg = buf.point_min_char() as i64 + 1;
                let z = buf.point_max_char() as i64 + 1;
                if pos < beg || pos > z {
                    return Err(signal(
                        "args-out-of-range",
                        vec![args[0], Value::fixnum(beg), Value::fixnum(z)],
                    ));
                }
                char_pos_to_byte(buf, pos)
            }
            _ => {
                return Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("fixnump"), args[0]],
                ));
            }
        }
    };
    let _absolute = args.get(1).is_some_and(|v| v.is_truthy());
    // Count newlines from start of buffer to byte_pos.
    let text = buffer_bytes(buf);
    let start = if _absolute { 0 } else { buf.begv_byte };
    let line_num = count_newlines(&text, start, byte_pos) + 1;
    Ok(Value::fixnum(line_num as i64))
}

/// (count-lines BEG END)
pub(crate) fn builtin_count_lines(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("count-lines", &args, 2)?;
    expect_max_args("count-lines", &args, 3)?;
    let beg = expect_int(&args[0])?;
    let end = expect_int(&args[1])?;
    let buf = eval.buffers.current_buffer().ok_or_else(no_buffer)?;
    let byte_beg = char_pos_to_byte(buf, beg);
    let byte_end = char_pos_to_byte(buf, end);
    let (s, e) = if byte_beg <= byte_end {
        (byte_beg, byte_end)
    } else {
        (byte_end, byte_beg)
    };
    let text = buffer_bytes(buf);
    let mut n = count_newlines(&text, s, e);
    // GNU Emacs: "can be one more if START is not equal to END and the
    // greater of them is not at the start of a line."
    // i.e., if the region is non-empty and the char before `e` is not '\n'.
    if s != e && e > 0 && buf.char_code_before(e) != Some('\n' as u32) {
        n += 1;
    }
    Ok(Value::fixnum(n as i64))
}

/// (forward-line &optional N) -> integer
pub(crate) fn builtin_forward_line(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let line_arg = optional_line_count_arg(&args, 1)?;
    let n = line_arg.count;
    let current_id = eval.buffers.current_buffer_id().ok_or_else(no_buffer)?;
    let (text, begv, zv, pt) = {
        let buf = eval.buffers.get(current_id).ok_or_else(no_buffer)?;
        (buffer_bytes(buf), buf.begv_byte, buf.zv_byte, buf.pt_byte)
    };
    let old_byte = pt;
    let (new_pos, moved) = move_by_lines_narrowed(&text, pt, n, begv, zv);
    let direction = if n >= 0 { 1 } else { -1 };
    let adjusted = adjust_for_intangible(eval, new_pos, direction);
    let _ = eval.buffers.goto_buffer_byte(current_id, adjusted);

    let mut shortage = n - moved;
    if shortage != 0 && n > 0 && begv < zv && new_pos != pt && new_pos > 0 {
        let at_line_start = eval
            .buffers
            .get(current_id)
            .is_some_and(|buf| buf.char_code_before(new_pos) == Some('\n' as u32));
        if !at_line_start {
            shortage -= 1;
        }
    }
    check_point_motion_hooks(eval, old_byte, adjusted)?;
    Ok(line_count_result(line_arg, shortage))
}

/// (beginning-of-line &optional N)
pub(crate) fn builtin_beginning_of_line(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let n = if args.is_empty() || args[0].is_nil() {
        1
    } else {
        expect_int(&args[0])?
    };
    // GNU `Fbeginning_of_line` (cmds.c:148) is literally
    // `SET_PT (XFIXNUM (Fline_beginning_position (n)))`. Delegate to our
    // line-beginning-position builtin so field constraints
    // (`Fconstrain_to_field`) apply uniformly.
    let constrained = builtin_line_beginning_position(eval, vec![Value::fixnum(n)])?;
    let target_char = match constrained.kind() {
        ValueKind::Fixnum(v) => v,
        _ => return Ok(Value::NIL),
    };
    let current_id = eval.buffers.current_buffer_id().ok_or_else(no_buffer)?;
    let target_byte = {
        let buf = eval.buffers.get(current_id).ok_or_else(no_buffer)?;
        let zero_based = (target_char - 1).max(0) as usize;
        buf.text.char_to_emacs_byte(zero_based)
    };
    let old_byte = {
        let buf = eval.buffers.get(current_id).ok_or_else(no_buffer)?;
        buf.pt_byte
    };
    let adjusted = adjust_for_intangible(eval, target_byte, -1);
    let _ = eval.buffers.goto_buffer_byte(current_id, adjusted);
    check_point_motion_hooks(eval, old_byte, adjusted)?;
    Ok(Value::NIL)
}

/// (end-of-line &optional N)
pub(crate) fn builtin_end_of_line(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    let n = if args.is_empty() || args[0].is_nil() {
        1
    } else {
        expect_int(&args[0])?
    };
    // GNU `Fend_of_line` (cmds.c:172) calls `Fline_end_position` (which
    // applies field constraints) then SET_PTs to that, looping over
    // intangible-then-newline corner cases. Mirror that pattern.
    let current_id = eval.buffers.current_buffer_id().ok_or_else(no_buffer)?;
    let old_byte = {
        let buf = eval.buffers.get(current_id).ok_or_else(no_buffer)?;
        buf.pt_byte
    };
    let constrained = builtin_line_end_position(eval, vec![Value::fixnum(n)])?;
    let target_char = match constrained.kind() {
        ValueKind::Fixnum(v) => v,
        _ => return Ok(Value::NIL),
    };
    let target_byte = {
        let buf = eval.buffers.get(current_id).ok_or_else(no_buffer)?;
        let zero_based = (target_char - 1).max(0) as usize;
        buf.text.char_to_emacs_byte(zero_based)
    };
    let adjusted = adjust_for_intangible(eval, target_byte, 1);
    let _ = eval.buffers.goto_buffer_byte(current_id, adjusted);
    check_point_motion_hooks(eval, old_byte, adjusted)?;
    Ok(Value::NIL)
}

// ===========================================================================
// Character movement
// ===========================================================================

/// (forward-char &optional N)
///
/// Mirrors GNU `Fforward_char` (`src/cmds.c:69`) and `move_point` at
/// `src/cmds.c:36`. The accessible portion of the buffer is bounded by
/// `BEGV` / `ZV` (the narrowing region), not the absolute buffer
/// extents — `forward-char` must clamp to and signal against those
/// fields, otherwise narrowing is silently ignored (audit §7.1).
pub(crate) fn builtin_forward_char(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let n = if args.is_empty() || args[0].is_nil() {
        1
    } else {
        expect_int(&args[0])?
    };
    let current_id = eval.buffers.current_buffer_id().ok_or_else(no_buffer)?;
    let (old_byte, cur_char, begv_char, zv_char, new_byte) = {
        let buf = eval.buffers.get(current_id).ok_or_else(no_buffer)?;
        let old_byte = buf.pt_byte;
        let cur_char = buf.point_char();
        let begv_char = buf.point_min_char();
        let zv_char = buf.point_max_char();
        let desired = cur_char as i64 + n;
        let clamped_char = desired.clamp(begv_char as i64, zv_char as i64) as usize;
        (
            old_byte,
            cur_char,
            begv_char,
            zv_char,
            buf.text.char_to_emacs_byte(clamped_char),
        )
    };
    let direction = if n >= 0 { 1 } else { -1 };
    let adjusted = adjust_for_intangible(eval, new_byte, direction);
    let _ = eval.buffers.goto_buffer_byte(current_id, adjusted);
    // GNU `move_point`: signal beginning-of-buffer / end-of-buffer when
    // the requested position falls outside the accessible portion.
    let desired = cur_char as i64 + n;
    if desired < begv_char as i64 {
        return Err(signal("beginning-of-buffer", vec![]));
    }
    if desired > zv_char as i64 {
        return Err(signal("end-of-buffer", vec![]));
    }
    check_point_motion_hooks(eval, old_byte, adjusted)?;
    Ok(Value::NIL)
}

/// (backward-char &optional N)
pub(crate) fn builtin_backward_char(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let n = if args.is_empty() || args[0].is_nil() {
        1
    } else {
        expect_int(&args[0])?
    };
    // backward-char N == forward-char (- N)
    builtin_forward_char(eval, vec![Value::fixnum(-n)])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumString, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
#[strum(serialize_all = "lowercase")]
enum SkipCharClass {
    Alnum = 1,
    Alpha = 2,
    Word = 3,
    Graph = 4,
    Print = 5,
    Lower = 6,
    Upper = 7,
    Punct = 8,
    Cntrl = 9,
    Digit = 10,
    Xdigit = 11,
    Blank = 12,
    Space = 13,
    Multibyte = 14,
    Nonascii = 15,
    Ascii = 16,
    Unibyte = 17,
}

#[derive(Clone, Debug)]
struct SkipCharsSet {
    negate: bool,
    ranges: Vec<(u32, u32)>,
    classes: Vec<SkipCharClass>,
}

/// Parse GNU ISO C character class syntax used by `skip_chars`.
///
/// Mirrors GNU `re_wctype_parse`: only a leading `[:name:]` token is a class;
/// if the token is closed but the class name is invalid, `skip_chars` signals
/// `Invalid ISO C character class`.  If there is no closing `:]`, the leading
/// `[` is treated as an ordinary character by the caller.
fn parse_skip_char_class(codes: &[u32], i: usize) -> Result<Option<(SkipCharClass, usize)>, Flow> {
    if codes.get(i) != Some(&('[' as u32)) || codes.get(i + 1) != Some(&(':' as u32)) {
        return Ok(None);
    }

    let mut end = i + 2;
    while end + 1 < codes.len() {
        if codes[end] == ':' as u32 && codes[end + 1] == ']' as u32 {
            let name: String = codes[i + 2..end]
                .iter()
                .filter_map(|code| char::from_u32(*code))
                .collect();
            let Ok(class) = name.parse::<SkipCharClass>() else {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid ISO C character class")],
                ));
            };
            return Ok(Some((class, end + 2)));
        }
        end += 1;
    }

    Ok(None)
}

/// Parse a skip-chars set matching GNU `syntax.c:skip_chars` behavior.
/// Handles `\` as escape character, `-` as range operator, and ISO C
/// character classes such as `[:alpha:]`.
fn parse_skip_chars_set(codes: &[u32]) -> Result<SkipCharsSet, Flow> {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let mut classes: Vec<SkipCharClass> = Vec::new();
    let mut negate = false;
    let mut i = 0;

    if i < codes.len() && codes[i] == '^' as u32 {
        negate = true;
        i += 1;
    }

    while i < codes.len() {
        if let Some((class, next_i)) = parse_skip_char_class(codes, i)? {
            if !classes.contains(&class) {
                classes.push(class);
            }
            i = next_i;
            continue;
        }

        // Handle backslash escape (GNU syntax.c: `\-` = literal `-`)
        let c = if codes[i] == '\\' as u32 && i + 1 < codes.len() {
            i += 1;
            codes[i]
        } else {
            codes[i]
        };
        i += 1;

        // Check for range: c followed by `-` and another char
        if i + 1 < codes.len() && codes[i] == '-' as u32 {
            i += 1; // skip '-'
            let end_c = if codes[i] == '\\' as u32 && i + 1 < codes.len() {
                i += 1;
                codes[i]
            } else {
                codes[i]
            };
            i += 1;
            if c <= end_c {
                ranges.push((c, end_c));
            }
        } else {
            ranges.push((c, c));
        }
    }

    Ok(SkipCharsSet {
        negate,
        ranges,
        classes,
    })
}

fn skip_char_in_explicit_ranges(set: &SkipCharsSet, code: u32) -> bool {
    set.ranges
        .iter()
        .any(|(start, end)| code >= *start && code <= *end)
}

fn skip_char_class_matches(class: SkipCharClass, code: u32, syntax_table: &SyntaxTable) -> bool {
    match class {
        SkipCharClass::Alnum => {
            is_ascii_alpha_code(code) || is_ascii_digit_code(code) || non_ascii_alnum(code)
        }
        SkipCharClass::Alpha => is_ascii_alpha_code(code) || non_ascii_alpha(code),
        SkipCharClass::Blank => {
            code == b' ' as u32 || code == b'\t' as u32 || non_ascii_blank(code)
        }
        SkipCharClass::Cntrl => code < 0x20 || code == 0x7f,
        SkipCharClass::Digit => is_ascii_digit_code(code),
        SkipCharClass::Graph => {
            if code <= 0xff {
                code > b' ' as u32 && !(0x7f..=0xa0).contains(&code)
            } else {
                char::from_u32(code).is_some_and(|ch| !ch.is_control() && !ch.is_whitespace())
            }
        }
        SkipCharClass::Lower => char::from_u32(code).is_some_and(char::is_lowercase),
        SkipCharClass::Print => {
            if code <= 0xff {
                code >= b' ' as u32 && !(0x7f..=0x9f).contains(&code)
            } else {
                char::from_u32(code).is_some_and(|ch| !ch.is_control())
            }
        }
        SkipCharClass::Punct => {
            if code < 0x80 {
                code > b' ' as u32
                    && code < 0x7f
                    && !is_ascii_alpha_code(code)
                    && !is_ascii_digit_code(code)
            } else {
                syntax_table.char_syntax_code(code) != SyntaxClass::Word
            }
        }
        SkipCharClass::Space => syntax_table.char_syntax_code(code) == SyntaxClass::Whitespace,
        SkipCharClass::Upper => char::from_u32(code).is_some_and(char::is_uppercase),
        SkipCharClass::Xdigit => (code as u8).is_ascii_hexdigit() && code <= 0x7f,
        SkipCharClass::Ascii => code < 0x80,
        SkipCharClass::Word => syntax_table.char_syntax_code(code) == SyntaxClass::Word,
        SkipCharClass::Nonascii => code >= 0x80,
        // Neomacs stores raw byte characters as Emacs character codes in the
        // 0x3FFF80..0x3FFFFF range, matching GNU's BYTE8_TO_CHAR space.
        SkipCharClass::Unibyte => code <= 0xff || (0x3f_ff80..=0x3f_ffff).contains(&code),
        SkipCharClass::Multibyte => {
            !skip_char_class_matches(SkipCharClass::Unibyte, code, syntax_table)
        }
    }
}

fn skip_char_matches(set: &SkipCharsSet, code: u32, syntax_table: &SyntaxTable) -> bool {
    let in_set = set
        .classes
        .iter()
        .any(|class| skip_char_class_matches(*class, code, syntax_table))
        || skip_char_in_explicit_ranges(set, code);
    if set.negate { !in_set } else { in_set }
}

fn non_ascii_alpha(code: u32) -> bool {
    code >= 0x80 && char::from_u32(code).is_some_and(char::is_alphabetic)
}

fn is_ascii_alpha_code(code: u32) -> bool {
    code <= 0x7f && (code as u8).is_ascii_alphabetic()
}

fn is_ascii_digit_code(code: u32) -> bool {
    code <= 0x7f && (code as u8).is_ascii_digit()
}

fn non_ascii_alnum(code: u32) -> bool {
    code >= 0x80 && char::from_u32(code).is_some_and(char::is_alphanumeric)
}

fn non_ascii_blank(code: u32) -> bool {
    code >= 0x80
        && char::from_u32(code).is_some_and(|ch| ch.is_whitespace() && ch != '\n' && ch != '\r')
}

/// (skip-chars-forward STRING &optional LIM)
pub(crate) fn builtin_skip_chars_forward(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("skip-chars-forward", &args, 1)?;
    let set_codes = match args[0].as_lisp_string() {
        Some(string) => super::builtins::lisp_string_char_codes(string),
        None => {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("stringp"), args[0]],
            ));
        }
    };
    let char_set = parse_skip_chars_set(&set_codes)?;
    let current_id = ctx.buffers.current_buffer_id().ok_or_else(no_buffer)?;
    let (start_pos, pos, limit, moved_chars) = {
        let buf = ctx.buffers.get(current_id).ok_or_else(no_buffer)?;
        let syntax_table = SyntaxTable::for_buffer(buf);
        let lim_byte = if args.len() > 1 && !args[1].is_nil() {
            char_pos_to_byte(buf, expect_int(&args[1])?)
        } else {
            buf.zv_byte
        };
        let start_pos = buf.pt_byte;
        let mut pos = buf.pt_byte;
        let mut moved_chars = 0_i64;
        let limit = lim_byte.min(buf.zv_byte);

        while pos < limit {
            if let Some(code) = buf.char_code_after(pos) {
                if !skip_char_matches(&char_set, code, &syntax_table) {
                    break;
                }
                pos += buf
                    .char_after_emacs_len(pos)
                    .expect("char width should exist at valid point");
                moved_chars += 1;
            } else {
                break;
            }
        }

        (start_pos, pos, limit, moved_chars)
    };

    debug_assert!(pos >= start_pos || limit <= start_pos);
    let _ = ctx.buffers.goto_buffer_byte(current_id, pos);
    Ok(Value::fixnum(moved_chars))
}

/// (skip-chars-backward STRING &optional LIM)
pub(crate) fn builtin_skip_chars_backward(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("skip-chars-backward", &args, 1)?;
    let set_codes = match args[0].as_lisp_string() {
        Some(string) => super::builtins::lisp_string_char_codes(string),
        None => {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("stringp"), args[0]],
            ));
        }
    };
    let char_set = parse_skip_chars_set(&set_codes)?;
    let current_id = ctx.buffers.current_buffer_id().ok_or_else(no_buffer)?;
    let (pos, moved_chars) = {
        let buf = ctx.buffers.get(current_id).ok_or_else(no_buffer)?;
        let syntax_table = SyntaxTable::for_buffer(buf);
        let limit = if args.len() > 1 && !args[1].is_nil() {
            char_pos_to_byte(buf, expect_int(&args[1])?)
        } else {
            buf.begv_byte
        };
        let start_pos = buf.pt_byte;
        let mut pos = buf.pt_byte;
        let mut moved_chars = 0_i64;

        while pos > limit {
            // Find the character before `pos`.
            if let Some(code) = buf.char_code_before(pos) {
                if !skip_char_matches(&char_set, code, &syntax_table) {
                    break;
                }
                pos -= buf
                    .char_before_emacs_len(pos)
                    .expect("char width should exist before valid point");
                moved_chars -= 1;
            } else {
                break;
            }
        }

        debug_assert!(pos <= start_pos);
        (pos, moved_chars)
    };
    let _ = ctx.buffers.goto_buffer_byte(current_id, pos);
    Ok(Value::fixnum(moved_chars))
}

// ===========================================================================
// Mark and region
// ===========================================================================

/// (mark &optional FORCE) -> integer or signal
pub(crate) fn builtin_mark_nav(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    let _force = args.first().is_some_and(|v| v.is_truthy());
    let buf = eval.buffers.current_buffer().ok_or_else(no_buffer)?;
    match buf.mark() {
        Some(byte_pos) => Ok(Value::fixnum(byte_to_char_pos(buf, byte_pos))),
        None => Ok(Value::NIL),
    }
}

/// (region-beginning) -> integer
pub(crate) fn builtin_region_beginning(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("region-beginning", &args, 0)?;
    let buf = eval.buffers.current_buffer().ok_or_else(no_buffer)?;
    let mark = buf.mark().ok_or_else(|| {
        signal(
            "error",
            vec![Value::string(
                "The mark is not set now, so there is no region",
            )],
        )
    })?;
    let pt = clamp_byte_to_accessible(buf, buf.pt_byte);
    let mark = clamp_byte_to_accessible(buf, mark);
    let start = pt.min(mark);
    Ok(Value::fixnum(byte_to_char_pos(buf, start)))
}

/// (region-end) -> integer
pub(crate) fn builtin_region_end(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("region-end", &args, 0)?;
    let buf = eval.buffers.current_buffer().ok_or_else(no_buffer)?;
    let mark = buf.mark().ok_or_else(|| {
        signal(
            "error",
            vec![Value::string(
                "The mark is not set now, so there is no region",
            )],
        )
    })?;
    let pt = clamp_byte_to_accessible(buf, buf.pt_byte);
    let mark = clamp_byte_to_accessible(buf, mark);
    let end = pt.max(mark);
    Ok(Value::fixnum(byte_to_char_pos(buf, end)))
}

// ===========================================================================
// transient-mark-mode  (define-minor-mode in GNU simple.el)
// ===========================================================================

/// `(transient-mark-mode &optional ARG)` — toggle transient-mark-mode.
///
/// Matches GNU's define-minor-mode toggle logic:
/// - no arg or nil  → enable (set to t)
/// - positive number → enable
/// - zero or negative → disable (set to nil)
/// - 'toggle         → flip current value
pub(crate) fn builtin_transient_mark_mode(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 1 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("transient-mark-mode"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let sym_id = intern("transient-mark-mode");
    let current = eval
        .obarray
        .symbol_value("transient-mark-mode")
        .cloned()
        .unwrap_or(Value::NIL);

    let new_val = if args.is_empty() || args[0].is_nil() {
        // No arg or nil → enable
        Value::T
    } else if args[0].is_symbol_named("toggle") {
        // 'toggle → flip
        if current.is_truthy() {
            Value::NIL
        } else {
            Value::T
        }
    } else {
        // Numeric arg: positive → enable, zero/negative → disable.
        // Floats are truncated to integer first (GNU define-minor-mode behavior).
        match args[0].kind() {
            ValueKind::Fixnum(n) => {
                if n > 0 {
                    Value::T
                } else {
                    Value::NIL
                }
            }
            ValueKind::Float => {
                let truncated = args[0].xfloat() as i64;
                if truncated > 0 { Value::T } else { Value::NIL }
            }
            _ => Value::T,
        }
    };

    eval.obarray.set_symbol_value_id(sym_id, new_val);
    Ok(new_val)
}

// ===========================================================================
// Tests
// ===========================================================================
#[cfg(test)]
#[path = "navigation_test.rs"]
mod tests;
