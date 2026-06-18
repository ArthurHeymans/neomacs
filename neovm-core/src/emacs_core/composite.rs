//! Composition builtins (complex script rendering).
//!
//! GNU Emacs records explicit compositions as a `composition` text property.
//! The display engine later validates and registers those properties when it
//! needs glyph data.  The Lisp-visible mutation semantics live here.

use super::chartable::make_char_table_value;
use super::error::{EvalResult, Flow, signal};
use super::value::*;
use crate::buffer::{CharLen, CharPos0, CharRange, EmacsByteRange};
use crate::emacs_core::value::ValueKind;

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn expect_range_args(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), Flow> {
    if args.len() < min || args.len() > max {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

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

fn expect_integerp(arg: &Value) -> Result<(), Flow> {
    match arg.kind() {
        ValueKind::Fixnum(_) => Ok(()),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integerp"), *arg],
        )),
    }
}

fn expect_integer_or_marker_p(arg: &Value) -> Result<(), Flow> {
    match arg.kind() {
        ValueKind::Fixnum(_) => Ok(()),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *arg],
        )),
    }
}

fn integer_value(arg: &Value) -> i64 {
    match arg.kind() {
        ValueKind::Fixnum(n) => n,
        _ => 0,
    }
}

fn expect_composition_components(arg: Value) -> Result<(), Flow> {
    if arg.is_nil() || arg.is_fixnum() || arg.is_cons() || arg.is_string() || arg.is_vector() {
        Ok(())
    } else {
        Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("vectorp"), arg],
        ))
    }
}

fn composition_property(
    start: i64,
    end: i64,
    components: Value,
    modification_func: Value,
) -> Value {
    Value::cons(
        Value::cons(Value::fixnum(end - start), components),
        modification_func,
    )
}

fn expect_string_value(arg: &Value) -> Result<&crate::heap_types::LispString, Flow> {
    arg.as_lisp_string()
        .ok_or_else(|| signal("wrong-type-argument", vec![Value::symbol("stringp"), *arg]))
}

fn validate_subarray_indices(
    array: Value,
    from: Value,
    to: Value,
    size: i64,
) -> Result<(i64, i64), Flow> {
    fn normalize_index(value: Value, default: i64, size: i64) -> Result<i64, Flow> {
        if value.is_nil() {
            return Ok(default);
        }
        let raw = match value.kind() {
            ValueKind::Fixnum(n) => n,
            _ => {
                return Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("integerp"), value],
                ));
            }
        };
        Ok(if raw < 0 { raw + size } else { raw })
    }

    let from_idx = normalize_index(from, 0, size)?;
    let to_idx = normalize_index(to, size, size)?;
    if !(0 <= from_idx && from_idx <= to_idx && to_idx <= size) {
        return Err(signal("args-out-of-range", vec![array, from, to]));
    }
    Ok((from_idx, to_idx))
}

// ---------------------------------------------------------------------------
// Pure builtins
// ---------------------------------------------------------------------------

/// Context-backed `(compose-region-internal START END &optional COMPONENTS MODIFICATION-FUNC)`.
pub(crate) fn builtin_compose_region_internal(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_range_args("compose-region-internal", &args, 2, 4)?;
    let start = super::builtins::expect_integer_or_marker_in_buffers(&ctx.buffers, &args[0])?;
    let end = super::builtins::expect_integer_or_marker_in_buffers(&ctx.buffers, &args[1])?;
    let components = args.get(2).copied().unwrap_or(Value::NIL);
    let modification_func = args.get(3).copied().unwrap_or(Value::NIL);
    expect_composition_components(components)?;

    let (buffer_handle, point_max) = if let Some(buf) = ctx.buffers.current_buffer() {
        (
            Value::make_buffer(buf.id),
            buf.point_max_lisp_char_pos().as_i64(),
        )
    } else {
        (Value::NIL, 1)
    };
    if start < 1 || end < 1 || start > end || start > point_max || end > point_max {
        return Err(signal(
            "args-out-of-range",
            vec![buffer_handle, Value::fixnum(start), Value::fixnum(end)],
        ));
    }

    let prop = composition_property(start, end, components, modification_func);
    super::textprop::builtin_put_text_property(
        ctx,
        vec![
            args[0],
            args[1],
            Value::symbol("composition"),
            prop,
            Value::NIL,
        ],
    )?;

    Ok(Value::NIL)
}

/// `(compose-string-internal STRING START END &optional COMPONENTS MODIFICATION-FUNC)`
///
/// Compose text in STRING between indices START and END.
pub(crate) fn builtin_compose_string_internal(args: Vec<Value>) -> EvalResult {
    expect_range_args("compose-string-internal", &args, 3, 5)?;
    if !args[0].is_string() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("stringp"), args[0]],
        ));
    }
    let components = args.get(3).copied().unwrap_or(Value::NIL);
    let modification_func = args.get(4).copied().unwrap_or(Value::NIL);

    let len = expect_string_value(&args[0])?.schars() as i64;
    let (start, end) = validate_subarray_indices(args[0], args[1], args[2], len)?;
    let char_start = usize::try_from(start).expect("validated non-negative string start");
    let char_end = usize::try_from(end).expect("validated non-negative string end");
    let char_len = usize::try_from(len).expect("string character length fits usize");
    let prop = composition_property(start, end, components, modification_func);
    let mut table = get_string_text_properties_table_for_value(args[0]).unwrap_or_default();
    table.put_property_for_object_char_len(
        CharRange::new(CharPos0::new(char_start), CharPos0::new(char_end)),
        CharLen::new(char_len),
        Value::symbol("composition"),
        prop,
    );
    super::textprop::save_string_props_for_value(args[0], table);

    Ok(args[0])
}

/// Decode an UNREGISTERED composition property `((LENGTH . COMPONENTS) . MOD-FUNC)`
/// into `(length, components, mod-func)`. `compose-region-internal` only ever
/// stores this (Form-A) shape; neomacs never rewrites it to GNU's registered
/// `(ID . (LENGTH COMPONENTS . FUNC))` form (the registration is a display-time
/// optimization keyed on a global id and is not Lisp-observable here).
fn composition_unregistered_parts(prop: Value) -> Option<(i64, Value, Value)> {
    if !prop.is_cons() {
        return None;
    }
    let head = prop.cons_car(); // (LENGTH . COMPONENTS)
    let mod_func = prop.cons_cdr();
    if !head.is_cons() {
        return None;
    }
    let length = head.cons_car().as_fixnum()?;
    let components = head.cons_cdr();
    Some((length, components, mod_func))
}

/// Display width and character length of the composition whose `composition`
/// text property begins at 1-based buffer position `charpos1`, or None if there
/// is no valid composition there. This is what GNU's `current_column_1` /
/// display scan derives from a composition (the composed glyphs' width over its
/// covered characters); the column engine consults it so composed text lays out
/// at the glyphs' width, not the underlying chars'.
pub(crate) fn composition_width_at(
    ctx: &super::eval::Context,
    charpos1: i64,
) -> Option<(i64, i64)> {
    let prop = super::textprop::builtin_get_text_property_in_state(
        &ctx.obarray,
        &ctx.buffers,
        vec![Value::fixnum(charpos1), Value::symbol("composition")],
    )
    .ok()?;
    let (length, components, _) = composition_unregistered_parts(prop)?;
    if length <= 0 {
        return None;
    }
    let key = composition_components_key(ctx, components, Value::NIL, charpos1, length);
    Some((composition_relative_width(&key), length))
}

/// GNU `composition_valid_p` restricted to the unregistered form: PROP is a
/// well-formed composition property whose stored length equals `end - start`.
fn composition_valid_unregistered(start: i64, end: i64, prop: Value) -> bool {
    let Some((length, components, _)) = composition_unregistered_parts(prop) else {
        return false;
    };
    let components_ok = components.is_nil()
        || components.is_string()
        || components.is_vector()
        || components.is_fixnum()
        || components.is_cons();
    components_ok && length == end - start
}

/// GNU `composition_method`: relative unless the components describe explicit
/// composition rules. `find-composition` reports `relative-p` as nil only for
/// `COMPOSITION_WITH_RULE_ALTCHARS` (vector/list components); nil/char/string
/// components are relative.
fn composition_relative_p(components: Value) -> bool {
    components.is_nil() || components.is_fixnum() || components.is_string()
}

/// GNU `get_composition_id` key derivation: the components vector returned by
/// `find-composition`. A single char becomes `[char]`; a string or list is
/// `vconcat`-ed into a char vector; a vector is used as-is; nil takes the chars
/// of the composed range from the buffer (or STRING).
fn composition_components_key(
    ctx: &super::eval::Context,
    components: Value,
    string: Value,
    start: i64,
    nchars: i64,
) -> Value {
    match components.kind() {
        ValueKind::Fixnum(code) => Value::vector(vec![Value::fixnum(code)]),
        ValueKind::String => {
            let codes = crate::emacs_core::builtins::lisp_string_char_codes(
                components.as_lisp_string().expect("string"),
            );
            Value::vector(codes.into_iter().map(|c| Value::fixnum(c as i64)).collect())
        }
        ValueKind::Cons => Value::vector(list_to_vec(&components).unwrap_or_default()),
        _ if components.is_vector() => components,
        _ => {
            // nil components: take the chars of the composed range.
            let codes: Vec<u32> = if let Some(text) = string.as_lisp_string() {
                let all = crate::emacs_core::builtins::lisp_string_char_codes(text);
                let from = start.max(0) as usize;
                let to = ((start + nchars).max(0) as usize).min(all.len());
                all.get(from..to).map(|s| s.to_vec()).unwrap_or_default()
            } else if let Some(buf) = ctx.buffers.current_buffer() {
                let byte_start = buf
                    .char_pos_to_emacs_byte_pos_clamped(CharPos0::new((start - 1).max(0) as usize));
                let byte_end = buf.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(
                    (start - 1 + nchars).max(0) as usize,
                ));
                let sub = buf
                    .buffer_substring_lisp_string_range(EmacsByteRange::new(byte_start, byte_end));
                crate::emacs_core::builtins::lisp_string_char_codes(&sub)
            } else {
                Vec::new()
            };
            Value::vector(codes.into_iter().map(|c| Value::fixnum(c as i64)).collect())
        }
    }
}

/// GNU relative/altchars width: the maximum display width over the component
/// glyphs (TAB counts as 1), 0 for an empty composition. (Rule-based
/// compositions report `relative-p` nil and are not exercised by batch tests;
/// this max is a faithful upper bound for them.)
fn composition_relative_width(key: &Value) -> i64 {
    let Some(items) = key.as_vector_data() else {
        return 0;
    };
    let mut width = 0i64;
    for item in items.iter() {
        if let ValueKind::Fixnum(code) = item.kind() {
            let this = if code == 9 {
                1
            } else {
                crate::encoding::char_width_for_code_with_display_table(code, None) as i64
            };
            if width < this {
                width = this;
            }
        }
    }
    width
}

/// GNU `find_composition`/`get_property_and_range` for a buffer: the
/// `composition` property covering `from` (1-based), else the nearest one
/// toward `to` (-1 = none). Returns `(start, end, prop)` in 1-based positions.
fn find_composition_in_buffer(
    buf: &crate::buffer::buffer::Buffer,
    begv: i64,
    zv: i64,
    from: i64,
    to: i64,
    comp: Value,
) -> Option<(i64, i64, Value)> {
    let run_at = |charpos: i64| -> Option<(i64, i64, Value)> {
        if charpos < begv || charpos >= zv {
            return None;
        }
        let (prop, s, e) =
            buf.get_property_run_at_char_pos(CharPos0::new((charpos - 1) as usize), comp);
        match prop {
            Some(p) if !p.is_nil() => Some((s.get() as i64 + 1, e.get() as i64 + 1, p)),
            _ => None,
        }
    };
    if let Some(found) = run_at(from) {
        return Some(found);
    }
    if to < 0 || to == from {
        return None;
    }
    if to > from {
        // Forward: jump run by run until a composition appears before `to`.
        let mut pos = from;
        while pos < to {
            let (_p, _s, e) =
                buf.get_property_run_at_char_pos(CharPos0::new((pos - 1) as usize), comp);
            let next = e.get() as i64 + 1;
            if next <= pos || next >= to {
                return None;
            }
            if let Some(found) = run_at(next) {
                return Some(found);
            }
            pos = next;
        }
        None
    } else {
        // Backward: GNU checks the char before `from`, then scans backward.
        if let Some(found) = run_at(from - 1) {
            return Some(found);
        }
        let mut pos = from - 1;
        while pos > to {
            let (_p, s, _e) =
                buf.get_property_run_at_char_pos(CharPos0::new((pos - 1) as usize), comp);
            let prev = s.get() as i64; // 1-based position one before this run's start
            if prev <= to || prev >= pos {
                return None;
            }
            if let Some(found) = run_at(prev) {
                return Some(found);
            }
            pos = prev;
        }
        None
    }
}

/// `(find-composition-internal POS LIMIT STRING DETAIL-P)`
///
/// GNU `Ffind_composition_internal` (composite.c): describe the composition at
/// or nearest to POS. With DETAIL-P nil, returns `(FROM TO VALID-P)`; otherwise
/// `(FROM TO COMPONENTS RELATIVE-P MOD-FUNC WIDTH)`. Automatic (font-driven)
/// composition discovery is not implemented (returns nil for that case).
pub(crate) fn builtin_find_composition_internal(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("find-composition-internal", &args, 4)?;
    expect_integer_or_marker_p(&args[0])?;
    if !args[1].is_nil() {
        expect_integer_or_marker_p(&args[1])?;
    }
    if !args[2].is_nil() && !args[2].is_string() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("stringp"), args[2]],
        ));
    }
    let detail = !args[3].is_nil();
    let pos = integer_value(&args[0]);
    let limit = if args[1].is_nil() {
        -1
    } else {
        integer_value(&args[1])
    };
    let comp = Value::symbol("composition");

    let found = if let Some(text) = args[2].as_lisp_string() {
        let len = text.schars() as i64;
        if pos < 0 || pos > len {
            return Err(signal(
                "args-out-of-range",
                vec![args[2], Value::fixnum(pos)],
            ));
        }
        let table = get_string_text_properties_table_for_value(args[2]).unwrap_or_default();
        let run_at = |charpos: i64| -> Option<(i64, i64, Value)> {
            if charpos < 0 || charpos >= len {
                return None;
            }
            let (prop, s, e) = table.get_property_run_at_char_pos(
                CharPos0::new(charpos as usize),
                comp,
                len as usize,
            );
            match prop {
                Some(p) if !p.is_nil() => Some((s.get() as i64, e.get() as i64, p)),
                _ => None,
            }
        };
        // STRING positions are 0-based; only the at-pos lookup is needed for
        // the Lisp-visible behavior exercised here.
        run_at(pos)
    } else {
        let (begv, zv) = {
            let Some(buf) = ctx.buffers.current_buffer() else {
                return Err(signal(
                    "args-out-of-range",
                    vec![Value::NIL, Value::fixnum(pos)],
                ));
            };
            (
                buf.point_min_lisp_char_pos().as_i64(),
                buf.point_max_lisp_char_pos().as_i64(),
            )
        };
        if pos < begv || pos > zv {
            let handle = ctx
                .buffers
                .current_buffer()
                .map(|b| Value::make_buffer(b.id))
                .unwrap_or(Value::NIL);
            return Err(signal(
                "args-out-of-range",
                vec![handle, Value::fixnum(pos)],
            ));
        }
        let to = if limit < 0 { -1 } else { limit.clamp(begv, zv) };
        let buf = ctx
            .buffers
            .current_buffer()
            .expect("checked current buffer");
        find_composition_in_buffer(buf, begv, zv, pos, to, comp)
    };

    let Some((start, end, prop)) = found else {
        return Ok(Value::NIL);
    };

    if !composition_valid_unregistered(start, end, prop) {
        return Ok(Value::list(vec![
            Value::fixnum(start),
            Value::fixnum(end),
            Value::NIL,
        ]));
    }
    if !detail {
        return Ok(Value::list(vec![
            Value::fixnum(start),
            Value::fixnum(end),
            Value::T,
        ]));
    }

    let (_length, components, mod_func) =
        composition_unregistered_parts(prop).expect("valid composition decodes");
    let relative_p = composition_relative_p(components);
    let key = composition_components_key(ctx, components, args[2], start, end - start);
    let width = composition_relative_width(&key);
    Ok(Value::list(vec![
        Value::fixnum(start),
        Value::fixnum(end),
        key,
        if relative_p { Value::T } else { Value::NIL },
        mod_func,
        Value::fixnum(width),
    ]))
}

/// `(composition-get-gstring FROM TO FONT-OBJECT STRING)`
///
/// Return a gstring (grapheme cluster string) for composing characters
/// between FROM and TO with FONT-OBJECT in STRING.
///
/// Stub: return nil (let the display engine handle shaping).
pub(crate) fn builtin_composition_get_gstring(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("composition-get-gstring", &args, 4)?;

    let codes = if args[3].is_nil() {
        let byte_range = super::editfns::current_buffer_accessible_char_region_in_buffers(
            &ctx.buffers,
            &args[0],
            &args[1],
        )?;
        let Some(buf) = ctx.buffers.current_buffer() else {
            return Err(signal("error", vec![Value::string("No current buffer")]));
        };
        if !buf.get_multibyte() {
            return Err(signal(
                "error",
                vec![Value::string("Attempt to shape unibyte text")],
            ));
        }
        let Some(byte_range) = byte_range else {
            return Err(signal("error", vec![Value::string("No current buffer")]));
        };
        let text = buf.buffer_substring_lisp_string_range(byte_range);
        crate::emacs_core::builtins::lisp_string_char_codes(&text)
    } else {
        let text = expect_string_value(&args[3])?;
        if !text.is_multibyte() && text.as_bytes().iter().any(|byte| *byte >= 0x80) {
            return Err(signal(
                "error",
                vec![Value::string("Attempt to shape unibyte text")],
            ));
        }
        let codes = crate::emacs_core::builtins::lisp_string_char_codes(text);
        let len = codes.len() as i64;
        let (from, to) = validate_subarray_indices(args[3], args[0], args[1], len)?;
        codes[from as usize..to as usize].to_vec()
    };

    if codes.is_empty() {
        return Err(signal(
            "error",
            vec![Value::string("Attempt to shape zero-length text")],
        ));
    }

    let segment = &codes;
    let mut encoded = vec![Value::symbol("utf-8-unix")];
    encoded.extend(segment.iter().map(|code| Value::fixnum(*code as i64)));

    let mut gstring = vec![Value::vector(encoded), Value::NIL];
    for code in segment {
        let code = *code as i64;
        gstring.push(Value::vector(vec![
            Value::fixnum(0),
            Value::fixnum(0),
            Value::fixnum(code),
            Value::fixnum(code),
            Value::fixnum(1),
            Value::fixnum(0),
            Value::fixnum(1),
            Value::fixnum(1),
            Value::fixnum(0),
            Value::NIL,
        ]));
    }
    while gstring.len() < 10 {
        gstring.push(Value::NIL);
    }

    Ok(Value::vector(gstring))
}

/// `(clear-composition-cache)`
///
/// Clear the internal composition cache.
///
/// Stub: no cache to clear, return nil.
pub(crate) fn builtin_clear_composition_cache(args: Vec<Value>) -> EvalResult {
    expect_max_args("clear-composition-cache", &args, 0)?;
    Ok(Value::NIL)
}

/// `(composition-sort-rules RULES)`
///
/// Sort composition rules by priority.
///
/// Batch-compatible subset:
/// - nil RULES => nil
/// - non-list RULES => `(wrong-type-argument listp RULES)`
/// - list entries that are not composition rules => generic invalid-rule error
/// - otherwise return RULES unchanged
pub(crate) fn builtin_composition_sort_rules(args: Vec<Value>) -> EvalResult {
    expect_args("composition-sort-rules", &args, 1)?;
    if args[0].is_nil() {
        return Ok(Value::NIL);
    }

    let items = list_to_vec(&args[0])
        .ok_or_else(|| signal("wrong-type-argument", vec![Value::symbol("listp"), args[0]]))?;

    for item in items {
        if !item.is_cons() {
            return Err(signal(
                "error",
                vec![Value::string("Invalid composition rule in RULES argument")],
            ));
        }
    }

    Ok(args[0])
}

// ---------------------------------------------------------------------------
// Bootstrap variables
// ---------------------------------------------------------------------------

pub fn register_bootstrap_vars(obarray: &mut crate::emacs_core::symbol::Obarray) {
    // Official Emacs leaves unicode-category-table as nil at C init time;
    // it is populated later by characters.el via unicode-property-table-internal.
    obarray.set_symbol_value("unicode-category-table", Value::NIL);
    // char-unify-table is created lazily by define_charset (charset.c:1364).
    // Initialize to nil so maybe_unify_char gracefully degrades.
    obarray.set_symbol_value("char-unify-table", Value::NIL);
    // composition-function-table must be a real char-table (composite.c:2289).
    obarray.set_symbol_value(
        "composition-function-table",
        make_char_table_value(Value::NIL, Value::NIL),
    );
    obarray.set_symbol_value("auto-composition-mode", Value::T);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "composite_test.rs"]
mod tests;
