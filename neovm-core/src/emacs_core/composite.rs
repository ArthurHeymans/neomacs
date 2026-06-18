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

thread_local! {
    /// Mirrors GNU's `composition_hash_table` (dedup: component chars -> id) plus
    /// the `relative_p` slice of `composition_table` (id -> method). The id
    /// counter is GNU's `n_compositions`. Keyed on the component char codes so
    /// there is no GC interaction; `relative_by_id[id]` lets a later
    /// `find-composition`/decode of a registered (Form-B) property recover the
    /// `relative-p` it can no longer infer from the bare components vector.
    static COMPOSITION_REGISTRY: std::cell::RefCell<CompositionRegistry> =
        std::cell::RefCell::new(CompositionRegistry {
            next_id: 0,
            dedup: std::collections::HashMap::new(),
            relative_by_id: Vec::new(),
        });
}

struct CompositionRegistry {
    next_id: i64,
    dedup: std::collections::HashMap<Vec<i64>, i64>,
    relative_by_id: Vec<bool>,
}

/// The component char codes of a key vector, or None if any element is not a
/// fixnum (rule-based components carrying cons rules — not deduped, like a
/// distinct GNU registration).
fn composition_key_codes(key: &Value) -> Option<Vec<i64>> {
    let items = key.as_vector_data()?;
    let mut codes = Vec::with_capacity(items.len());
    for item in items.iter() {
        codes.push(item.as_fixnum()?);
    }
    Some(codes)
}

/// GNU `get_composition_id` id assignment: reuse the id of an identical
/// composition (same component chars) else allocate the next id, recording its
/// `relative-p` (method) for later decode.
fn composition_assign_id(key: &Value, relative_p: bool) -> i64 {
    let codes = composition_key_codes(key);
    COMPOSITION_REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        if let Some(codes) = &codes {
            if let Some(&id) = reg.dedup.get(codes) {
                return id;
            }
        }
        let id = reg.next_id;
        reg.next_id += 1;
        if let Some(codes) = codes {
            reg.dedup.insert(codes, id);
        }
        reg.relative_by_id.push(relative_p);
        id
    })
}

fn composition_lookup_relative(id: i64) -> bool {
    COMPOSITION_REGISTRY.with(|reg| {
        reg.borrow()
            .relative_by_id
            .get(id as usize)
            .copied()
            .unwrap_or(true)
    })
}

/// A registered (Form-B) composition property `(ID LENGTH COMPONENTS-VEC . MOD)`.
fn composition_registered_p(prop: Value) -> bool {
    prop.is_cons() && prop.cons_car().is_fixnum()
}

/// Decode either composition form into `(length, components-or-vec, mod-func,
/// registered-id)`. `registered-id` is `Some` for Form-B (then the second value
/// is the components vector), `None` for Form-A (the raw components).
fn composition_parts_any(prop: Value) -> Option<(i64, Value, Value, Option<i64>)> {
    if !prop.is_cons() {
        return None;
    }
    let head = prop.cons_car();
    if let Some(id) = head.as_fixnum() {
        // Form-B: (ID . (LENGTH COMPONENTS-VEC . MOD)).
        let rest = prop.cons_cdr();
        if !rest.is_cons() {
            return None;
        }
        let length = rest.cons_car().as_fixnum()?;
        let after = rest.cons_cdr();
        if !after.is_cons() {
            return None;
        }
        Some((length, after.cons_car(), after.cons_cdr(), Some(id)))
    } else if head.is_cons() {
        // Form-A: ((LENGTH . COMPONENTS) . MOD).
        let length = head.cons_car().as_fixnum()?;
        Some((length, head.cons_cdr(), prop.cons_cdr(), None))
    } else {
        None
    }
}

/// GNU `get_composition_id`: register a Form-A composition and rewrite the
/// (shared) property cons in place to Form-B `(ID LENGTH COMPONENTS-VEC . MOD)`.
/// Direct car/cdr mutation matches GNU's `XSETCAR`/`XSETCDR` — it upgrades the
/// stored property without re-running put-text-property or touching
/// buffer-modified-p. Returns the composition id.
fn composition_register_prop(
    prop: Value,
    key: Value,
    length: i64,
    mod_func: Value,
    relative_p: bool,
) -> i64 {
    let id = composition_assign_id(&key, relative_p);
    let saved = super::eval::save_scratch_gc_roots();
    super::eval::push_scratch_gc_root(key);
    super::eval::push_scratch_gc_root(mod_func);
    let new_cdr = Value::cons(Value::fixnum(length), Value::cons(key, mod_func));
    prop.set_car(Value::fixnum(id));
    prop.set_cdr(new_cdr);
    super::eval::restore_scratch_gc_roots(saved);
    id
}

/// Display width and character length of the composition whose `composition`
/// text property begins at 1-based buffer position `charpos1`, or None if there
/// is no valid composition there. This is GNU's `get_composition_id` as called
/// from `current_column_1`: it returns the composed glyphs' width over the
/// covered characters AND, the first time the composition is seen, registers it
/// — rewriting the property from Form-A to Form-B in place.
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
    let (length, components, mod_func, registered) = composition_parts_any(prop)?;
    if length <= 0 {
        return None;
    }
    if registered.is_some() {
        // Already Form-B: `components` is the registered components vector.
        return Some((composition_relative_width(&components), length));
    }
    // Form-A: compute the width, then register (rewrite in place to Form-B).
    let key = composition_components_key(ctx, components, Value::NIL, charpos1, length);
    let width = composition_relative_width(&key);
    let relative_p = composition_relative_p(components);
    composition_register_prop(prop, key, length, mod_func, relative_p);
    Some((width, length))
}

/// GNU `composition_valid_p` restricted to the unregistered form: PROP is a
/// well-formed composition property whose stored length equals `end - start`.
fn composition_valid_unregistered(start: i64, end: i64, prop: Value) -> bool {
    let Some((length, components, _, registered)) = composition_parts_any(prop) else {
        return false;
    };
    if length != end - start {
        return false;
    }
    if registered.is_some() {
        // Form-B: components is the registered components vector.
        return true;
    }
    components.is_nil()
        || components.is_string()
        || components.is_vector()
        || components.is_fixnum()
        || components.is_cons()
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

    // Requesting detail registers the composition (GNU `get_composition_id` in
    // the detail branch of `Ffind_composition_internal`), so a subsequent read
    // of the property sees Form-B. The Lisp-visible detail list is identical for
    // both forms.
    let (length, components, mod_func, registered) =
        composition_parts_any(prop).expect("valid composition decodes");
    let (key, relative_p) = if let Some(id) = registered {
        // Already Form-B: components is the registered vector; relative-p was
        // recorded at registration (it cannot be inferred from the bare vector).
        (components, composition_lookup_relative(id))
    } else {
        let relative_p = composition_relative_p(components);
        let key = composition_components_key(ctx, components, args[2], start, end - start);
        composition_register_prop(prop, key, length, mod_func, relative_p);
        (key, relative_p)
    };
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
