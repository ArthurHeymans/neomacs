//! Indentation builtins for the Elisp interpreter.
//!
//! Implements stub versions of Emacs indentation primitives:
//! - `current-indentation`, `indent-to`, `current-column`, `move-to-column`
//! - `indent-line-to`, `indent-rigidly`, `newline-and-indent`,
//!   `tab-to-tab-stop`, `delete-indentation`
//!
//! Variables: `tab-width`, `indent-tabs-mode`, `standard-indent`, `tab-stop-list`

use super::error::{EvalResult, Flow, signal};
use super::symbol::Obarray;
use super::value::*;
use crate::buffer::{
    Buffer, CharLen, CharPos0, EmacsByteLen, EmacsBytePos, EmacsByteRange, TextExtent,
};
use crate::emacs_core::value::ValueKind;
use crate::heap_types::LispString;
use std::cell::Cell;

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
        _other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("fixnump"), *val],
        )),
    }
}

fn expect_wholenump(val: &Value) -> Result<usize, Flow> {
    match val.kind() {
        ValueKind::Fixnum(n) if n >= 0 => Ok(n as usize),
        _other => Err(signal(
            "wrong-type-argument",
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
    if byte < 0o40 || byte >= 0o177 { 4 } else { 1 }
}

pub(crate) fn display_advance_at(
    ctx: &mut super::eval::Context,
    buffer_id: crate::buffer::BufferId,
    byte: usize,
    column: usize,
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

    if let Some(next_visible) =
        super::xdisp::zero_width_invisible_run_end_byte(ctx, buffer_id, byte)?
    {
        if next_visible > byte {
            return Ok(Some(DisplayAdvance {
                next_byte: next_visible.min(end),
                width: 0,
                hard_newline: false,
                unbreakable_wide: false,
            }));
        }
    }

    if let Some((disp_width, run_end_byte)) = display_run_at(ctx, buffer_id, byte, column) {
        if run_end_byte > byte {
            return Ok(Some(DisplayAdvance {
                next_byte: run_end_byte.min(end),
                width: disp_width,
                hard_newline: false,
                unbreakable_wide: false,
            }));
        }
    }

    if let Some((comp_width, comp_end)) = composition_run_at(ctx, buffer_id, byte) {
        if comp_end > byte {
            return Ok(Some(DisplayAdvance {
                next_byte: comp_end.min(end),
                width: comp_width,
                hard_newline: false,
                unbreakable_wide: false,
            }));
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
    if let Some(f) = rel_prop.as_float() {
        if (0.0..=(i32::MAX as f64)).contains(&f) {
            return Some((f + 0.5) as usize);
        }
    }
    // `:align-to COL`: width = COL - col.
    let align_prop = super::plist::plist_get(plist, &qcalign).unwrap_or(Value::NIL);
    if let Some(n) = ranged_fixnum(align_prop, col as i64, align_to_max) {
        return Some((n as usize).saturating_sub(col));
    }
    if let Some(f) = align_prop.as_float() {
        if f >= col as f64 && f <= align_to_max as f64 {
            return Some(((f + 0.5) as usize).saturating_sub(col));
        }
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

        // A `display` property (text property or overlay) whose value is a string
        // or a `(space ...)` spec replaces the covered text for layout (GNU's
        // `current_column_1` / `Fmove_to_column` consult `display` specs via
        // `check_display_width`). Advance by the spec's display width over the
        // whole property/overlay run, atomically (no splitting a display run).
        if let Some((disp_width, run_end_byte)) = display_run_at(ctx, buffer_id, scan, column) {
            if run_end_byte > scan {
                previous_byte_pos = scan;
                previous_column = column;
                previous_code = None;
                column = column.saturating_add(disp_width);
                scan = run_end_byte.min(end);
                continue;
            }
        }

        // A `composition` property lays its covered characters out as the
        // composed glyphs (GNU's display scan via get_composition_id), so the
        // run advances by the glyphs' width over the composed character count.
        if let Some((comp_width, comp_end)) = composition_run_at(ctx, buffer_id, scan) {
            if comp_end > scan {
                previous_byte_pos = scan;
                previous_column = column;
                previous_code = None;
                column = column.saturating_add(comp_width);
                scan = comp_end.min(end);
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

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
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
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn is_horizontal_space(ch: char) -> bool {
    ch == ' ' || ch == '\t'
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
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
