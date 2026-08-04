//! Text property and overlay builtins for the Elisp interpreter.
//!
//! Bridges the buffer's `TextPropertyTable` and `OverlayList` to Elisp
//! functions like `put-text-property`, `make-overlay`, etc.

use crate::emacs_core::error::{expect_args, expect_min_args, expect_max_args};
use super::builtins::builtin_copy_sequence;
use super::error::{EvalResult, Flow, signal};
use super::intern::{NIL_SYM_ID, T_SYM_ID};
use crate::emacs_core::error::LispCondition;
// storage imports removed — now using emacs_char directly
use super::plist;
use super::symbol::Obarray;
use super::value::*;
use crate::buffer::text_props::{PropertyPlistApplication, TextPropertyTable};
use crate::buffer::{
    Buffer, BufferId, BufferManager, CharLen, CharPos0, CharRange, EmacsBytePos, EmacsByteRange,
    LispCharPos1,
};
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

#[inline]
fn buffer_char_to_emacs_byte_pos(buf: &Buffer, char_pos: CharPos0) -> EmacsBytePos {
    buf.char_pos_to_emacs_byte_pos_clamped(char_pos)
}

#[inline]
fn buffer_char_to_byte_pos(buf: &Buffer, char_pos: CharPos0) -> usize {
    buffer_char_to_emacs_byte_pos(buf, char_pos).get()
}

#[inline]
fn buffer_end_emacs_byte_pos(buf: &Buffer) -> EmacsBytePos {
    buf.total_emacs_byte_end_pos()
}

#[inline]
fn string_char_pos(pos: usize) -> CharPos0 {
    CharPos0::new(pos)
}

#[inline]
fn string_char_len(len: usize) -> CharLen {
    CharLen::new(len)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expect_int(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => super::marker::marker_position_as_int(value),
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integerp"), *value],
        )),
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expect_int_eval(eval: &super::eval::Context, value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => {
            super::marker::marker_position_as_int_eval(eval, value)
        }
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integerp"), *value],
        )),
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expect_integer_or_marker(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => super::marker::marker_position_as_int(value),
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
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

/// Interned id of `char-property-alias-alist`, resolved once. The
/// get-char-property miss path consults this var on every miss during
/// redisplay/fontification; caching the id avoids re-hashing the 25-char
/// name on each call (GNU compares against the `Qchar_property_alias_alist`
/// symbol by identity, never by name).
fn char_property_alias_alist_sym_id() -> SymId {
    static ID: std::sync::OnceLock<SymId> = std::sync::OnceLock::new();
    *ID.get_or_init(|| super::intern::intern("char-property-alias-alist"))
}

/// Interned id of `default-text-properties`, resolved once. See
/// [`char_property_alias_alist_sym_id`].
fn default_text_properties_sym_id() -> SymId {
    static ID: std::sync::OnceLock<SymId> = std::sync::OnceLock::new();
    *ID.get_or_init(|| super::intern::intern("default-text-properties"))
}

fn current_textprop_variable_value(
    obarray: &Obarray,
    buffers: &BufferManager,
    sym_id: SymId,
) -> Option<Value> {
    // Text-property control vars (char-property-alias-alist, default-text-
    // properties, inhibit-*, ...) are almost always plain globals. A global
    // (non-Localized) symbol can never be in a buffer's local_var_alist, so
    // skip the per-buffer scan for it -- this runs during redisplay and was
    // ~3% of the layout profile. See `Obarray::is_localized`. The caller
    // passes a cached SymId so the hot miss path never re-interns the name.
    let localized = obarray.is_localized(sym_id);
    if let Some(buf) = buffers.current_buffer()
        && let Some(binding) = buf.get_buffer_local_binding_by_sym_id_gated(sym_id, localized)
    {
        return binding.as_value();
    }
    obarray.symbol_value_id_copied(sym_id)
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
        current_textprop_variable_value(obarray, buffers, char_property_alias_alist_sym_id())
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
            current_textprop_variable_value(obarray, buffers, default_text_properties_sym_id())
        && defaults.is_cons()
        && let Some(value) = plist_get_value(defaults, prop)
    {
        return value;
    }

    fallback
}

/// Resolve a char/text property from an interval's plist (in slice form),
/// resolving through a `category` symbol just like GNU `textget`
/// (`lookup_char_property` with `textprop = true`).  Used when collecting
/// `modification-hooks` from text-property intervals so a `category' interval
/// contributes the category symbol's `modification-hooks' property.
pub(crate) fn lookup_text_property_from_plist_slice(
    obarray: &Obarray,
    buffers: &BufferManager,
    plist: &[(Value, Value)],
    prop: Value,
) -> Value {
    lookup_char_property_from_direct(
        obarray,
        buffers,
        |name| plist_slice_get_value(plist, name),
        prop,
        true,
    )
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
        |name| table.get_property_at_char_pos(string_char_pos(char_pos), name),
        prop,
        true,
    )
}

fn lookup_buffer_text_property_at_char_pos(
    obarray: &Obarray,
    buffers: &BufferManager,
    buf: &crate::buffer::buffer::Buffer,
    char_pos: CharPos0,
    prop: Value,
) -> Value {
    lookup_char_property_from_direct(
        obarray,
        buffers,
        |name| buf.text_props_get_property_at_char_pos(char_pos, name),
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
    lookup_buffer_text_property_at_emacs_byte_pos(
        obarray,
        buffers,
        buf,
        EmacsBytePos::new(byte_pos),
        prop,
    )
}

pub(crate) fn lookup_buffer_text_property_at_emacs_byte_pos(
    obarray: &Obarray,
    buffers: &BufferManager,
    buf: &crate::buffer::buffer::Buffer,
    byte_pos: EmacsBytePos,
    prop: Value,
) -> Value {
    lookup_buffer_text_property_at_char_pos(
        obarray,
        buffers,
        buf,
        buf.emacs_byte_pos_to_char_pos_clamped(byte_pos),
        prop,
    )
}

pub(crate) fn lookup_overlay_property(
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
fn elisp_pos_to_byte(buf: &crate::buffer::buffer::Buffer, pos: LispCharPos1) -> EmacsBytePos {
    debug_assert!(pos.as_i64() >= 1);
    buffer_char_to_emacs_byte_pos(buf, pos.to_char_pos())
}

fn validated_lisp_char_pos(pos: i64) -> LispCharPos1 {
    debug_assert!(pos >= 1);
    LispCharPos1::from_one_based_usize(usize::try_from(pos).expect("Lisp position fits usize"))
}

fn elisp_pos_to_byte_clipped_full(
    buf: &crate::buffer::buffer::Buffer,
    pos: LispCharPos1,
) -> EmacsBytePos {
    let max = buf.z_lisp_char_pos().as_i64();
    let clipped = validated_lisp_char_pos(pos.as_i64().clamp(1, max));
    elisp_pos_to_byte(buf, clipped)
}

fn elisp_range_to_byte_clipped_full(
    buf: &crate::buffer::buffer::Buffer,
    mut beg: i64,
    mut end: i64,
) -> EmacsByteRange {
    if beg > end {
        std::mem::swap(&mut beg, &mut end);
    }
    let max = buf.z_lisp_char_pos().as_i64();
    let clipped_beg = beg.clamp(1, max);
    let clipped_end = end.clamp(clipped_beg, max);
    EmacsByteRange::new(
        elisp_pos_to_byte(buf, validated_lisp_char_pos(clipped_beg)),
        elisp_pos_to_byte(buf, validated_lisp_char_pos(clipped_end)),
    )
}

fn args_out_of_range_point(pos: i64) -> Flow {
    signal(LispCondition::ArgsOutOfRange, vec![Value::fixnum(pos)])
}

fn args_out_of_range_point_pair(pos0: Value) -> Flow {
    signal(LispCondition::ArgsOutOfRange, vec![pos0, pos0])
}

fn args_out_of_range_range(begin0: Value, end0: Value) -> Flow {
    signal(LispCondition::ArgsOutOfRange, vec![begin0, end0])
}

pub(crate) fn validate_string_point_raw(
    s: &crate::heap_types::LispString,
    pos: i64,
    pos0: Value,
) -> Result<usize, Flow> {
    validate_string_char_pos_raw(s, pos, pos0).map(CharPos0::get)
}

pub(crate) fn validate_string_char_pos_raw(
    s: &crate::heap_types::LispString,
    pos: i64,
    pos0: Value,
) -> Result<CharPos0, Flow> {
    let len = s.schars() as i64;
    if !(0 <= pos && pos <= len) {
        return Err(args_out_of_range_point_pair(pos0));
    }
    Ok(string_char_pos(pos as usize))
}

fn validate_string_range(
    s: &crate::heap_types::LispString,
    beg: i64,
    end: i64,
    beg0: Value,
    end0: Value,
) -> Result<Option<CharRange>, Flow> {
    if beg == end {
        return Ok(None);
    }
    let (start, finish) = if beg > end { (end, beg) } else { (beg, end) };
    let len = s.schars() as i64;
    if !(0 <= start && start <= finish && finish <= len) {
        return Err(args_out_of_range_range(beg0, end0));
    }
    Ok(Some(CharRange::new(
        string_char_pos(start as usize),
        string_char_pos(finish as usize),
    )))
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
    _pos0: Value,
) -> Result<usize, Flow> {
    validate_buffer_point_emacs_byte_pos_raw(buf, pos, _pos0).map(EmacsBytePos::get)
}

pub(crate) fn validate_buffer_point_emacs_byte_pos_raw(
    buf: &crate::buffer::buffer::Buffer,
    pos: i64,
    _pos0: Value,
) -> Result<EmacsBytePos, Flow> {
    let point_min = buf.point_min_lisp_char_pos().as_i64();
    let point_max = buf.point_max_lisp_char_pos().as_i64();
    if !(point_min <= pos && pos <= point_max) {
        return Err(args_out_of_range_point(pos));
    }
    Ok(elisp_pos_to_byte(buf, validated_lisp_char_pos(pos)))
}

fn validate_buffer_property_point_emacs_byte_pos_raw(
    buf: &crate::buffer::buffer::Buffer,
    pos: i64,
    pos0: Value,
) -> Result<EmacsBytePos, Flow> {
    let point_min = buf.point_min_lisp_char_pos().as_i64();
    let point_max = buf.point_max_lisp_char_pos().as_i64();
    if !(point_min <= pos && pos <= point_max) {
        return Err(args_out_of_range_point_pair(pos0));
    }
    Ok(elisp_pos_to_byte(buf, validated_lisp_char_pos(pos)))
}

fn validate_buffer_property_range(
    buf: &crate::buffer::buffer::Buffer,
    beg: i64,
    end: i64,
    beg0: Value,
    end0: Value,
) -> Result<Option<EmacsByteRange>, Flow> {
    validate_buffer_property_emacs_byte_range(buf, beg, end, beg0, end0)
}

fn validate_buffer_property_emacs_byte_range(
    buf: &crate::buffer::buffer::Buffer,
    beg: i64,
    end: i64,
    beg0: Value,
    end0: Value,
) -> Result<Option<EmacsByteRange>, Flow> {
    if beg == end {
        return Ok(None);
    }
    let (start, finish) = if beg > end { (end, beg) } else { (beg, end) };
    let point_min = buf.point_min_lisp_char_pos().as_i64();
    let point_max = buf.point_max_lisp_char_pos().as_i64();
    if !(point_min <= start && start <= finish && finish <= point_max) {
        return Err(args_out_of_range_range(beg0, end0));
    }
    Ok(Some(EmacsByteRange::new(
        elisp_pos_to_byte(buf, validated_lisp_char_pos(start)),
        elisp_pos_to_byte(buf, validated_lisp_char_pos(finish)),
    )))
}

/// Convert a 0-based byte position to a 1-based Elisp char position.
pub(crate) fn byte_to_elisp_pos(
    buf: &crate::buffer::buffer::Buffer,
    byte_pos: EmacsBytePos,
) -> i64 {
    buf.emacs_byte_pos_to_lisp_char_pos(byte_pos).as_i64()
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
            LispCondition::WrongTypeArgument,
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
            LispCondition::WrongTypeArgument,
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
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("buffer-or-string-p"), *v],
                ));
            };
            let wid = WindowId(v.as_window_id().expect("window value has an id"));
            let window = frames.lookup_window(wid).ok_or_else(|| {
                signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("window-live-p"), *v],
                )
            })?;
            let buffer_id = window.buffer_id().ok_or_else(|| {
                signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("window-live-p"), *v],
                )
            })?;
            Ok((buffer_id, Some(wid)))
        }
        Some(other) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("buffer-or-string-p"), *other],
        )),
    }
}

/// Resolve a char-property OBJECT argument (None/nil/buffer/window) to a
/// `BufferId`, matching GNU `get_char_property_and_overlay`'s window handling:
/// a WINDOW object resolves to its buffer (requires `frames` to be `Some`).
/// The window-specific overlay matching is dropped here; callers that need the
/// `WindowId` for overlay matching use `resolve_char_property_target_in_state`
/// directly.
pub(crate) fn resolve_char_property_buffer_id_with_frames(
    frames: Option<&FrameManager>,
    buffers: &BufferManager,
    object: Option<&Value>,
) -> Result<BufferId, Flow> {
    resolve_char_property_target_in_state(frames, buffers, object).map(|(id, _wid)| id)
}

fn current_buffer_id_in_buffers(buffers: &BufferManager) -> Result<BufferId, Flow> {
    buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))
}

fn expect_overlay(value: &Value) -> Result<Value, Flow> {
    if !value.is_overlay() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
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

pub(crate) fn string_char_to_elisp_pos(
    _s: &crate::heap_types::LispString,
    char_pos: CharPos0,
) -> i64 {
    char_pos.get() as i64
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
    verify_text_read_only_emacs_byte_range_in_state(
        obarray,
        buffers,
        buf_id,
        EmacsByteRange::new(EmacsBytePos::new(byte_start), EmacsBytePos::new(byte_end)),
    )
}

pub(crate) fn verify_text_read_only_emacs_byte_range_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    buf_id: BufferId,
    byte_range: EmacsByteRange,
) -> Result<(), Flow> {
    if byte_range.is_empty() {
        return Ok(());
    }
    let Some(buf) = buffers.get(buf_id) else {
        return Ok(());
    };
    let iro = crate::emacs_core::intern::intern("inhibit-read-only");
    let inhibit = buf
        .get_buffer_local_by_sym_id_gated(iro, obarray.is_localized(iro))
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
    buf.text_props_try_for_each_interval_in_emacs_byte_range(byte_range, |_range, plist| {
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
        Err(signal(LispCondition::TextReadOnly, args))
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

/// GNU `TMEM(sym, set)` (intervals.h): if SET is a list, whether SYM is `memq`
/// it; otherwise whether SET is non-nil (`t` means "all properties").
fn text_prop_sticky_member(sym: Value, set: Value) -> bool {
    if set.is_cons() {
        value_in_list(sym, set)
    } else {
        !set.is_nil()
    }
}

/// `inhibit-read-only` silences this read-only value: never (nil), or when it
/// is a list containing the value. (The "non-nil non-list" blanket case is
/// handled by the caller before reaching here.)
fn read_only_silenced(read_only: Value, inhibit: Value) -> bool {
    inhibit.is_cons() && value_in_list(read_only, inhibit)
}

fn text_read_only_flow(read_only: Value) -> Flow {
    let args = if read_only.is_string() {
        vec![read_only]
    } else {
        vec![]
    };
    signal(LispCondition::TextReadOnly, args)
}

/// GNU `verify_interval_modification` (textprop.c:2184), the `start == end`
/// insertion case: signal `text-read-only` when inserting at `byte_pos` is
/// forbidden by the `read-only` property of the adjacent characters, honoring
/// stickiness. The char *after* blocks only when `read-only` is front-sticky;
/// the char *before* blocks unless `read-only` is rear-nonsticky (so a plain
/// `(put-text-property ... 'read-only t)` — rear-sticky by default — forbids
/// insertion right after it, while inserting before it stays allowed). This is
/// what lets minibuffer input through: the prompt is `rear-nonsticky t`.
pub(crate) fn verify_text_read_only_for_insert_in_state(
    obarray: &Obarray,
    buffers: &BufferManager,
    buf_id: BufferId,
    byte_pos: EmacsBytePos,
) -> Result<(), Flow> {
    let Some(buf) = buffers.get(buf_id) else {
        return Ok(());
    };
    let iro = crate::emacs_core::intern::intern("inhibit-read-only");
    let inhibit = buf
        .get_buffer_local_by_sym_id_gated(iro, obarray.is_localized(iro))
        .unwrap_or_else(|| {
            obarray
                .symbol_value("inhibit-read-only")
                .copied()
                .unwrap_or(Value::NIL)
        });
    // inhibit-read-only non-nil and not a list: every modification is allowed.
    if !inhibit.is_nil() && !inhibit.is_cons() {
        return Ok(());
    }
    let read_only_sym = Value::symbol("read-only");
    let accessible = buf.accessible_emacs_byte_range();
    let begv = accessible.start().get();
    let zv = accessible.end().get();
    let pos = byte_pos.get();

    // Character after the insertion point: blocks only if `read-only` is
    // front-sticky there.
    if pos < zv {
        let after = lookup_buffer_text_property_at_emacs_byte_pos(
            obarray,
            buffers,
            buf,
            byte_pos,
            read_only_sym,
        );
        if !after.is_nil() && !read_only_silenced(after, inhibit) {
            let front_sticky = lookup_buffer_text_property_at_emacs_byte_pos(
                obarray,
                buffers,
                buf,
                byte_pos,
                Value::symbol("front-sticky"),
            );
            if text_prop_sticky_member(read_only_sym, front_sticky) {
                return Err(text_read_only_flow(after));
            }
        }
    }

    // Character before the insertion point: blocks unless `read-only` is
    // rear-nonsticky there (rear-sticky is the default).
    if pos > begv {
        let before_byte = EmacsBytePos::new(pos - 1);
        let before = lookup_buffer_text_property_at_emacs_byte_pos(
            obarray,
            buffers,
            buf,
            before_byte,
            read_only_sym,
        );
        if !before.is_nil() && !read_only_silenced(before, inhibit) {
            let rear_nonsticky = lookup_buffer_text_property_at_emacs_byte_pos(
                obarray,
                buffers,
                buf,
                before_byte,
                Value::symbol("rear-nonsticky"),
            );
            if !text_prop_sticky_member(read_only_sym, rear_nonsticky) {
                return Err(text_read_only_flow(before));
            }
        }
    }

    Ok(())
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
    let byte_range = {
        let buf = eval
            .buffers
            .get(buf_id)
            .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
        let Some(range) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
            return Ok(());
        };
        range
    };
    verify_text_read_only_emacs_byte_range_in_state(
        &eval.obarray,
        &eval.buffers,
        buf_id,
        byte_range,
    )
}

fn buffer_property_range_for_args(
    eval: &super::eval::Context,
    args: &[Value],
    object_arg_idx: usize,
) -> Result<Option<(BufferId, EmacsByteRange)>, Flow> {
    if is_string_object(args.get(object_arg_idx)).is_some() {
        return Ok(None);
    }
    if args.len() < 2 {
        return Ok(None);
    }
    let beg = expect_integer_or_marker_in_buffers(&eval.buffers, &args[0])?;
    let end = expect_integer_or_marker_in_buffers(&eval.buffers, &args[1])?;
    let buf_id =
        resolve_text_property_buffer_id_in_buffers(&eval.buffers, args.get(object_arg_idx))?;
    let Some(buf) = eval.buffers.get(buf_id) else {
        return Err(signal(
            "error",
            vec![Value::string("Buffer does not exist")],
        ));
    };
    validate_buffer_property_range(buf, beg, end, args[0], args[1])
        .map(|range| range.map(|byte_range| (buf_id, byte_range)))
}

fn begin_buffer_text_property_change(
    eval: &mut super::eval::Context,
    buf_id: BufferId,
    byte_range: EmacsByteRange,
) -> Result<(Option<BufferId>, crate::buffer::TextChange), Flow> {
    let saved_current = eval.buffers.current_buffer_id();
    if saved_current != Some(buf_id) {
        eval.set_current_buffer_unrecorded(buf_id)?;
    }
    let change = super::editfns::text_change_for_unchanged_extent_in_manager(
        &eval.buffers,
        buf_id,
        byte_range,
    )?;
    super::editfns::signal_before_text_change(eval, change)?;
    Ok((saved_current, change))
}

fn finish_buffer_text_property_change(
    eval: &mut super::eval::Context,
    saved_current: Option<BufferId>,
    change: crate::buffer::TextChange,
) -> Result<(), Flow> {
    let result = super::editfns::signal_after_text_change(eval, change);
    if let Some(saved) = saved_current {
        eval.restore_current_buffer_if_live(saved);
    }
    result
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
    // The collected hook chains live only in this Rust Vec, and each hook
    // can unlink its own chain from the interval plist (the one-shot-hook
    // idiom) and trigger GC — freeing the conses the walk still reads. Keep
    // every chain alive under ONE root by threading the list heads onto a
    // heap list; the moving cursor stays inside a rooted chain. GNU's C
    // locals survive this via conservative stack scanning (textprop.c
    // verify_interval_modification), which the precise GC does not scan.
    let mut hook_holder = Value::NIL;
    for hook_list in hook_lists.iter().rev() {
        hook_holder = Value::cons(*hook_list, hook_holder);
    }
    let root_scope = eval.save_specpdl_roots();
    eval.push_specpdl_root(hook_holder);
    // Rooting the WALK CURSOR (updated per step) additionally keeps the
    // remaining chain alive even if a hook setcdr's the chain mid-walk —
    // marking is transitive from the cursor, exactly the survival GNU gets
    // from its conservatively-scanned tail local.
    let cursor_slot = eval.push_specpdl_root_slot(Value::NIL);
    let result = (|| -> Result<(), Flow> {
        for hook_list in hook_lists {
            let mut cursor = hook_list;
            while cursor.is_cons() {
                eval.set_specpdl_root_slot(&cursor_slot, cursor);
                let fn_v = cursor.cons_car();
                eval.apply(fn_v, vec![start_v, end_v])?;
                cursor = cursor.cons_cdr();
            }
        }
        Ok(())
    })();
    eval.restore_specpdl_roots(root_scope);
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
    byte_start: EmacsBytePos,
    byte_end: EmacsBytePos,
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
        let obarray = &eval.obarray;
        let buffers = &eval.buffers;
        let Some(buf) = buffers.get(buf_id) else {
            return Ok(());
        };
        let byte_range = EmacsByteRange::ordered(byte_start, byte_end);
        let lisp_start = buf
            .emacs_byte_pos_to_lisp_char_pos(byte_range.start())
            .as_i64();
        let lisp_end = buf
            .emacs_byte_pos_to_lisp_char_pos(byte_range.end())
            .as_i64();
        let mod_sym = Value::symbol("modification-hooks");
        let mut prev: Option<Value> = None;
        let mut hooks = Vec::new();
        let _ = buf.text_props_try_for_each_interval_in_emacs_byte_range(
            byte_range,
            |_range, plist| {
                // GNU `verify_interval_modification` reads `modification-hooks'
                // via `textget`, which resolves through a `category' symbol.
                let mh = lookup_text_property_from_plist_slice(obarray, buffers, plist, mod_sym);
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
            },
        );
        (lisp_start, lisp_end, hooks)
    };

    call_text_property_hook_lists(eval, hook_lists, lisp_start, lisp_end)
}

fn record_interval_insert_hooks(
    eval: &mut super::eval::Context,
    buf_id: BufferId,
    byte_pos: EmacsBytePos,
) {
    let Some(buf) = eval.buffers.get(buf_id) else {
        return;
    };
    // With no text properties anywhere, no `insert-in-front-hooks' /
    // `insert-behind-hooks' can exist, so skip the per-insert property lookups
    // -- each of which does a byte->char conversion.  This is the hot path for
    // inserts into property-free buffers (byte-compilation output, batch work).
    // The hook fields were already reset to nil by the caller, so leaving them
    // is correct.
    if buf.text_props_is_empty() {
        return;
    }
    let behind_sym = Value::symbol("insert-behind-hooks");
    let front_sym = Value::symbol("insert-in-front-hooks");
    let accessible = buf.accessible_emacs_byte_region();

    if byte_pos > accessible.start()
        && let Some(prev_len) = buf.char_before_emacs_byte_len(byte_pos)
    {
        let prev_byte = byte_pos.saturating_sub_len(prev_len);
        if let Some(hooks) = buf.text_props_get_property_at_emacs_byte_pos(prev_byte, behind_sym)
            && !hooks.is_nil()
        {
            eval.interval_insert_behind_hooks = hooks;
        }
    }

    if byte_pos < accessible.end()
        && let Some(hooks) = buf.text_props_get_property_at_emacs_byte_pos(byte_pos, front_sym)
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
    expect_min_args("put-text-property", &args, 4)?;
    expect_max_args("put-text-property", &args, 5)?;
    verify_property_change_read_only(eval, &args, 4)?;
    let change =
        buffer_property_range_for_args(eval, &args, 4)?.and_then(|(buf_id, byte_range)| {
            let buf = eval.buffers.get(buf_id)?;
            let properties = [(args[2], args[3])];
            (!buf.text_props_range_has_all_properties_in_emacs_byte_range(byte_range, &properties))
                .then_some((buf_id, byte_range))
        });
    let before = if let Some((buf_id, byte_range)) = change {
        Some(begin_buffer_text_property_change(eval, buf_id, byte_range)?)
    } else {
        None
    };
    let result = builtin_put_text_property_in_buffers(&mut eval.buffers, args.clone())?;
    if let Some((saved_current, change)) = before {
        finish_buffer_text_property_change(eval, saved_current, change)?;
    }
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
        let Some(char_range) = validate_string_range(s, beg, end, args[0], args[1])? else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        table.put_property_for_object_char_len(char_range, string_char_len(s.schars()), prop, val);
        save_string_props_for_value(str_val, table);
        return Ok(Value::NIL);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(4))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some(byte_range) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
        return Ok(Value::NIL);
    };
    if buffers
        .put_buffer_text_property_in_emacs_byte_range(buf_id, byte_range, prop, val)
        .unwrap_or(false)
    {
        let _ = buffers.record_buffer_text_property_modification(buf_id, byte_range);
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
        let char_pos = validate_string_char_pos_raw(s, pos, args[0])?;
        if char_pos.get() == s.schars() {
            return Ok(Value::NIL);
        }
        if let Some(table) = get_string_text_properties_table_for_value(str_val) {
            return Ok(lookup_string_text_property(
                obarray,
                buffers,
                &table,
                char_pos.get(),
                prop,
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

    let byte_pos = validate_buffer_property_point_emacs_byte_pos_raw(buf, pos, args[0])?;
    if byte_pos == buffer_end_emacs_byte_pos(buf) {
        return Ok(Value::NIL);
    }
    Ok(lookup_buffer_text_property(
        obarray,
        buffers,
        buf,
        byte_pos.get(),
        prop,
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
    let mut overlays = buf
        .overlays
        .overlays_at_emacs_byte_pos(EmacsBytePos::new(byte_pos));
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
        .highest_priority_overlay_for_inserted_emacs_byte_pos(EmacsBytePos::new(byte_pos), &prop)?;
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

pub(crate) fn builtin_get_char_property_with_frames(
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
    let byte_pos = validate_buffer_point_emacs_byte_pos_raw(buf, pos, args[0])?;
    if byte_pos == buffer_end_emacs_byte_pos(buf) {
        return Ok(Value::NIL);
    }

    if let Some((value, _overlay_id)) =
        buffer_overlay_property_at_byte_pos(obarray, buffers, buf, byte_pos.get(), prop, window_id)
    {
        return Ok(value);
    }

    Ok(lookup_buffer_text_property(
        obarray,
        buffers,
        buf,
        byte_pos.get(),
        prop,
    ))
}

/// (add-text-properties BEG END PROPS &optional OBJECT)
pub(crate) fn builtin_add_text_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("add-text-properties", &args, 3)?;
    expect_max_args("add-text-properties", &args, 4)?;
    verify_property_change_read_only(eval, &args, 3)?;
    let pairs_for_probe = plist_pairs(&args[2])?;
    let change =
        buffer_property_range_for_args(eval, &args, 3)?.and_then(|(buf_id, byte_range)| {
            let buf = eval.buffers.get(buf_id)?;
            (!buf.text_props_range_has_all_properties_in_emacs_byte_range(
                byte_range,
                &pairs_for_probe,
            ))
            .then_some((buf_id, byte_range))
        });
    let before = if let Some((buf_id, byte_range)) = change {
        Some(begin_buffer_text_property_change(eval, buf_id, byte_range)?)
    } else {
        None
    };
    let result = builtin_add_text_properties_in_buffers(&mut eval.buffers, args.clone())?;
    if let Some((saved_current, change)) = before {
        finish_buffer_text_property_change(eval, saved_current, change)?;
    }
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
        let Some(char_range) = validate_string_range(s, beg, end, args[0], args[1])? else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let any_changed = table.apply_property_plist_for_object_char_len(
            char_range,
            string_char_len(s.schars()),
            &pairs,
            PropertyPlistApplication::AddProperties,
        );
        save_string_props_for_value(str_val, table);
        return Ok(if any_changed { Value::T } else { Value::NIL });
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(3))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some(byte_range) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
        return Ok(Value::NIL);
    };
    let mut any_changed = false;
    for (name, val) in pairs {
        if buffers
            .put_buffer_text_property_in_emacs_byte_range(buf_id, byte_range, name, val)
            .unwrap_or(false)
        {
            any_changed = true;
        }
    }
    if any_changed {
        let _ = buffers.record_buffer_text_property_modification(buf_id, byte_range);
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
        if step.is_multiple_of(2) {
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
    if eq_value(&existing_value, &new_face) {
        return Ok(existing_value);
    }

    if existing_value.is_cons() && !is_anonymous_face_plist(&existing_value) {
        if append {
            if let Some(mut items) = list_to_vec(&existing_value) {
                items.push(new_face);
                return Ok(Value::list(items));
            }
            return Err(signal(
                LispCondition::WrongTypeArgument,
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
    expect_min_args("add-face-text-property", &args, 3)?;
    expect_max_args("add-face-text-property", &args, 5)?;
    verify_property_change_read_only(eval, &args, 4)?;
    let change =
        buffer_property_range_for_args(eval, &args, 4)?.and_then(|(buf_id, byte_range)| {
            let buf = eval.buffers.get(buf_id)?;
            let new_face = args[2];
            (!buf.text_props_range_has_all_properties_in_emacs_byte_range(
                byte_range,
                &[(Value::symbol("face"), new_face)],
            ))
            .then_some((buf_id, byte_range))
        });
    let before = if let Some((buf_id, byte_range)) = change {
        Some(begin_buffer_text_property_change(eval, buf_id, byte_range)?)
    } else {
        None
    };
    let result = builtin_add_face_text_property_in_buffers(&mut eval.buffers, args.clone())?;
    if let Some((saved_current, change)) = before {
        finish_buffer_text_property_change(eval, saved_current, change)?;
    }
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
        let Some(char_range) = validate_string_range(s, beg, end, args[0], args[1])? else {
            return Ok(Value::NIL);
        };
        let char_beg = char_range.start().get();
        let char_end = char_range.end().get();
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        // GNU iterates intervals in [beg, end); per interval, fetch its existing
        // face value and merge. Walk the range segment-by-segment.
        let mut seg_start = char_beg;
        while seg_start < char_end {
            let seg_end =
                match table.next_property_change_after_char_pos(string_char_pos(seg_start)) {
                    Some(p) if p.get() < char_end => p.get(),
                    _ => char_end,
                };
            let existing =
                table.get_property_at_char_pos(string_char_pos(seg_start), Value::symbol("face"));
            let merged = merge_face_property(existing, new_face, append)?;
            table.put_property_for_object_char_len(
                CharRange::new(CharPos0::new(seg_start), CharPos0::new(seg_end)),
                string_char_len(s.schars()),
                Value::symbol("face"),
                merged,
            );
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
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("buffer-or-string-p"), *other],
        )),
    }?;

    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
    let Some(byte_range) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
        return Ok(Value::NIL);
    };
    // GNU iterates intervals in [beg, end); per interval, fetch its existing
    // face value and merge. Walk the range segment-by-segment to preserve any
    // heterogeneous face properties already present.
    let mut segments: Vec<(EmacsByteRange, Value)> = Vec::new();
    let byte_end_pos = byte_range.end();
    let mut seg_start = byte_range.start();
    while seg_start < byte_end_pos {
        let seg_end = match buf.text_props_next_change_after_emacs_byte_pos(seg_start) {
            Some(p) if p < byte_end_pos => p,
            _ => byte_end_pos,
        };
        let existing =
            buf.text_props_get_property_at_emacs_byte_pos(seg_start, Value::symbol("face"));
        let merged = merge_face_property(existing, new_face, append)?;
        segments.push((EmacsByteRange::new(seg_start, seg_end), merged));
        seg_start = seg_end;
    }
    let mut any_changed = false;
    for (byte_range, merged) in segments {
        if buffers
            .put_buffer_text_property_in_emacs_byte_range(
                buf_id,
                byte_range,
                Value::symbol("face"),
                merged,
            )
            .unwrap_or(false)
        {
            any_changed = true;
        }
    }
    if any_changed {
        let _ = buffers.record_buffer_text_property_modification(buf_id, byte_range);
    }
    Ok(Value::NIL)
}

/// (remove-text-properties BEG END PROPS &optional OBJECT)
pub(crate) fn builtin_remove_text_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("remove-text-properties", &args, 3)?;
    expect_max_args("remove-text-properties", &args, 4)?;
    verify_property_change_read_only(eval, &args, 3)?;
    let names_for_probe = plist_names_for_remove(args[2]);
    let change =
        buffer_property_range_for_args(eval, &args, 3)?.and_then(|(buf_id, byte_range)| {
            let buf = eval.buffers.get(buf_id)?;
            buf.text_props_range_has_any_property_named_in_emacs_byte_range(
                byte_range,
                &names_for_probe,
            )
            .then_some((buf_id, byte_range))
        });
    let before = if let Some((buf_id, byte_range)) = change {
        Some(begin_buffer_text_property_change(eval, buf_id, byte_range)?)
    } else {
        None
    };
    let result = builtin_remove_text_properties_in_buffers(&mut eval.buffers, args.clone())?;
    if let Some((saved_current, change)) = before {
        finish_buffer_text_property_change(eval, saved_current, change)?;
    }
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
        let Some(char_range) = validate_string_range(s, beg, end, args[0], args[1])? else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let mut any_removed = false;
        for name in names {
            if table.remove_property_in_char_range(char_range, name) {
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

    let Some(byte_range) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
        return Ok(Value::NIL);
    };
    let mut any_removed = false;
    for name in names {
        if buffers
            .remove_buffer_text_property_in_emacs_byte_range(buf_id, byte_range, name)
            .unwrap_or(false)
        {
            any_removed = true;
        }
    }
    if any_removed {
        let _ = buffers.record_buffer_text_property_modification(buf_id, byte_range);
    }
    Ok(if any_removed { Value::T } else { Value::NIL })
}

/// (set-text-properties BEG END PROPS &optional OBJECT)
pub(crate) fn builtin_set_text_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("set-text-properties", &args, 3)?;
    expect_max_args("set-text-properties", &args, 4)?;
    verify_property_change_read_only(eval, &args, 3)?;
    let pairs_for_probe = if args[2].is_nil() {
        Vec::new()
    } else {
        plist_pairs(&args[2])?
    };
    let change =
        buffer_property_range_for_args(eval, &args, 3)?.and_then(|(buf_id, byte_range)| {
            let buf = eval.buffers.get(buf_id)?;
            (!pairs_for_probe.is_empty()
                || buf.text_props_range_has_any_interval_in_emacs_byte_range(byte_range))
            .then_some((buf_id, byte_range))
        });
    let before = if let Some((buf_id, byte_range)) = change {
        Some(begin_buffer_text_property_change(eval, buf_id, byte_range)?)
    } else {
        None
    };
    let result = builtin_set_text_properties_in_buffers(&mut eval.buffers, args.clone())?;
    if let Some((saved_current, change)) = before {
        finish_buffer_text_property_change(eval, saved_current, change)?;
    }
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
        let Some(char_range) = validate_string_range(s, beg, end, args[0], args[1])? else {
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
        table.set_properties_for_object_char_len(char_range, string_char_len(s.schars()), pairs);
        save_string_props_for_value(str_val, table);
        return Ok(Value::T);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(3))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some(byte_range) = validate_buffer_property_range(buf, beg, end, args[0], args[1])? else {
        return Ok(Value::NIL);
    };
    let _ = buffers.set_buffer_text_properties_in_emacs_byte_range(buf_id, byte_range, pairs);
    let _ = buffers.record_buffer_text_property_modification(buf_id, byte_range);
    Ok(Value::T)
}

/// (remove-list-of-text-properties BEG END LIST &optional OBJECT)
pub(crate) fn builtin_remove_list_of_text_properties(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("remove-list-of-text-properties", &args, 3)?;
    expect_max_args("remove-list-of-text-properties", &args, 4)?;
    verify_property_change_read_only(eval, &args, 3)?;
    let names_for_probe = list_names_for_remove(args[2]);
    let change =
        buffer_property_range_for_args(eval, &args, 3)?.and_then(|(buf_id, byte_range)| {
            let buf = eval.buffers.get(buf_id)?;
            buf.text_props_range_has_any_property_named_in_emacs_byte_range(
                byte_range,
                &names_for_probe,
            )
            .then_some((buf_id, byte_range))
        });
    let before = if let Some((buf_id, byte_range)) = change {
        Some(begin_buffer_text_property_change(eval, buf_id, byte_range)?)
    } else {
        None
    };
    let result =
        builtin_remove_list_of_text_properties_in_buffers(&mut eval.buffers, args.clone())?;
    if let Some((saved_current, change)) = before {
        finish_buffer_text_property_change(eval, saved_current, change)?;
    }
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
        let Some(char_range) = validate_string_range(s, beg, end, args[0], args[1])? else {
            return Ok(Value::NIL);
        };
        let mut table = get_string_text_properties_table_for_value(str_val).unwrap_or_default();
        let mut changed = false;
        for name in names {
            if table.remove_property_in_char_range(char_range, name) {
                changed = true;
            }
        }
        save_string_props_for_value(str_val, table);
        return Ok(if changed { Value::T } else { Value::NIL });
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(3))?;
    let byte_range = {
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
        let byte_end_pos = byte_range.end();
        let mut cursor = byte_range.start();
        while cursor < byte_end_pos {
            let Some(buf) = buffers.get(buf_id) else {
                break;
            };
            if buf
                .text_props_get_property_at_emacs_byte_pos(cursor, name)
                .is_some()
            {
                changed = true;
                break;
            }
            match buf.text_props_next_change_after_emacs_byte_pos(cursor) {
                Some(next) if next > cursor && next < byte_end_pos => cursor = next,
                _ => break,
            }
        }
        let _ = buffers.remove_buffer_text_property_in_emacs_byte_range(buf_id, byte_range, name);
    }
    if changed {
        let _ = buffers.record_buffer_text_property_modification(buf_id, byte_range);
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
        let char_pos = validate_string_char_pos_raw(s, pos, args[0])?;
        if char_pos.get() == s.schars() {
            return Ok(Value::NIL);
        }
        if let Some(table) = get_string_text_properties_table_for_value(str_val) {
            return Ok(table.get_properties_plist_value_at_char_pos(char_pos));
        }
        return Ok(Value::NIL);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(1))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = validate_buffer_property_point_emacs_byte_pos_raw(buf, pos, args[0])?;
    if byte_pos == buffer_end_emacs_byte_pos(buf) {
        return Ok(Value::NIL);
    }
    Ok(buf.text_props_get_properties_plist_value_at_emacs_byte_pos(byte_pos))
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
        let char_pos = validate_string_char_pos_raw(s, pos, args[0])?;
        let (limit_pos, limit_val) = match args.get(3) {
            Some(v) if !v.is_nil() => {
                let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
                (Some(lim_int), Some(lim_int))
            }
            _ => (None, None),
        };
        let current_val =
            lookup_string_text_property(obarray, buffers, &table, char_pos.get(), prop);
        let str_len = s.schars();
        let mut cursor = char_pos;
        while let Some(next) = table.next_interval_boundary_after_char_pos(cursor) {
            let next = next.get();
            if let Some(lim) = limit_pos
                && next as i64 >= lim
            {
                return Ok(match limit_val {
                    Some(lv) => Value::fixnum(lv),
                    None => Value::NIL,
                });
            }
            if next >= str_len {
                break;
            }
            let new_val = lookup_string_text_property(obarray, buffers, &table, next, prop);
            let changed = !eq_value(&current_val, &new_val);
            if changed {
                return Ok(Value::fixnum(string_char_to_elisp_pos(
                    s,
                    string_char_pos(next),
                )));
            }
            cursor = string_char_pos(next);
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

    let byte_pos = validate_buffer_property_point_emacs_byte_pos_raw(buf, pos, args[0])?;
    let (limit_pos, limit_val) = match args.get(3) {
        Some(v) if !v.is_nil() => {
            let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
            (Some(lim_int), Some(lim_int))
        }
        _ => (None, None),
    };

    let current_val = lookup_buffer_text_property(obarray, buffers, buf, byte_pos.get(), prop);
    let buf_end = buf.accessible_emacs_byte_region().end();
    let mut cursor = byte_pos;

    while let Some(next) = buf.text_props_next_interval_boundary_after_emacs_byte_pos(cursor) {
        if let Some(lim) = limit_pos
            && byte_to_elisp_pos(buf, next) >= lim
        {
            return Ok(match limit_val {
                Some(lv) => Value::fixnum(lv),
                None => Value::NIL,
            });
        }
        if next >= buf_end {
            break;
        }
        let new_val = lookup_buffer_text_property(obarray, buffers, buf, next.get(), prop);
        let changed = !eq_value(&current_val, &new_val);
        if changed {
            return Ok(Value::fixnum(byte_to_elisp_pos(buf, next)));
        }
        cursor = next;
    }

    Ok(match limit_val {
        Some(lv) => Value::fixnum(lv),
        None => Value::NIL,
    })
}

/// Byte position of the character immediately preceding `byte_pos`.
///
/// GNU's `previous-single-property-change` inspects the property of the
/// character *before* a position/boundary using a one-*character* step
/// (`position - 1`, since GNU works in character positions).  In a multibyte
/// buffer that character can be several bytes back, so a one-*byte* decrement
/// would land mid-character and trip the Emacs-char-boundary assertion in
/// `emacs_byte_pos_to_char_pos`.  `byte_pos` must already be a character
/// boundary (validated points and interval boundaries always are).
pub(crate) fn emacs_byte_pos_of_preceding_char(
    buf: &Buffer,
    byte_pos: EmacsBytePos,
) -> EmacsBytePos {
    if byte_pos <= EmacsBytePos::ZERO {
        return EmacsBytePos::ZERO;
    }
    let char_pos = buf.emacs_byte_pos_to_char_pos_clamped(byte_pos);
    let prev_char = char_pos.saturating_sub_len(CharLen::new(1));
    EmacsBytePos::new(buffer_char_to_byte_pos(buf, prev_char))
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
        let char_pos = validate_string_char_pos_raw(s, pos, args[0])?;
        let (limit_pos, limit_val) = match args.get(3) {
            Some(v) if !v.is_nil() => {
                let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
                (Some(lim_int), Some(lim_int))
            }
            _ => (None, None),
        };
        let ref_char = char_pos.saturating_sub_len(CharLen::new(1));
        let current_val =
            lookup_string_text_property(obarray, buffers, &table, ref_char.get(), prop);
        let mut cursor = char_pos;
        while let Some(prev) = table.previous_interval_boundary_before_char_pos(cursor) {
            if let Some(lim) = limit_pos
                && (prev.get() as i64) <= lim
            {
                return Ok(match limit_val {
                    Some(lv) => Value::fixnum(lv),
                    None => Value::NIL,
                });
            }
            let check = prev.saturating_sub_len(CharLen::new(1));
            let new_val = lookup_string_text_property(obarray, buffers, &table, check.get(), prop);
            let changed = !eq_value(&current_val, &new_val);
            if changed {
                return Ok(Value::fixnum(string_char_to_elisp_pos(s, prev)));
            }
            if prev == CharPos0::ZERO {
                break;
            }
            cursor = if prev < cursor {
                prev
            } else {
                prev.saturating_sub_len(CharLen::new(1))
            };
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

    let byte_pos = validate_buffer_property_point_emacs_byte_pos_raw(buf, pos, args[0])?;
    let (limit_pos, limit_val) = match args.get(3) {
        Some(v) if !v.is_nil() => {
            let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
            (Some(lim_int), Some(lim_int))
        }
        _ => (Some(buf.point_min_lisp_char_pos().as_i64()), None),
    };

    let ref_byte = emacs_byte_pos_of_preceding_char(buf, byte_pos);
    let current_val = lookup_buffer_text_property(obarray, buffers, buf, ref_byte.get(), prop);
    let mut cursor = byte_pos;

    while let Some(prev) = buf.text_props_previous_interval_boundary_before_emacs_byte_pos(cursor) {
        if let Some(lim) = limit_pos
            && byte_to_elisp_pos(buf, prev) <= lim
        {
            return Ok(match limit_val {
                Some(lv) => Value::fixnum(lv),
                None => Value::NIL,
            });
        }
        let check = emacs_byte_pos_of_preceding_char(buf, prev);
        let new_val = lookup_buffer_text_property(obarray, buffers, buf, check.get(), prop);
        let changed = !eq_value(&current_val, &new_val);
        if changed {
            return Ok(Value::fixnum(byte_to_elisp_pos(buf, prev)));
        }
        if prev == EmacsBytePos::ZERO {
            break;
        }
        cursor = if prev < cursor {
            prev
        } else {
            emacs_byte_pos_of_preceding_char(buf, prev)
        };
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
        let char_pos = validate_string_char_pos_raw(s, pos, args[0])?;
        let limit_arg = args.get(2);
        if limit_arg.is_some_and(|v| v.is_t()) {
            let next = table
                .next_interval_boundary_after_char_pos(char_pos)
                .map(|pos| pos.get())
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
        return match table.next_property_change_after_char_pos(char_pos) {
            Some(next) => {
                let next = next.get();
                if let Some(lim) = limit_pos
                    && (next as i64) >= lim
                {
                    return Ok(limit_val.unwrap_or(Value::NIL));
                }
                // If the change is at or past the end of the string, treat as no change
                if next >= str_char_len {
                    return Ok(limit_val.unwrap_or(Value::NIL));
                }
                Ok(Value::fixnum(string_char_to_elisp_pos(
                    s,
                    string_char_pos(next),
                )))
            }
            None => Ok(limit_val.unwrap_or(Value::NIL)),
        };
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(1))?;
    let limit_arg = args.get(2);

    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_pos = validate_buffer_property_point_emacs_byte_pos_raw(buf, pos, args[0])?;
    if limit_arg.is_some_and(|v| v.is_t()) {
        let next = buf
            .text_props_next_interval_boundary_after_emacs_byte_pos(byte_pos)
            .unwrap_or_else(|| buf.accessible_emacs_byte_region().end());
        return Ok(Value::fixnum(byte_to_elisp_pos(buf, next)));
    }
    let (limit_pos, limit_val) = match limit_arg {
        Some(v) if !v.is_nil() => {
            let lim_int = expect_integer_or_marker_in_buffers(buffers, v)?;
            (Some(lim_int), Some(Value::fixnum(lim_int)))
        }
        _ => (None, None),
    };
    let buf_end = buf.accessible_emacs_byte_region().end();

    match buf.text_props_next_change_after_emacs_byte_pos(byte_pos) {
        Some(next) => {
            if let Some(lim) = limit_pos
                && byte_to_elisp_pos(buf, next) >= lim
            {
                return Ok(limit_val.unwrap_or(Value::NIL));
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
        let Some(char_range) = validate_string_range(s, beg, end, args[0], args[1])? else {
            return Ok(Value::NIL);
        };
        let char_beg = char_range.start().get();
        let char_end = char_range.end().get();
        let Some(table) = get_string_text_properties_interval_table_for_value(str_val) else {
            return Ok(if val.is_nil() {
                if char_beg < char_end {
                    Value::fixnum(string_char_to_elisp_pos(s, string_char_pos(char_beg)))
                } else {
                    Value::NIL
                }
            } else {
                Value::NIL
            });
        };
        if val.is_nil() {
            let mut cursor = char_beg;
            while cursor < char_end {
                let found = lookup_string_text_property(obarray, buffers, &table, cursor, prop);
                if found.is_nil() {
                    return Ok(Value::fixnum(string_char_to_elisp_pos(
                        s,
                        string_char_pos(cursor),
                    )));
                }
                match table.next_interval_boundary_after_char_pos(string_char_pos(cursor)) {
                    Some(next) if next.get() <= char_end => cursor = next.get(),
                    _ => break,
                }
            }
            return Ok(Value::NIL);
        }
        let mut cursor = char_beg;
        while cursor < char_end {
            let found = lookup_string_text_property(obarray, buffers, &table, cursor, prop);
            if eq_value(&found, val) {
                return Ok(Value::fixnum(string_char_to_elisp_pos(
                    s,
                    string_char_pos(cursor),
                )));
            }
            match table.next_interval_boundary_after_char_pos(string_char_pos(cursor)) {
                Some(next) if next.get() > cursor && next.get() <= char_end => cursor = next.get(),
                _ => break,
            }
        }
        return Ok(Value::NIL);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(4))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some(byte_range) =
        validate_buffer_property_emacs_byte_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };
    let byte_beg = byte_range.start();
    let byte_end = byte_range.end();

    if buf.text_props_is_empty() {
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

    if val.is_nil() {
        let mut cursor = byte_beg;
        while cursor < byte_end {
            let found =
                lookup_buffer_text_property_at_emacs_byte_pos(obarray, buffers, buf, cursor, prop);
            if found.is_nil() {
                return Ok(Value::fixnum(byte_to_elisp_pos(buf, cursor)));
            }
            match buf.text_props_next_interval_boundary_after_emacs_byte_pos(cursor) {
                Some(next) if next <= byte_end => {
                    cursor = next;
                }
                _ => break,
            }
        }
        return Ok(Value::NIL);
    }
    let mut cursor = byte_beg;
    while cursor < byte_end {
        let found =
            lookup_buffer_text_property_at_emacs_byte_pos(obarray, buffers, buf, cursor, prop);
        if eq_value(&found, val) {
            return Ok(Value::fixnum(byte_to_elisp_pos(buf, cursor)));
        }
        match buf.text_props_next_interval_boundary_after_emacs_byte_pos(cursor) {
            Some(next) if next > cursor && next <= byte_end => cursor = next,
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
        let Some(char_range) = validate_string_range(s, beg, end, args[0], args[1])? else {
            return Ok(Value::NIL);
        };
        let char_beg = char_range.start().get();
        let char_end = char_range.end().get();
        let Some(table) = get_string_text_properties_interval_table_for_value(str_val) else {
            return Ok(if val.is_nil() {
                Value::NIL
            } else if char_beg < char_end {
                Value::fixnum(string_char_to_elisp_pos(s, string_char_pos(char_beg)))
            } else {
                Value::NIL
            });
        };
        let mut cursor = char_beg;
        while cursor < char_end {
            let found = lookup_string_text_property(obarray, buffers, &table, cursor, prop);
            let matches = eq_value(&found, val);
            if !matches {
                return Ok(Value::fixnum(string_char_to_elisp_pos(
                    s,
                    string_char_pos(cursor),
                )));
            }
            match table.next_property_change_after_char_pos(string_char_pos(cursor)) {
                Some(next) if next.get() > cursor && next.get() < char_end => cursor = next.get(),
                _ => break,
            }
        }
        return Ok(Value::NIL);
    }

    let buf_id = resolve_text_property_buffer_id_in_buffers(buffers, args.get(4))?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let Some(byte_range) =
        validate_buffer_property_emacs_byte_range(buf, beg, end, args[0], args[1])?
    else {
        return Ok(Value::NIL);
    };
    let byte_beg = byte_range.start();
    let byte_end = byte_range.end();

    if buf.text_props_is_empty() {
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
        let found =
            lookup_buffer_text_property_at_emacs_byte_pos(obarray, buffers, buf, cursor, prop);
        let matches = eq_value(&found, val);
        if !matches {
            return Ok(Value::fixnum(byte_to_elisp_pos(buf, cursor)));
        }

        match buf.text_props_next_change_after_emacs_byte_pos(cursor) {
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
        let byte_pos = validate_buffer_point_emacs_byte_pos_raw(buf, pos, args[0])?;
        if byte_pos == buffer_end_emacs_byte_pos(buf) {
            return Ok(Value::cons(Value::NIL, Value::NIL));
        }
        if let Some((value, ov_val)) = buffer_overlay_property_at_byte_pos(
            obarray,
            buffers,
            buf,
            byte_pos.get(),
            prop,
            window_id,
        ) {
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

    let byte_pos = elisp_pos_to_byte_clipped_full(buf, LispCharPos1::new(pos));
    let accessible = buf.accessible_emacs_byte_region();
    match buf
        .overlays
        .next_boundary_after_until_emacs_byte_pos(byte_pos, accessible.end())
    {
        Some(next) => Ok(Value::fixnum(byte_to_elisp_pos(buf, next))),
        None => Ok(Value::fixnum(byte_to_elisp_pos(buf, accessible.end()))),
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

    let byte_pos = elisp_pos_to_byte_clipped_full(buf, LispCharPos1::new(pos));
    let accessible = buf.accessible_emacs_byte_region();
    match buf
        .overlays
        .previous_boundary_before_since_emacs_byte_pos(byte_pos, accessible.start())
    {
        Some(prev) => Ok(Value::fixnum(byte_to_elisp_pos(buf, prev))),
        None => Ok(Value::fixnum(byte_to_elisp_pos(buf, accessible.start()))),
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

    let byte_range = elisp_range_to_byte_clipped_full(buf, beg, end);
    let overlay = Value::make_overlay(crate::heap_types::OverlayData {
        serial: 0,
        plist: Value::NIL,
        buffer: Some(buf_id),
        start: byte_range.start().get(),
        end: byte_range.end().get(),
        front_advance,
        rear_advance,
    });
    buf.overlays.insert_overlay(overlay);
    // Creating an overlay changes what redisplay must consider (it can carry a
    // face/display/before-string the moment a property is attached), so bump
    // the modification tick here — matching move/put/delete, which already do.
    buf.increment_overlay_modified_tick();
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
    if let Some(buf_id) = resolve_overlay_buffer_id(overlay)
        && changed
    {
        if let Some(buf) = buffers.get_mut(buf_id) {
            buf.increment_overlay_modified_tick();
        }
        let evaporate = args[1].is_symbol_named("evaporate") && val.is_truthy();
        let is_empty = buffers
            .get(buf_id)
            .and_then(|buf| {
                let start = buf.overlays.overlay_start_emacs_byte_pos(overlay)?;
                let end = buf.overlays.overlay_end_emacs_byte_pos(overlay)?;
                Some(start == end)
            })
            .unwrap_or(false);
        if evaporate && is_empty {
            let _ = buffers.delete_buffer_overlay(buf_id, overlay);
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

    let byte_pos = elisp_pos_to_byte_clipped_full(buf, LispCharPos1::new(pos));
    let mut ids = buf.overlays.overlays_at_emacs_byte_pos(byte_pos);
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
    // GNU `overlays_in` (buffer.c) treats BEG > END as an empty region: the
    // interval-tree walk `[beg, search_end)` is empty and the very first node
    // (whose `begin > end`) breaks the loop, so no overlays are returned.
    // Unlike `make-overlay`/`move-overlay`, `overlays-in` must NOT swap the
    // endpoints, so guard before the (swapping) clip helper.
    if beg > end {
        return Ok(Value::NIL);
    }
    let buf_id = current_buffer_id_in_buffers(buffers)?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

    let byte_range = elisp_range_to_byte_clipped_full(buf, beg, end);
    let accessible = buf.accessible_emacs_byte_region();
    let ids = buf
        .overlays
        .overlays_in_accessible_emacs_byte_range(byte_range, accessible.end());
    Ok(Value::list(ids))
}

/// (overlay-lists)
///
/// Mirrors GNU `Foverlay_lists` (buffer.c). Returns `(BEFORE . AFTER)`: the
/// car holds every overlay of the current buffer, the cdr is always empty.
/// GNU's docstring still describes the pair as the overlays before/after the
/// "overlay center", but since Emacs 29.1 (commit moving overlays to the
/// `itree` interval tree) that center no longer exists: `Foverlay_lists`
/// conses every node of `current_buffer->overlays` (walked `BEG..Z`
/// DESCENDING, which reverses back to ascending `begin` order) into a single
/// list and returns `(cons overlays Qnil)`. Even for an empty buffer GNU
/// returns `(nil)` (i.e. `(cons nil nil)`), never bare `nil`.
pub(crate) fn builtin_overlay_lists(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_overlay_lists_in_buffers(&eval.buffers, args)
}

pub(crate) fn builtin_overlay_lists_in_buffers(
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlay-lists", &args, 0)?;
    let buf_id = current_buffer_id_in_buffers(buffers)?;
    let buf = buffers
        .get(buf_id)
        .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
    let before = Value::list(buf.overlays.overlays_in_gnu_lists_order());
    Ok(Value::cons(before, Value::NIL))
}

/// (overlay-recenter POS)
///
/// Mirrors GNU `Foverlay_recenter` (buffer.c): since Emacs 29.1 this is a
/// no-op (overlay lookup is fast at any position with the `itree` store), but
/// it still type-checks POS as a fixnum-or-marker and returns nil.
pub(crate) fn builtin_overlay_recenter(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("overlay-recenter", &args, 1)?;
    let _ = expect_integer_or_marker_in_buffers(&eval.buffers, &args[0])?;
    Ok(Value::NIL)
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
        let byte_range = elisp_range_to_byte_clipped_full(buf, beg, end);
        buf.overlays
            .move_overlay_to_emacs_byte_range(overlay, byte_range);
        buf.increment_overlay_modified_tick();
        Ok(args[0])
    } else {
        if let Some(old_buf_id) = old_buf_id
            && let Some(buf) = buffers.get_mut(old_buf_id)
            && buf.overlays.detach_overlay(overlay)
        {
            buf.increment_overlay_modified_tick();
        }

        let new_buf = buffers
            .get_mut(new_buf_id)
            .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;
        let byte_range = elisp_range_to_byte_clipped_full(new_buf, beg, end);
        let _ = overlay.with_overlay_data_mut(|object| {
            object.buffer = Some(new_buf_id);
            object.start = byte_range.start().get();
            object.end = byte_range.end().get();
        });
        new_buf.overlays.insert_overlay(overlay);
        new_buf.increment_overlay_modified_tick();
        if byte_range.is_empty()
            && new_buf
                .overlays
                .overlay_get_named(overlay, Value::symbol("evaporate"))
                .is_some_and(|value| value.is_truthy())
            && new_buf.overlays.delete_overlay(overlay)
        {
            new_buf.increment_overlay_modified_tick();
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

    match buf.overlays.overlay_start_emacs_byte_pos(overlay) {
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

    match buf.overlays.overlay_end_emacs_byte_pos(overlay) {
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
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
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
            buf.point_min_emacs_byte_pos()
        } else {
            elisp_pos_to_byte_clipped_full(buf, LispCharPos1::new(expect_int_eval(eval, &args[0])?))
        };
        let end = if args.len() < 2 || args[1].is_nil() {
            buf.point_max_emacs_byte_pos()
        } else {
            elisp_pos_to_byte_clipped_full(buf, LispCharPos1::new(expect_int_eval(eval, &args[1])?))
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
    let accessible = buf.accessible_emacs_byte_region();
    let ids = buf.overlays.overlays_in_accessible_emacs_byte_range(
        EmacsByteRange::new(start_pos, end_pos),
        accessible.end(),
    );

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
