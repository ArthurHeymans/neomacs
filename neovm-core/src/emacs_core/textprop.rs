//! Text property and overlay builtins for the Elisp interpreter.
//!
//! Bridges the buffer's `TextPropertyTable` and `OverlayList` to Elisp
//! functions like `put-text-property`, `make-overlay`, etc.

use super::builtins::builtin_copy_sequence;
use super::error::{EvalResult, Flow, signal};
use super::intern::{NIL_SYM_ID, T_SYM_ID, resolve_sym};
// storage imports removed — now using emacs_char directly
use super::plist;
use super::symbol::Obarray;
use super::value::*;
use crate::buffer::text_props::TextPropertyTable;
use crate::buffer::{BufferId, BufferManager};
use crate::emacs_core::SymId;
use crate::window::{FrameManager, WindowId};

pub(crate) fn init_textprop_vars(
    obarray: &mut crate::emacs_core::symbol::Obarray,
    _custom: &mut crate::emacs_core::custom::CustomManager,
) {
    obarray.set_symbol_value("default-text-properties", Value::NIL);
    obarray.make_special("default-text-properties");

    obarray.set_symbol_value("char-property-alias-alist", Value::NIL);
    obarray.make_special("char-property-alias-alist");

    obarray.set_symbol_value("inhibit-point-motion-hooks", Value::T);
    obarray.make_special("inhibit-point-motion-hooks");

    obarray.set_symbol_value(
        "text-property-default-nonsticky",
        Value::list(vec![
            Value::cons(Value::symbol("syntax-table"), Value::T),
            Value::cons(Value::symbol("display"), Value::T),
        ]),
    );
    obarray.make_special("text-property-default-nonsticky");
    // Mirrors GNU `Fmake_variable_buffer_local` (`data.c:2142-2207`):
    // flip the redirect tag to LOCALIZED, allocate a BLV, set
    // local_if_set = 1. Replaces the legacy `make_buffer_local`
    // helper which was destructive (set the redirect back to
    // PLAINVAL and orphaned the BLV).
    {
        let id = crate::emacs_core::intern::intern("text-property-default-nonsticky");
        let default = obarray
            .find_symbol_value(id)
            .unwrap_or(crate::emacs_core::value::Value::NIL);
        obarray.make_symbol_localized(id, default);
        obarray.set_blv_local_if_set(id, true);
    }
}

// ---------------------------------------------------------------------------
// Helpers (local to this module)
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
        _ if super::marker::is_marker(value) => super::marker::marker_position_as_int(value),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integerp"), *value],
        )),
    }
}

fn expect_int_eval(eval: &super::eval::Context, value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => {
            super::marker::marker_position_as_int_eval(eval, value)
        }
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integerp"), *value],
        )),
    }
}

fn expect_integer_or_marker(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => super::marker::marker_position_as_int(value),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

fn expect_integer_or_marker_eval(eval: &super::eval::Context, value: &Value) -> Result<i64, Flow> {
    super::position::fix_position_eval(eval, value)
}

fn expect_integer_or_marker_in_buffers(
    buffers: &BufferManager,
    value: &Value,
) -> Result<i64, Flow> {
    super::position::fix_position_with_buffers(buffers, value)
}

/// Text property keys are Lisp objects and are compared by identity, matching
/// GNU Emacs interval plists.
pub(crate) fn expect_property_key(value: &Value) -> Result<Value, Flow> {
    Ok(*value)
}

pub fn register_bootstrap_vars(obarray: &mut crate::emacs_core::symbol::Obarray) {
    obarray.set_symbol_value("default-text-properties", Value::NIL);
    obarray.set_symbol_value("char-property-alias-alist", Value::NIL);
    obarray.set_symbol_value("inhibit-point-motion-hooks", Value::T);
    obarray.set_symbol_value(
        "text-property-default-nonsticky",
        Value::list(vec![
            Value::cons(Value::symbol("syntax-table"), Value::T),
            Value::cons(Value::symbol("display"), Value::T),
        ]),
    );
}

fn current_textprop_variable_value(
    obarray: &Obarray,
    buffers: &BufferManager,
    name: &str,
) -> Option<Value> {
    if let Some(buf) = buffers.current_buffer()
        && let Some(binding) = buf.get_buffer_local_binding(name)
    {
        return binding.as_value();
    }
    obarray.symbol_value(name).copied()
}

fn plist_get_value(plist: Value, prop: Value) -> Option<Value> {
    let mut tail = plist;
    loop {
        if !tail.is_cons() {
            return None;
        };
        let pair_car = tail.cons_car();
        let pair_cdr = tail.cons_cdr();
        if !pair_cdr.is_cons() {
            return None;
        };
        if pair_car == prop {
            return Some(pair_cdr.cons_car());
        }
        tail = pair_cdr.cons_cdr();
    }
}

fn plist_slice_get_value(plist: &[(Value, Value)], prop: Value) -> Option<Value> {
    plist
        .iter()
        .find_map(|(key, value)| eq_value(key, &prop).then_some(*value))
}

fn assq_rest(list: Value, prop: Value) -> Option<Value> {
    let mut cursor = list;
    while cursor.is_cons() {
        let pair_car = cursor.cons_car();
        let pair_cdr = cursor.cons_cdr();
        if pair_car.is_cons() {
            let entry_car = pair_car.cons_car();
            let entry_cdr = pair_car.cons_cdr();
            if entry_car == prop {
                return Some(entry_cdr);
            }
        }
        cursor = pair_cdr;
    }
    None
}

fn symbol_id_for_property_lookup(value: Value) -> Option<SymId> {
    match value.kind() {
        ValueKind::Nil => Some(NIL_SYM_ID),
        ValueKind::T => Some(T_SYM_ID),
        ValueKind::Symbol(id) => Some(id),
        _ => None,
    }
}

fn lookup_char_property_from_direct<F>(
    obarray: &Obarray,
    buffers: &BufferManager,
    mut direct_get: F,
    prop: Value,
    textprop: bool,
) -> Value
where
    F: FnMut(Value) -> Option<Value>,
{
    if let Some(value) = direct_get(prop) {
        return value;
    }

    let mut fallback = Value::NIL;

    if let Some(category) = direct_get(Value::symbol("category"))
        && let Some(category_id) = symbol_id_for_property_lookup(category)
        && let Some(prop_id) = symbol_id_for_property_lookup(prop)
        && let Some(value) = obarray.get_property_id(category_id, prop_id)
    {
        fallback = value;
    }

    if !fallback.is_nil() {
        return fallback;
    }

    if let Some(aliases) =
        current_textprop_variable_value(obarray, buffers, "char-property-alias-alist")
            .and_then(|value| assq_rest(value, prop))
    {
        let mut cursor = aliases;
        while cursor.is_cons() {
            let pair_car = cursor.cons_car();
            let pair_cdr = cursor.cons_cdr();
            if let Some(value) = direct_get(pair_car)
                && !value.is_nil()
            {
                return value;
            }
            cursor = pair_cdr;
        }
    }

    if textprop
        && let Some(defaults) =
            current_textprop_variable_value(obarray, buffers, "default-text-properties")
        && defaults.is_cons()
        && let Some(value) = plist_get_value(defaults, prop)
    {
        return value;
    }

    fallback
}

fn lookup_string_text_property(
    obarray: &Obarray,
    buffers: &BufferManager,
    table: &TextPropertyTable,
    char_pos: usize,
    prop: Value,
) -> Value {
    lookup_char_property_from_direct(
        obarray,
        buffers,
        |name| table.get_property(char_pos, name),
        prop,
        true,
    )
}

fn lookup_buffer_text_property_at_char_pos(
    obarray: &Obarray,
    buffers: &BufferManager,
    buf: &crate::buffer::buffer::Buffer,
    char_pos: usize,
    prop: Value,
) -> Value {
    lookup_char_property_from_direct(
        obarray,
        buffers,
        |name| {
            buf.text
                .text_props_get_property(buf.text.char_to_byte(char_pos), name)
        },
        prop,
        true,
    )
}

pub(crate) fn lookup_buffer_text_property(
    obarray: &Obarray,
    buffers: &BufferManager,
    buf: &crate::buffer::buffer::Buffer,
    byte_pos: usize,
    prop: Value,
) -> Value {
    lookup_buffer_text_property_at_char_pos(
        obarray,
        buffers,
        buf,
        buf.text.byte_to_char(byte_pos),
        prop,
    )
}

fn lookup_overlay_property(
    obarray: &Obarray,
    buffers: &BufferManager,
    overlay_val: Value,
    prop: Value,
) -> Value {
    let plist = overlay_val
        .as_overlay_data()
        .map_or(Value::NIL, |d| d.plist);
    lookup_char_property_from_direct(
        obarray,
        buffers,
        |name| plist_get_value(plist, name),
        prop,
        false,
    )
}

/// Convert a 1-based Elisp char position to a 0-based byte position.
///
/// This is only valid after GNU-style range validation.  Text-property
/// builtins must not clamp positions: GNU `validate_interval_range` signals
/// `args-out-of-range` for invalid positions.
fn elisp_pos_to_byte(buf: &crate::buffer::buffer::Buffer, pos: i64) -> usize {
    debug_assert!(pos >= 1);
    buf.text.char_to_byte((pos - 1) as usize)
}

fn elisp_pos_to_byte_clipped_full(buf: &crate::buffer::buffer::Buffer, pos: i64) -> usize {
    let max = buf.text.char_count() as i64 + 1;
    let clipped = pos.clamp(1, max);
    elisp_pos_to_byte(buf, clipped)
}

fn elisp_range_to_byte_clipped_full(
    buf: &crate::buffer::buffer::Buffer,
    mut beg: i64,
    mut end: i64,
) -> (usize, usize) {
    if beg > end {
        std::mem::swap(&mut beg, &mut end);
    }
    let max = buf.text.char_count() as i64 + 1;
    let clipped_beg = beg.clamp(1, max);
    let clipped_end = end.clamp(clipped_beg, max);
    (
        elisp_pos_to_byte(buf, clipped_beg),
        elisp_pos_to_byte(buf, clipped_end),
    )
}

fn args_out_of_range_point(pos: i64) -> Flow {
    signal("args-out-of-range", vec![Value::fixnum(pos)])
}

fn args_out_of_range_range(begin0: Value, end0: Value) -> Flow {
    signal("args-out-of-range", vec![begin0, end0])
}

pub(crate) fn validate_string_point(
    s: &crate::heap_types::LispString,
    pos: i64,
) -> Result<usize, Flow> {
    validate_string_point_raw(s, pos, Value::fixnum(pos))
}

pub(crate) fn validate_string_point_raw(
    s: &crate::heap_types::LispString,
    pos: i64,
    pos0: Value,
) -> Result<usize, Flow> {
    let len = s.schars() as i64;
    if !(0 <= pos && pos <= len) {
        return Err(args_out_of_range_range(pos0, pos0));
    }
    Ok(pos as usize)
}

fn validate_string_range(
    s: &crate::heap_types::LispString,
    beg: i64,
    end: i64,
    beg0: Value,
    end0: Value,
) -> Result<Option<(usize, usize)>, Flow> {
    if beg == end {
        return Ok(None);
    }
    let (start, finish) = if beg > end { (end, beg) } else { (beg, end) };
    let len = s.schars() as i64;
    if !(0 <= start && start <= finish && finish <= len) {
        return Err(args_out_of_range_range(beg0, end0));
    }
    Ok(Some((start as usize, finish as usize)))
}

pub(crate) fn validate_buffer_point(
    buf: &crate::buffer::buffer::Buffer,
    pos: i64,
) -> Result<usize, Flow> {
    validate_buffer_point_raw(buf, pos, Value::fixnum(pos))
}

pub(crate) fn validate_buffer_point_raw(
    buf: &crate::buffer::buffer::Buffer,
    pos: i64,
    pos0: Value,
) -> Result<usize, Flow> {
    let point_min = buf.point_min_char() as i64 + 1;
    let point_max = buf.point_max_char() as i64 + 1;
    if !(point_min <= pos && pos <= point_max) {
        return Err(args_out_of_range_range(pos0, pos0));
    }
    Ok(elisp_pos_to_byte(buf, pos))
}

fn validate_buffer_property_range(
    buf: &crate::buffer::buffer::Buffer,
    beg: i64,
    end: i64,
    beg0: Value,
    end0: Value,
) -> Result<Option<(usize, usize)>, Flow> {
    if beg == end {
        return Ok(None);
    }
    let (start, finish) = if beg > end { (end, beg) } else { (beg, end) };
    let point_min = buf.point_min_char() as i64 + 1;
    let point_max = buf.point_max_char() as i64 + 1;
    if !(point_min <= start && start <= finish && finish <= point_max) {
        return Err(args_out_of_range_range(beg0, end0));
    }
    Ok(Some((
        elisp_pos_to_byte(buf, start),
        elisp_pos_to_byte(buf, finish),
    )))
}

/// Convert a 0-based byte position to a 1-based Elisp char position.
pub(crate) fn byte_to_elisp_pos(buf: &crate::buffer::buffer::Buffer, byte_pos: usize) -> i64 {
    buf.text.byte_to_char(byte_pos) as i64 + 1
}

/// Resolve the optional OBJECT argument to a buffer id.
/// If nil or absent, uses the current buffer.
fn resolve_buffer_id(
    eval: &super::eval::Context,
    object: Option<&Value>,
) -> Result<BufferId, Flow> {
    resolve_buffer_id_in_buffers(&eval.buffers, object)
}

fn resolve_buffer_id_in_buffers(
    buffers: &BufferManager,
    object: Option<&Value>,
) -> Result<BufferId, Flow> {
    match object {
        None => buffers
            .current_buffer()
            .map(|b| b.id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")])),
        Some(v) if v.is_nil() => buffers
            .current_buffer()
            .map(|b| b.id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")])),
        Some(v) if v.is_buffer() => v
            .as_buffer_id()
            .ok_or_else(|| signal("error", vec![Value::string("Invalid buffer")])),
        Some(other) => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("bufferp"), *other],
        )),
    }
}

fn resolve_text_property_buffer_id_in_buffers(
    buffers: &BufferManager,
    object: Option<&Value>,
) -> Result<BufferId, Flow> {
    match object {
        None => buffers
            .current_buffer()
            .map(|b| b.id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")])),
        Some(v) if v.is_nil() => buffers
            .current_buffer()
            .map(|b| b.id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")])),
        Some(v) if v.is_buffer() => v
            .as_buffer_id()
            .ok_or_else(|| signal("error", vec![Value::string("Invalid buffer")])),
        Some(other) => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("buffer-or-string-p"), *other],
        )),
    }
}

fn resolve_char_property_target_in_state(
    frames: Option<&FrameManager>,
    buffers: &BufferManager,
    object: Option<&Value>,
) -> Result<(BufferId, Option<WindowId>), Flow> {
    match object {
        None => buffers
            .current_buffer()
            .map(|b| (b.id, None))
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")])),
        Some(v) if v.is_nil() => buffers
            .current_buffer()
            .map(|b| (b.id, None))
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")])),
        Some(v) if v.is_buffer() => v
            .as_buffer_id()
            .map(|id| (id, None))
            .ok_or_else(|| signal("error", vec![Value::string("Invalid buffer")])),
        Some(v) if v.is_window() => {
            let Some(frames) = frames else {
                return Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("buffer-or-string-p"), *v],
                ));
            };
            let wid = WindowId(v.as_window_id().expect("window value has an id"));
            let window = frames.lookup_window(wid).ok_or_else(|| {
                signal(
                    "wrong-type-argument",
                    vec![Value::symbol("window-live-p"), *v],
                )
            })?;
            let buffer_id = window.buffer_id().ok_or_else(|| {
                signal(
                    "wrong-type-argument",
                    vec![Value::symbol("window-live-p"), *v],
                )
            })?;
            Ok((buffer_id, Some(wid)))
        }
        Some(other) => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("buffer-or-string-p"), *other],
        )),
    }
}

fn current_buffer_id_in_buffers(buffers: &BufferManager) -> Result<BufferId, Flow> {
    buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))
}

fn expect_overlay(value: &Value) -> Result<Value, Flow> {
    if !value.is_overlay() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("overlayp"), *value],
        ));
    }
    Ok(*value)
}

fn resolve_overlay_buffer_id(overlay_val: Value) -> Option<BufferId> {
    overlay_val.as_overlay_data().and_then(|d| d.buffer)
}

fn ensure_marker_points_into_buffer(
    buffers: &BufferManager,
    value: &Value,
    buffer_id: BufferId,
) -> Result<(), Flow> {
    let Some((Some(marker_buffer_id), _, _)) = super::marker::marker_logical_fields(value) else {
        return Ok(());
    };
    if buffers.get(marker_buffer_id).is_none() {
        return Ok(());
    }
    if marker_buffer_id == buffer_id {
        return Ok(());
    }
    Err(signal(
        "error",
        vec![Value::string("Marker points into wrong buffer"), *value],
    ))
}

/// Check if the OBJECT argument is a string.  Returns Some(Value) if so.
pub(crate) fn is_string_object(object: Option<&Value>) -> Option<Value> {
    match object {
        Some(v) if v.is_string() => Some(*v),
        _ => None,
    }
}

pub(crate) fn string_char_to_elisp_pos(_s: &crate::heap_types::LispString, char_pos: usize) -> i64 {
    char_pos as i64
}

/// Write back a modified TextPropertyTable to string text properties.
pub(crate) fn save_string_props_for_value(value: Value, table: TextPropertyTable) {
    set_string_text_properties_table_for_value(value, table);
}

/// Iterate a plist (alternating key value key value ...) from a list or vec.
/// Returns pairs of (property-name, value).
fn plist_pairs(plist: &Value) -> Result<Vec<(Value, Value)>, Flow> {
    if plist.is_nil() {
        return Ok(Vec::new());
    }
    if !plist.is_cons() {
        return Ok(vec![(expect_property_key(plist)?, Value::NIL)]);
    }

    let mut pairs = Vec::new();
    let mut tail = *plist;
    loop {
        if !tail.is_cons() {
            break;
        }
        let name = tail.cons_car();
        let rest = tail.cons_cdr();
        if !rest.is_cons() {
            return Err(signal(
                "error",
                vec![Value::string("Odd length text property list")],
            ));
        }
        pairs.push((expect_property_key(&name)?, rest.cons_car()));
        tail = rest.cons_cdr();
    }
    Ok(pairs)
}

fn plist_names_for_remove(plist: Value) -> Vec<Value> {
    let mut names = Vec::new();
    let mut tail = plist;
    while tail.is_cons() {
        names.push(tail.cons_car());
        tail = tail.cons_cdr();
        if tail.is_cons() {
            tail = tail.cons_cdr();
        } else {
            break;
        }
    }
    names
}

fn list_names_for_remove(list: Value) -> Vec<Value> {
    let mut names = Vec::new();
    let mut tail = list;
    while tail.is_cons() {
        names.push(tail.cons_car());
        tail = tail.cons_cdr();
    }
    names
}

/// Convert ordered property pairs to an Elisp plist.
/// Preserves the order from the property interval (matching GNU Emacs behavior).
fn ordered_pairs_to_plist(pairs: &[(Value, Value)]) -> Value {
    let mut items = Vec::new();
    for (key, val) in pairs {
        items.push(*key);
        items.push(*val);
    }
    Value::list(items)
}

// ===========================================================================
// Text property builtins
// ===========================================================================

/// GNU `verify_interval_modification` (textprop.c:2184), restricted to the
/// read-only check.  Walks intervals overlapping `[byte_start, byte_end)`
/// in BUF_ID and signals `text-read-only` if any interval has a non-nil
/// `read-only` property that is not silenced by either the
/// `inhibit-read-only` interval property or the dynamic
/// `inhibit-read-only` variable.
pub(crate) fn verify_text_read_only_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    buf_id: BufferId,
    byte_start: usize,
    byte_end: usize,
) -> Result<(), Flow> {
    if byte_start >= byte_end {
        return Ok(());
    }
    let Some(buf) = buffers.get(buf_id) else {
        return Ok(());
    };
    let inhibit = buf
        .get_buffer_local("inhibit-read-only")
        .unwrap_or_else(|| {
            obarray
                .symbol_value("inhibit-read-only")
                .copied()
                .unwrap_or(Value::NIL)
        });
    // INTERVAL_GENERALLY_WRITABLE_P: when inhibit-read-only is non-nil
    // and not a list, every interval is writable regardless of its
    // read-only property.  GNU intervals.h:210.
    if !inhibit.is_nil() && !inhibit.is_cons() {
        return Ok(());
    }
    let read_only_sym = Value::symbol("read-only");
    let inhibit_sym = Value::symbol("inhibit-read-only");
    buf.text
        .text_props_try_for_each_interval_in_range(byte_start, byte_end, |_, _, plist| {
            let read_only = lookup_char_property_from_direct(
                obarray,
                buffers,
                |name| plist_slice_get_value(plist, name),
                read_only_sym,
                true,
            );
            if read_only.is_nil() {
                return Ok::<(), Flow>(());
            }
            // INTERVAL_EXPRESSLY_WRITABLE_P (intervals.h:217).
            let express_inhibit = plist_slice_get_value(plist, inhibit_sym).unwrap_or(Value::NIL);
            if !express_inhibit.is_nil() {
                return Ok(());
            }
            if inhibit.is_cons() && value_in_list(read_only, inhibit) {
                return Ok(());
            }
            let args = if read_only.is_string() {
                vec![read_only]
            } else {
                vec![]
            };
            Err(signal("text-read-only", args))
        })?;
    Ok(())
}

fn value_in_list(needle: Value, list: Value) -> bool {
    let mut cursor = list;
    while cursor.is_cons() {
        if eq_value(&cursor.cons_car(), &needle) {
            return true;
        }
        cursor = cursor.cons_cdr();
    }
    false
}

/// Resolve OBJECT-arg to a buffer and verify text-read-only over the
/// `[BEG, END)` byte range.  No-op if OBJECT is a string (text properties
/// on strings have no read-only enforcement in GNU either).
fn verify_property_change_read_only(
    eval: &mut super::eval::Context,
    args: &[Value],
    object_arg_idx: usize,
) -> Result<(), Flow> {
    if is_string_object(args.get(object_arg_idx)).is_some() {
        return Ok(());
    }
    if args.len() < 2 {
        return Ok(());
    }
    let beg = expect_integer_or_marker_in_buffers(&eval.buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(&eval.buffers, &args[1])?;
    let buf_id =
        resolve_text_property_buffer_id_in_buffers(&eval.buffers, args.get(object_arg_idx))?;
    let (byte_beg, byte_end) = {
        let buf = eval
            .buffers
            .get(buf_id)
            .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
        let Some((a, b)) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
            return Ok(());
        };
        (a, b)
    };
    verify_text_read_only_in_state(&eval.obarray, &eval.buffers, buf_id, byte_beg, byte_end)
}

/// GNU `verify_interval_modification` modification-hooks branch
/// (textprop.c:2289-2363, 2342-2353): walk intervals overlapping
/// `[byte_start, byte_end)` and call each interval's `modification-hooks`
/// list (deduplicating consecutive identical hook lists).  Used by the
/// property-change DEFUNs, which in GNU funnel through
/// `modify_text_properties` -> `prepare_to_modify_buffer_1` ->
/// `verify_interval_modification`.
fn run_interval_modification_hooks(
    eval: &mut super::eval::Context,
    args: &[Value],
    object_arg_idx: usize,
) -> Result<(), Flow> {
    if is_string_object(args.get(object_arg_idx)).is_some() {
        return Ok(());
    }
    if args.len() < 2 {
        return Ok(());
    }
    if super::editfns::inhibit_modification_hooks(eval) {
        return Ok(());
    }
    let beg = expect_integer_or_marker_in_buffers(&eval.buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(&eval.buffers, &args[1])?;
    let buf_id =
        resolve_text_property_buffer_id_in_buffers(&eval.buffers, args.get(object_arg_idx))?;
    let (byte_start, byte_end, lisp_start, lisp_end, hook_lists) = {
        let Some(buf) = eval.buffers.get(buf_id) else {
            return Ok(());
        };
        let Some((a, b)) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
            return Ok(());
        };
        let lisp_a = buf.text.emacs_byte_to_char(a) as i64 + 1;
        let lisp_b = buf.text.emacs_byte_to_char(b) as i64 + 1;
        let mod_sym = Value::symbol("modification-hooks");
        let mut prev: Option<Value> = None;
        let mut hooks: Vec<Value> = Vec::new();
        let _ = buf
            .text
            .text_props_try_for_each_interval_in_range(a, b, |_, _, plist| {
                let mh = plist_slice_get_value(plist, mod_sym).unwrap_or(Value::NIL);
                if mh.is_nil() {
                    return Ok::<(), ()>(());
                }
                if let Some(p) = prev
                    && eq_value(&p, &mh)
                {
                    return Ok(());
                }
                prev = Some(mh);
                hooks.push(mh);
                Ok(())
            });
        (a, b, lisp_a, lisp_b, hooks)
    };
    let _ = (byte_start, byte_end);
    if hook_lists.is_empty() {
        return Ok(());
    }
    call_text_property_hook_lists(eval, hook_lists, lisp_start, lisp_end)
}

fn call_text_property_hook_lists(
    eval: &mut super::eval::Context,
    hook_lists: Vec<Value>,
    lisp_start: i64,
    lisp_end: i64,
) -> Result<(), Flow> {
    if hook_lists.is_empty() {
        return Ok(());
    }
    let start_v = Value::fixnum(lisp_start);
    let end_v = Value::fixnum(lisp_end);
    let specpdl_count = eval.specpdl.len();
    eval.specbind(
        super::intern::intern("inhibit-modification-hooks"),
        Value::T,
    );
    let result = (|| -> Result<(), Flow> {
        for hook_list in hook_lists {
            let mut cursor = hook_list;
            while cursor.is_cons() {
                let fn_v = cursor.cons_car();
                eval.apply(fn_v, vec![start_v, end_v])?;
                cursor = cursor.cons_cdr();
            }
        }
        Ok(())
    })();
    eval.unbind_to(specpdl_count);
    result
}

/// GNU `verify_interval_modification` for buffer text changes.
///
/// This is the interval-hook part of `prepare_to_modify_buffer_1`: for
/// non-empty changes, call `modification-hooks` before the text is changed;
/// for insertions, record `insert-behind-hooks` and `insert-in-front-hooks`
/// so `signal_after_change` can replay them after the inserted text exists.
pub(crate) fn prepare_interval_modification_for_change(
    eval: &mut super::eval::Context,
    buf_id: BufferId,
    byte_start: usize,
    byte_end: usize,
) -> Result<(), Flow> {
    eval.interval_insert_behind_hooks = Value::NIL;
    eval.interval_insert_in_front_hooks = Value::NIL;

    if byte_start == byte_end {
        record_interval_insert_hooks(eval, buf_id, byte_start);
        return Ok(());
    }

    if super::editfns::inhibit_modification_hooks(eval) {
        return Ok(());
    }

    let (lisp_start, lisp_end, hook_lists) = {
        let Some(buf) = eval.buffers.get(buf_id) else {
            return Ok(());
        };
        let start = byte_start.min(byte_end);
        let end = byte_start.max(byte_end);
        let lisp_start = buf.text.emacs_byte_to_char(start) as i64 + 1;
        let lisp_end = buf.text.emacs_byte_to_char(end) as i64 + 1;
        let mod_sym = Value::symbol("modification-hooks");
        let mut prev: Option<Value> = None;
        let mut hooks = Vec::new();
        let _ = buf
            .text
            .text_props_try_for_each_interval_in_range(start, end, |_, _, plist| {
                let mh = plist_slice_get_value(plist, mod_sym).unwrap_or(Value::NIL);
                if mh.is_nil() {
                    return Ok::<(), ()>(());
                }
                if let Some(p) = prev
                    && eq_value(&p, &mh)
                {
                    return Ok(());
                }
                prev = Some(mh);
                hooks.push(mh);
                Ok(())
            });
        (lisp_start, lisp_end, hooks)
    };

    call_text_property_hook_lists(eval, hook_lists, lisp_start, lisp_end)
}

fn record_interval_insert_hooks(
    eval: &mut super::eval::Context,
    buf_id: BufferId,
    byte_pos: usize,
) {
    let Some(buf) = eval.buffers.get(buf_id) else {
        return;
    };
    let behind_sym = Value::symbol("insert-behind-hooks");
    let front_sym = Value::symbol("insert-in-front-hooks");

    if byte_pos > buf.begv_byte
        && let Some(prev_len) = buf.char_before_storage_len(byte_pos)
    {
        let prev_byte = byte_pos.saturating_sub(prev_len);
        if let Some(hooks) = buf.text.text_props_get_property(prev_byte, behind_sym)
            && !hooks.is_nil()
        {
            eval.interval_insert_behind_hooks = hooks;
        }
    }

    if byte_pos < buf.zv_byte
        && let Some(hooks) = buf.text.text_props_get_property(byte_pos, front_sym)
        && !hooks.is_nil()
    {
        eval.interval_insert_in_front_hooks = hooks;
    }
}

/// GNU `report_interval_modification`: run insert text-property hooks after
/// insertion, passing the inserted character range.
pub(crate) fn report_interval_modification(
    eval: &mut super::eval::Context,
    lisp_start: i64,
    lisp_end: i64,
) -> Result<(), Flow> {
    let behind = eval.interval_insert_behind_hooks;
    let front = eval.interval_insert_in_front_hooks;
    if !behind.is_nil() {
        call_text_property_hook_lists(eval, vec![behind], lisp_start, lisp_end)?;
    }
    if !front.is_nil() && !eq_value(&front, &behind) {
        call_text_property_hook_lists(eval, vec![front], lisp_start, lisp_end)?;
    }
    Ok(())
}

/// (put-text-property BEG END PROP VAL &optional OBJECT)
pub(crate) fn builtin_put_text_property(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    verify_property_change_read_only(eval, &args, 4)?;
    run_interval_modification_hooks(eval, &args, 4)?;
    let result = builtin_put_text_property_in_buffers(&mut eval.buffers, args.clone())?;
    Ok(result)
}

pub(crate) fn builtin_put_text_property_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("put-text-property", &args, 4)?;
    expect_max_args("put-text-property", &args, 5)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let prop = expect_property_key(&args[2])?;
    let val = args[3];

    if let Some(str_val) = is_string_object(args.get(4)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let Some((char_beg, char_end)) = validate_string_range(s, beg, end, args[0], args[1])?
        else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        table.put_property(char_beg, char_end, prop, val);
        save_string_props_for_value(str_val, table);
        return Ok(Value::NIL);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(4))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some((byte_beg, byte_end)) =
        validate_buffer_property_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };
    if buffers
        .put_buffer_text_property(buf_id, byte_beg, byte_end, prop, val)
        .unwrap_or(false)
    {
        let _ = buffers.record_buffer_text_property_modification(buf_id);
    }
    Ok(Value::NIL)
}

/// (get-text-property POS PROP &optional OBJECT)
pub(crate) fn builtin_get_text_property(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_get_text_property_in_state(&eval.obarray, &eval.buffers, args)
}

pub(crate) fn builtin_get_text_property_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("get-text-property", &args, 2)?;
    expect_max_args("get-text-property", &args, 3)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let prop = expect_property_key(&args[1])?;

    if let Some(str_val) = is_string_object(args.get(2)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let char_pos = validate_string_point_raw(s, pos, args[0])?;
        if char_pos == s.schars() {
            return Ok(Value::NIL);
        }
        if let Some(table) = get_string_text_properties_table_for_value(str_val) {
            return Ok(lookup_string_text_property(
                obarray, buffers, &table, char_pos, prop,
            ));
        }
        return Ok(lookup_char_property_from_direct(
            obarray,
            buffers,
            |_| None,
            prop,
            true,
        ));
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(2))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = validate_buffer_point_raw(buf, pos, args[0])?;
    if byte_pos == buf.text.char_to_byte(buf.text.char_count()) {
        return Ok(Value::NIL);
    }
    Ok(lookup_buffer_text_property(
        obarray, buffers, buf, byte_pos, prop,
    ))
}

pub(crate) fn buffer_overlay_property_at_byte_pos(
    obarray: &Obarray,
    buffers: &BufferManager,
    buf: &crate::buffer::buffer::Buffer,
    byte_pos: usize,
    prop: Value,
    window_id: Option<WindowId>,
) -> Option<(Value, Value)> {
    let mut overlays = buf.overlays.overlays_at(byte_pos);
    buf.overlays
        .sort_overlay_ids_by_priority_desc(&mut overlays);
    for overlay in overlays {
        if let Some(wid) = window_id {
            let window_prop =
                lookup_overlay_property(obarray, buffers, overlay, Value::symbol("window"));
            if window_prop
                .as_window_id()
                .is_some_and(|overlay_wid| overlay_wid != wid.0)
            {
                continue;
            }
        }
        let value = lookup_overlay_property(obarray, buffers, overlay, prop);
        if !value.is_nil() {
            return Some((value, overlay));
        }
    }
    None
}

pub(crate) fn buffer_overlay_property_for_inserted_char_at_byte_pos(
    buf: &crate::buffer::buffer::Buffer,
    byte_pos: usize,
    prop: Value,
) -> Option<(Value, Value)> {
    let overlay_id = buf
        .overlays
        .highest_priority_overlay_for_inserted_char(byte_pos, &prop)?;
    let value = buf.overlays.overlay_get(overlay_id, &prop)?;
    Some((value, overlay_id))
}

/// (get-char-property POS PROP &optional OBJECT)
/// For strings, same as get-text-property (no overlays).
pub(crate) fn builtin_get_char_property(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_get_char_property_with_frames(&eval.obarray, &eval.buffers, Some(&eval.frames), args)
}

pub(crate) fn builtin_get_char_property_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    builtin_get_char_property_with_frames(obarray, buffers, None, args)
}

fn builtin_get_char_property_with_frames(
    obarray: &Obarray,
    buffers: &BufferManager,
    frames: Option<&FrameManager>,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("get-char-property", &args, 2)?;
    expect_max_args("get-char-property", &args, 3)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let prop = expect_property_key(&args[1])?;

    if is_string_object(args.get(2)).is_some() {
        return builtin_get_text_property_in_state(obarray, buffers, args);
    }

    let (buf_id, window_id) = resolve_char_property_target_in_state(frames, buffers, args.get(2))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
    let byte_pos = validate_buffer_point_raw(buf, pos, args[0])?;
    if byte_pos == buf.text.char_to_byte(buf.text.char_count()) {
        return Ok(Value::NIL);
    }

    if let Some((value, _overlay_id)) =
        buffer_overlay_property_at_byte_pos(obarray, buffers, buf, byte_pos, prop, window_id)
    {
        return Ok(value);
    }

    Ok(lookup_buffer_text_property(
        obarray, buffers, buf, byte_pos, prop,
    ))
}

/// (add-text-properties BEG END PROPS &optional OBJECT)
pub(crate) fn builtin_add_text_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    verify_property_change_read_only(eval, &args, 3)?;
    run_interval_modification_hooks(eval, &args, 3)?;
    let result = builtin_add_text_properties_in_buffers(&mut eval.buffers, args.clone())?;
    Ok(result)
}

pub(crate) fn builtin_add_text_properties_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("add-text-properties", &args, 3)?;
    expect_max_args("add-text-properties", &args, 4)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let pairs = plist_pairs(&args[2])?;

    if let Some(str_val) = is_string_object(args.get(3)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let Some((char_beg, char_end)) = validate_string_range(s, beg, end, args[0], args[1])?
        else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let mut any_changed = false;
        for (name, val) in pairs {
            if table.put_property(char_beg, char_end, name, val) {
                any_changed = true;
            }
        }
        save_string_props_for_value(str_val, table);
        return Ok(if any_changed { Value::T } else { Value::NIL });
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(3))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some((byte_beg, byte_end)) =
        validate_buffer_property_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };
    let mut any_changed = false;
    for (name, val) in pairs {
        if buffers
            .put_buffer_text_property(buf_id, byte_beg, byte_end, name, val)
            .unwrap_or(false)
        {
            any_changed = true;
        }
    }
    if any_changed {
        let _ = buffers.record_buffer_text_property_modification(buf_id);
    }
    Ok(if any_changed { Value::T } else { Value::NIL })
}

fn is_anonymous_face_plist(v: &Value) -> bool {
    // GNU treats a cons whose car is a keyword as an anonymous face plist
    // (e.g. (:foreground "red")), not a list of faces.
    v.is_cons() && v.cons_car().is_keyword()
}

fn improper_list_tail(list: Value) -> Value {
    let mut tail = list;
    let mut tortoise = list;
    let mut step = 0u64;
    while tail.is_cons() {
        tail = tail.cons_cdr();
        step += 1;
        if step % 2 == 0 {
            if tortoise.is_cons() {
                tortoise = tortoise.cons_cdr();
            }
            if tortoise.bits() == tail.bits() {
                return list;
            }
        }
    }
    tail
}

fn merge_face_property(
    existing: Option<Value>,
    new_face: Value,
    append: bool,
) -> Result<Value, Flow> {
    let Some(existing_value) = existing else {
        return Ok(new_face);
    };
    if existing_value.is_nil() {
        return Ok(new_face);
    }

    if existing_value.is_cons() && !is_anonymous_face_plist(&existing_value) {
        if append {
            if let Some(mut items) = list_to_vec(&existing_value) {
                items.push(new_face);
                return Ok(Value::list(items));
            }
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("listp"), improper_list_tail(existing_value)],
            ));
        }
        return Ok(Value::cons(new_face, existing_value));
    }

    Ok(if append {
        Value::list(vec![existing_value, new_face])
    } else {
        Value::list(vec![new_face, existing_value])
    })
}

/// `(add-face-text-property START END FACE &optional APPENDP OBJECT)`
pub(crate) fn builtin_add_face_text_property(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    verify_property_change_read_only(eval, &args, 4)?;
    run_interval_modification_hooks(eval, &args, 4)?;
    let result = builtin_add_face_text_property_in_buffers(&mut eval.buffers, args.clone())?;
    Ok(result)
}

pub(crate) fn builtin_add_face_text_property_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("add-face-text-property", &args, 3)?;
    expect_max_args("add-face-text-property", &args, 5)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let new_face = args[2];
    let append = args.get(3).is_some_and(|v| v.is_truthy());

    let object = args.get(4);

    if let Some(str_val) = is_string_object(object) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let Some((char_beg, char_end)) = validate_string_range(s, beg, end, args[0], args[1])?
        else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        // GNU iterates intervals in [beg, end); per interval, fetch its existing
        // face value and merge. Walk the range segment-by-segment.
        let mut seg_start = char_beg;
        while seg_start < char_end {
            let seg_end = match table.next_property_change(seg_start) {
                Some(p) if p < char_end => p,
                _ => char_end,
            };
            let existing = table.get_property(seg_start, Value::symbol("face"));
            let merged = merge_face_property(existing, new_face, append)?;
            table.put_property(seg_start, seg_end, Value::symbol("face"), merged);
            seg_start = seg_end;
        }
        save_string_props_for_value(str_val, table);
        return Ok(Value::NIL);
    }

    let buf_id = match object {
        None => buffers
            .current_buffer()
            .map(|b| b.id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")])),
        Some(v) if v.is_nil() => buffers
            .current_buffer()
            .map(|b| b.id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")])),
        Some(v) if v.is_buffer() => v
            .as_buffer_id()
            .ok_or_else(|| signal("error", vec![Value::string("Invalid buffer")])),
        Some(other) => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("buffer-or-string-p"), *other],
        )),
    }?;

    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
    let Some((byte_beg, byte_end)) =
        validate_buffer_property_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };
    // GNU iterates intervals in [beg, end); per interval, fetch its existing
    // face value and merge. Walk the range segment-by-segment to preserve any
    // heterogeneous face properties already present.
    let mut segments: Vec<(usize, usize, Value)> = Vec::new();
    let mut seg_start = byte_beg;
    while seg_start < byte_end {
        let seg_end = match buf.text.text_props_next_change(seg_start) {
            Some(p) if p < byte_end => p,
            _ => byte_end,
        };
        let existing = buf
            .text
            .text_props_get_property(seg_start, Value::symbol("face"));
        let merged = merge_face_property(existing, new_face, append)?;
        segments.push((seg_start, seg_end, merged));
        seg_start = seg_end;
    }
    let mut any_changed = false;
    for (s, e, merged) in segments {
        if buffers
            .put_buffer_text_property(buf_id, s, e, Value::symbol("face"), merged)
            .unwrap_or(false)
        {
            any_changed = true;
        }
    }
    if any_changed {
        let _ = buffers.record_buffer_text_property_modification(buf_id);
    }
    Ok(Value::NIL)
}

/// (remove-text-properties BEG END PROPS &optional OBJECT)
pub(crate) fn builtin_remove_text_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    verify_property_change_read_only(eval, &args, 3)?;
    run_interval_modification_hooks(eval, &args, 3)?;
    let result = builtin_remove_text_properties_in_buffers(&mut eval.buffers, args.clone())?;
    Ok(result)
}

pub(crate) fn builtin_remove_text_properties_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("remove-text-properties", &args, 3)?;
    expect_max_args("remove-text-properties", &args, 4)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let names = plist_names_for_remove(args[2]);

    if let Some(str_val) = is_string_object(args.get(3)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let Some((char_beg, char_end)) = validate_string_range(s, beg, end, args[0], args[1])?
        else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let mut any_removed = false;
        for name in names {
            if table.remove_property(char_beg, char_end, name) {
                any_removed = true;
            }
        }
        save_string_props_for_value(str_val, table);
        return Ok(if any_removed { Value::T } else { Value::NIL });
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(3))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some((byte_beg, byte_end)) =
        validate_buffer_property_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };
    let mut any_removed = false;
    for name in names {
        if buffers
            .remove_buffer_text_property(buf_id, byte_beg, byte_end, name)
            .unwrap_or(false)
        {
            any_removed = true;
        }
    }
    if any_removed {
        let _ = buffers.record_buffer_text_property_modification(buf_id);
    }
    Ok(if any_removed { Value::T } else { Value::NIL })
}

/// (set-text-properties BEG END PROPS &optional OBJECT)
pub(crate) fn builtin_set_text_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    verify_property_change_read_only(eval, &args, 3)?;
    run_interval_modification_hooks(eval, &args, 3)?;
    let result = builtin_set_text_properties_in_buffers(&mut eval.buffers, args.clone())?;
    Ok(result)
}

pub(crate) fn builtin_set_text_properties_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("set-text-properties", &args, 3)?;
    expect_max_args("set-text-properties", &args, 4)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    // set-text-properties accepts nil for PROPS (= remove all)
    let pairs = if args[2].is_nil() {
        Vec::new()
    } else {
        plist_pairs(&args[2])?
    };

    if let Some(str_val) = is_string_object(args.get(3)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let full_string = beg == 0 && end == s.schars() as i64;
        let had_intervals = string_has_text_property_interval_tree(str_val);
        let Some((char_beg, char_end)) = validate_string_range(s, beg, end, args[0], args[1])?
        else {
            return Ok(Value::NIL);
        };
        if pairs.is_empty() && !had_intervals {
            return Ok(Value::NIL);
        }
        if pairs.is_empty() && full_string {
            clear_string_text_properties_for_value(str_val);
            return Ok(Value::T);
        }
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        table.remove_all_properties(char_beg, char_end);
        for (name, val) in pairs.into_iter().rev() {
            table.put_property(char_beg, char_end, name, val);
        }
        save_string_props_for_value(str_val, table);
        return Ok(Value::T);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(3))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some((byte_beg, byte_end)) =
        validate_buffer_property_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };
    let _ = buffers.clear_buffer_text_properties(buf_id, byte_beg, byte_end);
    for (name, val) in pairs.into_iter().rev() {
        let _ = buffers.put_buffer_text_property(buf_id, byte_beg, byte_end, name, val);
    }
    let _ = buffers.record_buffer_text_property_modification(buf_id);
    Ok(Value::T)
}

/// (remove-list-of-text-properties BEG END LIST &optional OBJECT)
pub(crate) fn builtin_remove_list_of_text_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    verify_property_change_read_only(eval, &args, 3)?;
    run_interval_modification_hooks(eval, &args, 3)?;
    let result =
        builtin_remove_list_of_text_properties_in_buffers(&mut eval.buffers, args.clone())?;
    Ok(result)
}

pub(crate) fn builtin_remove_list_of_text_properties_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("remove-list-of-text-properties", &args, 3)?;
    expect_max_args("remove-list-of-text-properties", &args, 4)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let names = list_names_for_remove(args[2]);

    if let Some(str_val) = is_string_object(args.get(3)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let Some((char_beg, char_end)) = validate_string_range(s, beg, end, args[0], args[1])?
        else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let mut changed = false;
        for name in names {
            if table.remove_property(char_beg, char_end, name) {
                changed = true;
            }
        }
        save_string_props_for_value(str_val, table);
        return Ok(if changed { Value::T } else { Value::NIL });
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(3))?;
    let (byte_beg, byte_end) = {
        let buf = buffers
            .get(buf_id)
            .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
        let Some(range) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
            return Ok(Value::NIL);
        };
        range
    };

    let mut changed = false;
    for name in names {
        let mut cursor = byte_beg;
        while cursor < byte_end {
            let Some(buf) = buffers.get(buf_id) else {
                break;
            };
            if buf.text.text_props_get_property(cursor, name).is_some() {
                changed = true;
                break;
            }
            match buf.text.text_props_next_change(cursor) {
                Some(next) if next > cursor && next < byte_end => cursor = next,
                _ => break,
            }
        }
        let _ = buffers.remove_buffer_text_property(buf_id, byte_beg, byte_end, name);
    }
    if changed {
        let _ = buffers.record_buffer_text_property_modification(buf_id);
    }
    Ok(if changed { Value::T } else { Value::NIL })
}

/// (text-properties-at POS &optional OBJECT)
pub(crate) fn builtin_text_properties_at(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_text_properties_at_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_text_properties_at_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("text-properties-at", &args, 1)?;
    expect_max_args("text-properties-at", &args, 2)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;

    if let Some(str_val) = is_string_object(args.get(1)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let char_pos = validate_string_point_raw(s, pos, args[0])?;
        if char_pos == s.schars() {
            return Ok(Value::NIL);
        }
        if let Some(table) = get_string_text_properties_table_for_value(str_val) {
            return Ok(table.get_properties_plist_value(char_pos));
        }
        return Ok(Value::NIL);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(1))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = validate_buffer_point_raw(buf, pos, args[0])?;
    if byte_pos == buf.text.char_to_byte(buf.text.char_count()) {
        return Ok(Value::NIL);
    }
    Ok(buf.text.text_props_get_properties_plist_value(byte_pos))
}

/// (next-single-property-change POS PROP &optional OBJECT LIMIT)
pub(crate) fn builtin_next_single_property_change(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_next_single_property_change_in_state(&eval.obarray, &eval.buffers, args)
}

pub(crate) fn builtin_next_single_property_change_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("next-single-property-change", &args, 2)?;
    expect_max_args("next-single-property-change", &args, 4)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let prop = expect_property_key(&args[1])?;

    if let Some(str_val) = is_string_object(args.get(2)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let char_pos = validate_string_point_raw(s, pos, args[0])?;
        let (limit_pos, limit_val) = match args.get(3) {
            Some(v) if !v.is_nil() => {
                let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
                (Some(lim_int), Some(lim_int))
            }
            _ => (None, None),
        };
        let current_val = lookup_string_text_property(obarray, buffers, &table, char_pos, prop);
        let str_len = s.schars();
        let mut cursor = char_pos;
        loop {
            match table.next_interval_boundary(cursor) {
                Some(next) => {
                    if let Some(lim) = limit_pos {
                        if next as i64 >= lim {
                            return Ok(match limit_val {
                                Some(lv) => Value::fixnum(lv),
                                None => Value::NIL,
                            });
                        }
                    }
                    if next >= str_len {
                        break;
                    }
                    let new_val = lookup_string_text_property(obarray, buffers, &table, next, prop);
                    let changed = !eq_value(&current_val, &new_val);
                    if changed {
                        return Ok(Value::fixnum(string_char_to_elisp_pos(s, next)));
                    }
                    cursor = next;
                }
                None => break,
            }
        }
        return Ok(match limit_val {
            Some(lv) => Value::fixnum(lv),
            None => Value::NIL,
        });
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(2))?;

    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = validate_buffer_point_raw(buf, pos, args[0])?;
    let (limit_pos, limit_val) = match args.get(3) {
        Some(v) if !v.is_nil() => {
            let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
            (Some(lim_int), Some(lim_int))
        }
        _ => (None, None),
    };

    let current_val = lookup_buffer_text_property(obarray, buffers, buf, byte_pos, prop);
    let buf_end = buf.point_max();
    let mut cursor = byte_pos;

    loop {
        match buf.text.text_props_next_interval_boundary(cursor) {
            Some(next) => {
                if let Some(lim) = limit_pos {
                    if byte_to_elisp_pos(buf, next) >= lim {
                        return Ok(match limit_val {
                            Some(lv) => Value::fixnum(lv),
                            None => Value::NIL,
                        });
                    }
                }
                if next >= buf_end {
                    break;
                }
                let new_val = lookup_buffer_text_property(obarray, buffers, buf, next, prop);
                let changed = !eq_value(&current_val, &new_val);
                if changed {
                    return Ok(Value::fixnum(byte_to_elisp_pos(buf, next)));
                }
                cursor = next;
            }
            None => break,
        }
    }

    Ok(match limit_val {
        Some(lv) => Value::fixnum(lv),
        None => Value::NIL,
    })
}

/// (previous-single-property-change POS PROP &optional OBJECT LIMIT)
pub(crate) fn builtin_previous_single_property_change(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_previous_single_property_change_in_state(&eval.obarray, &eval.buffers, args)
}

pub(crate) fn builtin_previous_single_property_change_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("previous-single-property-change", &args, 2)?;
    expect_max_args("previous-single-property-change", &args, 4)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let prop = expect_property_key(&args[1])?;

    if let Some(str_val) = is_string_object(args.get(2)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let char_pos = validate_string_point_raw(s, pos, args[0])?;
        let (limit_pos, limit_val) = match args.get(3) {
            Some(v) if !v.is_nil() => {
                let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
                (Some(lim_int), Some(lim_int))
            }
            _ => (None, None),
        };
        let ref_char = if char_pos > 0 { char_pos - 1 } else { 0 };
        let current_val = lookup_string_text_property(obarray, buffers, &table, ref_char, prop);
        let mut cursor = char_pos;
        loop {
            match table.previous_interval_boundary(cursor) {
                Some(prev) => {
                    if let Some(lim) = limit_pos {
                        if (prev as i64) <= lim {
                            return Ok(match limit_val {
                                Some(lv) => Value::fixnum(lv),
                                None => Value::NIL,
                            });
                        }
                    }
                    let check = if prev > 0 { prev - 1 } else { 0 };
                    let new_val =
                        lookup_string_text_property(obarray, buffers, &table, check, prop);
                    let changed = !eq_value(&current_val, &new_val);
                    if changed {
                        return Ok(Value::fixnum(string_char_to_elisp_pos(s, prev)));
                    }
                    if prev == 0 {
                        break;
                    }
                    cursor = if prev < cursor { prev } else { prev - 1 };
                }
                None => break,
            }
        }
        return Ok(match limit_val {
            Some(lv) => Value::fixnum(lv),
            None => Value::NIL,
        });
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(2))?;

    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = validate_buffer_point_raw(buf, pos, args[0])?;
    let (limit_pos, limit_val) = match args.get(3) {
        Some(v) if !v.is_nil() => {
            let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
            (Some(lim_int), Some(lim_int))
        }
        _ => (None, None),
    };

    let ref_byte = if byte_pos > 0 { byte_pos - 1 } else { 0 };
    let current_val = lookup_buffer_text_property(obarray, buffers, buf, ref_byte, prop);
    let mut cursor = byte_pos;

    loop {
        match buf.text.text_props_previous_interval_boundary(cursor) {
            Some(prev) => {
                if let Some(lim) = limit_pos {
                    if byte_to_elisp_pos(buf, prev) <= lim {
                        return Ok(match limit_val {
                            Some(lv) => Value::fixnum(lv),
                            None => Value::NIL,
                        });
                    }
                }
                let check = if prev > 0 { prev - 1 } else { 0 };
                let new_val = lookup_buffer_text_property(obarray, buffers, buf, check, prop);
                let changed = !eq_value(&current_val, &new_val);
                if changed {
                    return Ok(Value::fixnum(byte_to_elisp_pos(buf, prev)));
                }
                if prev == 0 {
                    break;
                }
                cursor = if prev < cursor { prev } else { prev - 1 };
            }
            None => break,
        }
    }

    Ok(match limit_val {
        Some(lv) => Value::fixnum(lv),
        None => Value::NIL,
    })
}

/// (next-property-change POS &optional OBJECT LIMIT)
pub(crate) fn builtin_next_property_change(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_next_property_change_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_next_property_change_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("next-property-change", &args, 1)?;
    expect_max_args("next-property-change", &args, 3)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;

    if let Some(str_val) = is_string_object(args.get(1)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let char_pos = validate_string_point_raw(s, pos, args[0])?;
        let limit_arg = args.get(2);
        if limit_arg.is_some_and(|v| v.is_t()) {
            let next = table
                .next_interval_boundary(char_pos)
                .unwrap_or_else(|| s.schars());
            return Ok(Value::fixnum(next as i64));
        }
        let (limit_pos, limit_val) = match limit_arg {
            Some(v) if !v.is_nil() => {
                let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
                (Some(lim_int), Some(Value::fixnum(lim_int)))
            }
            _ => (None, None),
        };
        let str_char_len = s.schars();
        return match table.next_property_change(char_pos) {
            Some(next) => {
                if let Some(lim) = limit_pos {
                    if (next as i64) >= lim {
                        return Ok(limit_val.unwrap_or(Value::NIL));
                    }
                }
                // If the change is at or past the end of the string, treat as no change
                if next >= str_char_len {
                    return Ok(limit_val.unwrap_or(Value::NIL));
                }
                Ok(Value::fixnum(string_char_to_elisp_pos(s, next)))
            }
            None => Ok(limit_val.unwrap_or(Value::NIL)),
        };
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(1))?;
    let limit_arg = args.get(2);

    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = validate_buffer_point_raw(buf, pos, args[0])?;
    if limit_arg.is_some_and(|v| v.is_t()) {
        let next = buf
            .text
            .text_props_next_interval_boundary(byte_pos)
            .unwrap_or_else(|| buf.point_max());
        return Ok(Value::fixnum(byte_to_elisp_pos(buf, next)));
    }
    let (limit_pos, limit_val) = match limit_arg {
        Some(v) if !v.is_nil() => {
            let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
            (Some(lim_int), Some(Value::fixnum(lim_int)))
        }
        _ => (None, None),
    };
    let buf_end = buf.point_max();

    match buf.text.text_props_next_change(byte_pos) {
        Some(next) => {
            if let Some(lim) = limit_pos {
                if byte_to_elisp_pos(buf, next) >= lim {
                    return Ok(limit_val.unwrap_or(Value::NIL));
                }
            }
            // If the change is at or past buffer end, treat as no change
            if next >= buf_end {
                return Ok(limit_val.unwrap_or(Value::NIL));
            }
            Ok(Value::fixnum(byte_to_elisp_pos(buf, next)))
        }
        None => Ok(limit_val.unwrap_or(Value::NIL)),
    }
}

/// (text-property-any BEG END PROP VAL &optional OBJECT)
pub(crate) fn builtin_text_property_any(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_text_property_any_in_state(&eval.obarray, &eval.buffers, args)
}

pub(crate) fn builtin_text_property_any_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("text-property-any", &args, 4)?;
    expect_max_args("text-property-any", &args, 5)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let prop = expect_property_key(&args[2])?;
    let val = &args[3];

    if let Some(str_val) = is_string_object(args.get(4)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let Some((char_beg, char_end)) = validate_string_range(s, beg, end, args[0], args[1])?
        else {
            return Ok(Value::NIL);
        };
        let Some(table) = get_string_text_properties_interval_table_for_value(str_val) else {
            return Ok(if val.is_nil() {
                if char_beg < char_end {
                    Value::fixnum(string_char_to_elisp_pos(s, char_beg))
                } else {
                    Value::NIL
                }
            } else {
                Value::NIL
            });
        };
        let mut cursor = char_beg;
        while cursor < char_end {
            let found = lookup_string_text_property(obarray, buffers, &table, cursor, prop);
            if eq_value(&found, val) {
                return Ok(Value::fixnum(string_char_to_elisp_pos(s, cursor)));
            }
            match table.next_property_change(cursor) {
                Some(next) if next <= char_end => cursor = next,
                _ => break,
            }
        }
        return Ok(Value::NIL);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(4))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some((byte_beg, byte_end)) =
        validate_buffer_property_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };

    if buf.text.text_props_is_empty() {
        return Ok(if val.is_nil() {
            if byte_beg < byte_end {
                Value::fixnum(byte_to_elisp_pos(buf, byte_beg))
            } else {
                Value::NIL
            }
        } else {
            Value::NIL
        });
    }

    let mut cursor = byte_beg;
    while cursor < byte_end {
        let found = lookup_buffer_text_property(obarray, buffers, buf, cursor, prop);
        if eq_value(&found, val) {
            return Ok(Value::fixnum(byte_to_elisp_pos(buf, cursor)));
        }
        match buf.text.text_props_next_change(cursor) {
            Some(next) if next <= byte_end => {
                cursor = next;
            }
            _ => break,
        }
    }
    Ok(Value::NIL)
}

/// (text-property-not-all BEG END PROP VAL &optional OBJECT)
pub(crate) fn builtin_text_property_not_all(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_text_property_not_all_in_state(&eval.obarray, &eval.buffers, args)
}

pub(crate) fn builtin_text_property_not_all_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("text-property-not-all", &args, 4)?;
    expect_max_args("text-property-not-all", &args, 5)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let prop = expect_property_key(&args[2])?;
    let val = &args[3];

    if let Some(str_val) = is_string_object(args.get(4)) {
        let s = str_val
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let Some((char_beg, char_end)) = validate_string_range(s, beg, end, args[0], args[1])?
        else {
            return Ok(Value::NIL);
        };
        let Some(table) = get_string_text_properties_interval_table_for_value(str_val) else {
            return Ok(if val.is_nil() {
                Value::NIL
            } else if char_beg < char_end {
                Value::fixnum(string_char_to_elisp_pos(s, char_beg))
            } else {
                Value::NIL
            });
        };
        let mut cursor = char_beg;
        while cursor < char_end {
            let found = lookup_string_text_property(obarray, buffers, &table, cursor, prop);
            let matches = eq_value(&found, val);
            if !matches {
                return Ok(Value::fixnum(string_char_to_elisp_pos(s, cursor)));
            }
            match table.next_property_change(cursor) {
                Some(next) if next > cursor && next < char_end => cursor = next,
                _ => break,
            }
        }
        return Ok(Value::NIL);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(4))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some((byte_beg, byte_end)) =
        validate_buffer_property_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };

    if buf.text.text_props_is_empty() {
        return Ok(if val.is_nil() {
            Value::NIL
        } else if byte_beg < byte_end {
            Value::fixnum(byte_to_elisp_pos(buf, byte_beg))
        } else {
            Value::NIL
        });
    }

    let mut cursor = byte_beg;

    while cursor < byte_end {
        let found = lookup_buffer_text_property(obarray, buffers, buf, cursor, prop);
        let matches = eq_value(&found, val);
        if !matches {
            return Ok(Value::fixnum(byte_to_elisp_pos(buf, cursor)));
        }

        match buf.text.text_props_next_change(cursor) {
            Some(next) if next > cursor && next < byte_end => cursor = next,
            _ => break,
        }
    }

    Ok(Value::NIL)
}

/// (get-char-property-and-overlay POS PROP &optional OBJECT)
pub(crate) fn builtin_get_char_property_and_overlay(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_get_char_property_and_overlay_with_frames(
        &eval.obarray,
        &eval.buffers,
        Some(&eval.frames),
        args,
    )
}

pub(crate) fn builtin_get_char_property_and_overlay_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    builtin_get_char_property_and_overlay_with_frames(obarray, buffers, None, args)
}

fn builtin_get_char_property_and_overlay_with_frames(
    obarray: &Obarray,
    buffers: &BufferManager,
    frames: Option<&FrameManager>,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("get-char-property-and-overlay", &args, 2)?;
    expect_max_args("get-char-property-and-overlay", &args, 3)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let prop = expect_property_key(&args[1])?;

    // For strings, no overlays — just return (text-prop-value . nil)
    if is_string_object(args.get(2)).is_some() {
        let value = builtin_get_text_property_in_state(obarray, buffers, args)?;
        return Ok(Value::cons(value, Value::NIL));
    }

    let (buf_id, window_id) = resolve_char_property_target_in_state(frames, buffers, args.get(2))?;

    if let Some(buf) = buffers.get(buf_id) {
        let byte_pos = validate_buffer_point_raw(buf, pos, args[0])?;
        if byte_pos == buf.text.char_to_byte(buf.text.char_count()) {
            return Ok(Value::cons(Value::NIL, Value::NIL));
        }
        if let Some((value, ov_val)) =
            buffer_overlay_property_at_byte_pos(obarray, buffers, buf, byte_pos, prop, window_id)
        {
            return Ok(Value::cons(value, ov_val));
        }
    }

    let value = builtin_get_char_property_in_state(obarray, buffers, args)?;
    Ok(Value::cons(value, Value::NIL))
}

/// (get-display-property POS PROP &optional OBJECT PROPERTIES)
pub(crate) fn builtin_get_display_property(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_get_display_property_in_state(&eval.obarray, &eval.buffers, args)
}

pub(crate) fn builtin_get_display_property_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("get-display-property", &args, 2)?;
    expect_max_args("get-display-property", &args, 4)?;
    let prop = expect_property_key(&args[1])?;
    if prop != Value::symbol("display") {
        return Ok(Value::NIL);
    }
    let mut forwarded = vec![args[0], args[1]];
    if let Some(object) = args.get(2) {
        forwarded.push(*object);
    }
    builtin_get_char_property_in_state(obarray, buffers, forwarded)
}

// ===========================================================================
// Overlay builtins
// ===========================================================================

/// (next-overlay-change POS)
pub(crate) fn builtin_next_overlay_change(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_next_overlay_change_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_next_overlay_change_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("next-overlay-change", &args, 1)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let buf_id = current_buffer_id_in_buffers(buffers)?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = elisp_pos_to_byte_clipped_full(buf, pos);
    match buf
        .overlays
        .next_boundary_after_until(byte_pos, buf.point_max_byte())
    {
        Some(next) => Ok(Value::fixnum(byte_to_elisp_pos(buf, next))),
        None => Ok(Value::fixnum(byte_to_elisp_pos(buf, buf.point_max()))),
    }
}

/// (previous-overlay-change POS)
pub(crate) fn builtin_previous_overlay_change(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_previous_overlay_change_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_previous_overlay_change_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("previous-overlay-change", &args, 1)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let buf_id = current_buffer_id_in_buffers(buffers)?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = elisp_pos_to_byte_clipped_full(buf, pos);
    match buf
        .overlays
        .previous_boundary_before_since(byte_pos, buf.point_min_byte())
    {
        Some(prev) => Ok(Value::fixnum(byte_to_elisp_pos(buf, prev))),
        None => Ok(Value::fixnum(byte_to_elisp_pos(buf, buf.point_min()))),
    }
}

/// (make-overlay BEG END &optional BUFFER FRONT-ADVANCE REAR-ADVANCE)
pub(crate) fn builtin_make_overlay(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_make_overlay_in_buffers(&mut eval.buffers, args)
}

pub(crate) fn builtin_make_overlay_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("make-overlay", &args, 2)?;
    expect_max_args("make-overlay", &args, 5)?;
    let buf_id = resolve_buffer_id_in_buffers(buffers, args.get(2))?;
    ensure_marker_points_into_buffer(buffers, &args[0], buf_id)?;
    ensure_marker_points_into_buffer(buffers, &args[1], buf_id)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let front_advance = args.get(3).is_some_and(|v| v.is_truthy());
    let rear_advance = args.get(4).is_some_and(|v| v.is_truthy());

    let buf = buffers
        .get_mut(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let (byte_beg, byte_end) = elisp_range_to_byte_clipped_full(buf, beg, end);
    let overlay = Value::make_overlay(crate::heap_types::OverlayData {
        plist: Value::NIL,
        buffer: Some(buf_id),
        start: byte_beg,
        end: byte_end,
        front_advance,
        rear_advance,
    });
    buf.overlays.insert_overlay(overlay);
    Ok(overlay)
}

/// (delete-overlay OVERLAY)
pub(crate) fn builtin_delete_overlay(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_delete_overlay_in_buffers(&mut eval.buffers, args)
}

pub(crate) fn builtin_delete_overlay_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("delete-overlay", &args, 1)?;
    let overlay = expect_overlay(&args[0])?;
    if let Some(buf_id) = resolve_overlay_buffer_id(overlay) {
        let _ = buffers.delete_buffer_overlay(buf_id, overlay);
    }
    Ok(Value::NIL)
}

/// (overlay-put OVERLAY PROP VAL)
pub(crate) fn builtin_overlay_put(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_overlay_put_in_buffers(&mut eval.buffers, args)
}

pub(crate) fn builtin_overlay_put_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlay-put", &args, 3)?;
    let overlay = expect_overlay(&args[0])?;
    let val = args[2];
    let changed = if let Some(buf_id) = resolve_overlay_buffer_id(overlay) {
        buffers
            .get_mut(buf_id)
            .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?
            .overlays
            .overlay_put(overlay, args[1], val)?
    } else {
        overlay
            .with_overlay_data_mut(|object| {
                let (plist, changed) = plist::plist_put(object.plist, args[1], val)?;
                object.plist = plist;
                Ok::<bool, crate::emacs_core::error::Flow>(changed)
            })
            .unwrap()?
    };
    if let Some(buf_id) = resolve_overlay_buffer_id(overlay) {
        if changed {
            if let Some(buf) = buffers.get_mut(buf_id) {
                buf.increment_overlay_modified_tick();
            }
            let evaporate = args[1].is_symbol_named("evaporate") && val.is_truthy();
            let is_empty = buffers
                .get(buf_id)
                .and_then(|buf| {
                    let start = buf.overlays.overlay_start(overlay)?;
                    let end = buf.overlays.overlay_end(overlay)?;
                    Some(start == end)
                })
                .unwrap_or(false);
            if evaporate && is_empty {
                let _ = buffers.delete_buffer_overlay(buf_id, overlay);
            }
        }
    }
    Ok(val)
}

/// (overlay-get OVERLAY PROP)
pub(crate) fn builtin_overlay_get(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("overlay-get", &args, 2)?;
    let overlay = expect_overlay(&args[0])?;
    Ok(lookup_overlay_property(
        &eval.obarray,
        &eval.buffers,
        overlay,
        args[1],
    ))
}

pub(crate) fn builtin_overlay_get_in_buffers(
    _buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlay-get", &args, 2)?;
    let overlay = expect_overlay(&args[0])?;
    if let Some(data) = overlay.as_overlay_data() {
        if let Some(val) = plist::plist_get(data.plist, &args[1]) {
            return Ok(val);
        }
    }
    Ok(Value::NIL)
}

/// (overlayp OBJ)
pub(crate) fn builtin_overlayp(_eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_overlayp_pure(args)
}

pub(crate) fn builtin_overlayp_pure(args: Vec<Value>) -> EvalResult {
    expect_args("overlayp", &args, 1)?;
    if args[0].is_overlay() {
        return Ok(Value::T);
    }
    Ok(Value::NIL)
}

/// (overlays-at POS &optional SORTED)
pub(crate) fn builtin_overlays_at(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_overlays_at_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_overlays_at_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("overlays-at", &args, 1)?;
    expect_max_args("overlays-at", &args, 2)?;
    let pos = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let buf_id = current_buffer_id_in_buffers(buffers)?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = elisp_pos_to_byte_clipped_full(buf, pos);
    let mut ids = buf.overlays.overlays_at(byte_pos);
    if let Some(sorted) = args.get(1)
        && sorted.is_truthy()
    {
        // GNU `Foverlays_at` (buffer.c:3901): when SORTED is a window value,
        // `sort_overlays` filters via `overlay_matches_window` — overlays
        // whose `window` property is a window distinct from W are dropped.
        if let Some(target_window_id) = sorted.as_window_id() {
            let window_sym = Value::symbol("window");
            ids.retain(|ov| match buf.overlays.overlay_get_named(*ov, window_sym) {
                Some(prop) => prop
                    .as_window_id()
                    .is_none_or(|wid| wid == target_window_id),
                None => true,
            });
        }
        buf.overlays.sort_overlay_ids_by_priority_desc(&mut ids);
    }
    Ok(Value::list(ids))
}

/// (overlays-in BEG END)
pub(crate) fn builtin_overlays_in(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_overlays_in_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_overlays_in_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlays-in", &args, 2)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let buf_id = current_buffer_id_in_buffers(buffers)?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let (byte_beg, byte_end) = elisp_range_to_byte_clipped_full(buf, beg, end);
    let ids = buf
        .overlays
        .overlays_in_region(byte_beg, byte_end, buf.point_max_byte());
    Ok(Value::list(ids))
}

/// (move-overlay OVERLAY BEG END &optional BUFFER)
pub(crate) fn builtin_move_overlay(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_move_overlay_in_buffers(&mut eval.buffers, args)
}

pub(crate) fn builtin_move_overlay_in_buffers(
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("move-overlay", &args, 3)?;
    expect_max_args("move-overlay", &args, 4)?;
    let overlay = expect_overlay(&args[0])?;
    let old_buf_id = resolve_overlay_buffer_id(overlay);

    // Resolve target buffer: use BUFFER arg if given, otherwise same buffer.
    let new_buf_id = if let Some(buf_arg) = args.get(3) {
        if buf_arg.is_truthy() {
            resolve_buffer_id_in_buffers(buffers, Some(buf_arg))?
        } else {
            old_buf_id.unwrap_or_else(|| buffers.current_buffer_id().expect("current buffer"))
        }
    } else {
        old_buf_id.unwrap_or_else(|| buffers.current_buffer_id().expect("current buffer"))
    };

    ensure_marker_points_into_buffer(buffers, &args[1], new_buf_id)?;
    ensure_marker_points_into_buffer(buffers, &args[2], new_buf_id)?;
    let beg = expect_integer_or_marker_in_buffers(buffers, &args[1])?;
    let end = expect_integer_or_marker_in_buffers(buffers, &args[2])?;

    if old_buf_id == Some(new_buf_id) {
        // Same buffer: just move within the buffer.
        let buf = buffers
            .get_mut(new_buf_id)
            .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
        let (byte_beg, byte_end) = elisp_range_to_byte_clipped_full(buf, beg, end);
        buf.overlays.move_overlay(overlay, byte_beg, byte_end);
        buf.increment_overlay_modified_tick();
        Ok(args[0])
    } else {
        if let Some(old_buf_id) = old_buf_id
            && let Some(buf) = buffers.get_mut(old_buf_id)
        {
            if buf.overlays.detach_overlay(overlay) {
                buf.increment_overlay_modified_tick();
            }
        }

        let new_buf = buffers
            .get_mut(new_buf_id)
            .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
        let (byte_beg, byte_end) = elisp_range_to_byte_clipped_full(new_buf, beg, end);
        let _ = overlay.with_overlay_data_mut(|object| {
            object.buffer = Some(new_buf_id);
            object.start = byte_beg;
            object.end = byte_end;
        });
        new_buf.overlays.insert_overlay(overlay);
        new_buf.increment_overlay_modified_tick();
        if byte_beg == byte_end
            && new_buf
                .overlays
                .overlay_get_named(overlay, Value::symbol("evaporate"))
                .is_some_and(|value| value.is_truthy())
        {
            if new_buf.overlays.delete_overlay(overlay) {
                new_buf.increment_overlay_modified_tick();
            }
        }
        Ok(args[0])
    }
}

/// (overlay-start OVERLAY)
pub(crate) fn builtin_overlay_start(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_overlay_start_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_overlay_start_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlay-start", &args, 1)?;
    let overlay = expect_overlay(&args[0])?;
    let Some(buf_id) = resolve_overlay_buffer_id(overlay) else {
        return Ok(Value::NIL);
    };
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    match buf.overlays.overlay_start(overlay) {
        Some(byte_pos) => Ok(Value::fixnum(byte_to_elisp_pos(buf, byte_pos))),
        None => Ok(Value::NIL),
    }
}

/// (overlay-end OVERLAY)
pub(crate) fn builtin_overlay_end(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_overlay_end_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_overlay_end_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlay-end", &args, 1)?;
    let overlay = expect_overlay(&args[0])?;
    let Some(buf_id) = resolve_overlay_buffer_id(overlay) else {
        return Ok(Value::NIL);
    };
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    match buf.overlays.overlay_end(overlay) {
        Some(byte_pos) => Ok(Value::fixnum(byte_to_elisp_pos(buf, byte_pos))),
        None => Ok(Value::NIL),
    }
}

/// (overlay-buffer OVERLAY)
pub(crate) fn builtin_overlay_buffer(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_overlay_buffer_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_overlay_buffer_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlay-buffer", &args, 1)?;
    let overlay = expect_overlay(&args[0])?;
    if let Some(buf_id) = resolve_overlay_buffer_id(overlay)
        && buffers.get(buf_id).is_some()
    {
        return Ok(Value::make_buffer(buf_id));
    }
    Ok(Value::NIL)
}

/// (overlay-properties OVERLAY)
pub(crate) fn builtin_overlay_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_overlay_properties_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_overlay_properties_in_buffers(
    _buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlay-properties", &args, 1)?;
    let overlay = expect_overlay(&args[0])?;
    builtin_copy_sequence(vec![
        overlay.as_overlay_data().map_or(Value::NIL, |d| d.plist),
    ])
}

/// (remove-overlays &optional BEG END NAME VAL)
pub(crate) fn builtin_remove_overlays(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_max_args("remove-overlays", &args, 4)?;
    let buf_id = eval
        .buffers
        .current_buffer()
        .map(|b| b.id)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

    let (start_pos, end_pos) = {
        let buf = eval
            .buffers
            .get(buf_id)
            .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
        let start = if args.is_empty() || args[0].is_nil() {
            buf.point_min()
        } else {
            elisp_pos_to_byte_clipped_full(buf, expect_int_eval(eval, &args[0])?)
        };
        let end = if args.len() < 2 || args[1].is_nil() {
            buf.point_max()
        } else {
            elisp_pos_to_byte_clipped_full(buf, expect_int_eval(eval, &args[1])?)
        };
        (start, end)
    };

    let buf = eval
        .buffers
        .get_mut(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let filter_name = if args.len() >= 3 && !args[2].is_nil() {
        Some(expect_property_key(&args[2])?)
    } else {
        None
    };

    let filter_val = if args.len() >= 4 && !args[3].is_nil() {
        Some(args[3])
    } else {
        None
    };

    // Collect overlay ids in range.
    let ids = buf
        .overlays
        .overlays_in_region(start_pos, end_pos, buf.point_max_byte());

    // Filter and delete.
    for overlay in ids {
        let should_delete = match (&filter_name, &filter_val) {
            (Some(name), Some(val)) => buf
                .overlays
                .overlay_get(overlay, name)
                .is_some_and(|v| equal_value(&v, val, 0)),
            (Some(name), None) => buf.overlays.overlay_get(overlay, name).is_some(),
            _ => true,
        };
        if should_delete {
            buf.overlays.delete_overlay(overlay);
        }
    }

    Ok(Value::NIL)
}

// ===========================================================================
// Tests
// ===========================================================================
#[cfg(test)]
#[path = "textprop_test.rs"]
mod tests;
