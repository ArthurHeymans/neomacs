use crate::emacs_core::error::{expect_args, expect_min_args, expect_max_args, expect_args_range};
use super::*;
use crate::emacs_core::symbol::Obarray;

// ===========================================================================
// Keymap builtins
// ===========================================================================
use super::keymap::{
    KeyEvent, KeymapMarker, collect_minor_mode_map_entries_in_state,
    collect_minor_mode_maps_in_state, current_active_maps_for_position,
    expand_meta_prefix_char_events_in_obarray, get_keymap_in_obarray, get_keymap_in_runtime,
    is_list_keymap, key_event_to_emacs_event, list_keymap_accessible, list_keymap_copy,
    list_keymap_define_seq_in_obarray, list_keymap_define_seq_in_obarray_ex,
    list_keymap_inherits_from, list_keymap_parent, list_keymap_set_parent,
    lookup_key_in_keymaps_in_obarray_runtime, make_list_keymap, make_sparse_list_keymap,
    maybe_keymap_in_obarray, maybe_keymap_in_runtime,
};
use super::symbols::cache_event_symbol_value_properties_in_obarray;

fn map_keymap_binding_value(binding: Value) -> Value {
    if binding == Value::T {
        Value::NIL
    } else {
        binding
    }
}

/// Validate that a value is a keymap, returning it if so.
/// Accepts:
/// - Cons cells starting with 'keymap
/// - Symbols whose function definition is a keymap
pub(crate) fn expect_keymap_in_obarray(obarray: &Obarray, value: &Value) -> Result<Value, Flow> {
    get_keymap_in_obarray(obarray, value, true)
}

fn expect_keymap(eval: &mut super::eval::Context, value: &Value) -> EvalResult {
    get_keymap_in_runtime(eval, value, true, true)
}

#[allow(clippy::too_many_arguments)] // mirrors the Lisp helper's positional argument contract
fn call_help_describe_map_tree(
    eval: &mut super::eval::Context,
    startmap: Value,
    partial: Value,
    shadow: Value,
    prefix: Value,
    title: Value,
    nomenu: Value,
    transl: Value,
    always_title: Value,
    mention_shadow: Value,
    buffer: Value,
) -> Result<Value, Flow> {
    eval.apply(
        Value::symbol("help--describe-map-tree"),
        vec![
            startmap,
            partial,
            shadow,
            prefix,
            title,
            nomenu,
            transl,
            always_title,
            mention_shadow,
            buffer,
        ],
    )
}

/// Parse a key description from a Value, returning emacs event values.
///
/// For vectors, integer and symbol elements are used directly as emacs event
/// codes (preserving all modifier bits including Alt and Hyper).  For strings,
/// each character is treated as a raw key event.
pub(crate) fn expect_key_events(value: &Value) -> Result<Vec<Value>, Flow> {
    match value.kind() {
        // Vectors: use elements directly — integers are already emacs event codes,
        // symbols are already event symbols.
        ValueKind::Veclike(VecLikeType::Vector) => {
            let items = value.as_vector_data().unwrap().clone();
            let mut events = Vec::with_capacity(items.len());
            for item in &items {
                match item.kind() {
                    // Integer event codes (character + modifier bits)
                    ValueKind::Fixnum(_) => events.push(*item),
                    // Symbol events (function keys, remap, etc.)
                    ValueKind::Symbol(_) => events.push(*item),
                    // nil and t can appear as events in vectors
                    ValueKind::Nil => events.push(Value::symbol("nil")),
                    ValueKind::T => events.push(Value::symbol("t")),
                    // GNU only treats a cons vector element as a Lucid-style
                    // event type list when every element is an integer or a
                    // symbol.  Real mouse events are lists like
                    // (mouse-movement POSITION), where POSITION is itself a
                    // list; those remain parameterized events and key lookup
                    // matches on their car.
                    ValueKind::Cons => {
                        if let Some(event) = convert_lucid_event_type_list(item) {
                            // GNU `Fdefine_key` converts a Lucid event type list
                            // such as `(shift tab)` via `Fevent_convert_list`
                            // (src/keymap.c:1156-1157, 1264-1265).  That routine
                            // keeps a multi-character symbol base (e.g. `tab`) as
                            // a SYMBOL and applies modifiers to produce `S-tab`,
                            // whereas the kbd-designator path coerces `tab` to the
                            // character 9 and yields the integer 33554441.  Use
                            // the same conversion as `event-convert-list' so the
                            // stored key matches GNU exactly.
                            events.push(event);
                        } else {
                            events.push(*item);
                        }
                    }
                    _other => {
                        return Err(signal(
                            LispCondition::WrongTypeArgument,
                            vec![Value::symbol("arrayp"), *value],
                        ));
                    }
                }
            }
            Ok(events)
        }
        // Strings and other forms: go through KeyEvent roundtrip
        _ => {
            let key_events = expect_key_description(value)?;
            Ok(key_events.iter().map(key_event_to_emacs_event).collect())
        }
    }
}

fn cache_key_event_symbol_properties(
    eval: &mut super::eval::Context,
    events: &[Value],
) -> EvalResult {
    for event in events {
        cache_event_symbol_value_properties_in_obarray(eval.obarray_mut(), *event)?;
    }
    Ok(Value::NIL)
}

fn lucid_event_type_list_p(value: &Value) -> bool {
    if !value.is_cons() {
        return false;
    }
    if let Some("help-echo" | "vertical-line" | "mode-line" | "tab-line" | "header-line") =
        value.cons_car().as_symbol_name()
    {
        return false;
    }

    let mut cursor = *value;
    while cursor.is_cons() {
        let elt = cursor.cons_car();
        if !matches!(
            elt.kind(),
            ValueKind::Fixnum(_) | ValueKind::Symbol(_) | ValueKind::Nil | ValueKind::T
        ) {
            return false;
        }
        cursor = cursor.cons_cdr();
    }
    cursor.is_nil()
}

fn convert_lucid_event_type_list(value: &Value) -> Option<Value> {
    if !lucid_event_type_list_p(value) {
        return None;
    }

    let mut items = Vec::new();
    let mut cursor = *value;
    while cursor.is_cons() {
        items.push(cursor.cons_car());
        cursor = cursor.cons_cdr();
    }
    crate::emacs_core::keyboard::pure::convert_lucid_event_list(&items)
}

/// GNU `Fdefine_key` treats a vector whose first element is a cons as an
/// XEmacs-style keyboard macro and canonicalizes each Lucid event list in it.
/// Keep that compatibility conversion at the `define-key` boundary so stored
/// definitions, lookup results, and command-loop execution share one shape.
fn normalize_keyboard_macro_definition(definition: Value) -> Value {
    let Some(items) = definition.as_vector_data() else {
        return definition;
    };
    if items.first().is_none_or(|item| !item.is_cons()) {
        return definition;
    }

    Value::vector(
        items
            .iter()
            .map(|item| convert_lucid_event_type_list(item).unwrap_or(*item))
            .collect(),
    )
}

/// Parse a key description from a Value (must be a string or vector).
fn expect_key_description(value: &Value) -> Result<Vec<KeyEvent>, Flow> {
    match super::kbd::key_events_from_designator(value) {
        Ok(events) => Ok(events),
        Err(super::kbd::KeyDesignatorError::WrongType(other)) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("arrayp"), other],
        )),
        Err(super::kbd::KeyDesignatorError::Parse(msg)) => {
            Err(signal("error", vec![Value::string(msg)]))
        }
    }
}

/// `(accessible-keymaps KEYMAP &optional PREFIXES)` -> list of accessible keymaps.
pub(super) fn builtin_accessible_keymaps(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_accessible_keymaps_impl(eval.obarray(), &args)
}

pub(crate) fn builtin_accessible_keymaps_impl(obarray: &Obarray, args: &[Value]) -> EvalResult {
    use crate::emacs_core::value::{ValueKind, VecLikeType};

    expect_min_args("accessible-keymaps", args, 1)?;
    expect_max_args("accessible-keymaps", args, 2)?;
    let keymap = expect_keymap_in_obarray(obarray, &args[0])?;

    // Collect all accessible keymaps
    let mut all_out = Vec::new();
    list_keymap_accessible(&keymap, &mut all_out);

    // If prefix argument is provided, filter results
    if let Some(prefix_arg) = args.get(1)
        && !prefix_arg.is_nil()
    {
        // Must be a sequence (string or vector), not a list or non-sequence
        let prefix_events: Vec<Value> = match prefix_arg.kind() {
            ValueKind::String => {
                // String prefix — convert to events
                expect_key_events(prefix_arg)?
            }
            ValueKind::Veclike(VecLikeType::Vector) => {
                // Vector prefix — elements are events directly
                prefix_arg.as_vector_data().unwrap().clone()
            }
            ValueKind::Cons => {
                // Lists are not valid as key sequences for prefix
                return Err(super::error::signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("arrayp"), *prefix_arg],
                ));
            }
            _ => {
                return Err(super::error::signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("sequencep"), *prefix_arg],
                ));
            }
        };

        // Filter: only keep entries whose prefix starts with the given prefix
        let filtered: Vec<Value> = all_out
            .into_iter()
            .filter(|entry| {
                if entry.is_cons() {
                    let pair_car = entry.cons_car();
                    let _pair_cdr = entry.cons_cdr();
                    // pair_car is the prefix vector
                    if pair_car.is_vector() {
                        let entry_prefix = pair_car.as_vector_data().unwrap().clone();
                        if entry_prefix.len() >= prefix_events.len() {
                            return entry_prefix[..prefix_events.len()] == prefix_events[..];
                        }
                    }
                }
                false
            })
            .collect();

        if filtered.is_empty() {
            return Ok(Value::NIL);
        }
        return Ok(Value::list(filtered));
    }

    Ok(Value::list(all_out))
}

/// (make-keymap) -> keymap
pub(super) fn builtin_make_keymap(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_make_keymap_pure(&args)
}

pub(crate) fn builtin_make_keymap_pure(args: &[Value]) -> EvalResult {
    expect_max_args("make-keymap", args, 1)?;
    let keymap = make_list_keymap();
    if let Some(prompt) = args.first()
        && !prompt.is_nil()
    {
        let tail = keymap.cons_cdr();
        if tail.is_cons() {
            tail.set_cdr(Value::cons(*prompt, Value::NIL));
        }
    }
    Ok(keymap)
}

/// (make-sparse-keymap &optional NAME) -> keymap
pub(super) fn builtin_make_sparse_keymap(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_max_args("make-sparse-keymap", &args, 1)?;
    // GNU keymap.c: (make-sparse-keymap "prompt") → (keymap "prompt")
    if let Some(prompt) = args.first()
        && prompt.is_string()
    {
        return Ok(Value::cons(
            KeymapMarker::Keymap.symbol_value(),
            Value::cons(*prompt, Value::NIL),
        ));
    }
    Ok(make_sparse_list_keymap())
}

/// `(copy-keymap KEYMAP)` -> keymap copy.
pub(super) fn builtin_copy_keymap(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_copy_keymap_impl(eval.obarray(), &args)
}

pub(crate) fn builtin_copy_keymap_impl(obarray: &Obarray, args: &[Value]) -> EvalResult {
    expect_args("copy-keymap", args, 1)?;
    let keymap = expect_keymap_in_obarray(obarray, &args[0])?;
    Ok(list_keymap_copy(&keymap))
}

/// (define-key KEYMAP KEY DEF &optional REMOVE) -> DEF
pub(super) fn builtin_define_key(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("define-key", &args, 3)?;
    expect_max_args("define-key", &args, 4)?;
    let keymap = expect_keymap(eval, &args[0])?;
    let mut events = expect_key_events(&args[1])?;
    cache_key_event_symbol_properties(eval, &events)?;
    let def = normalize_keyboard_macro_definition(args[2]);
    let remove = args.get(3).is_some_and(|v| v.is_truthy());
    // Expand meta-prefixed events to ESC + base, matching GNU Emacs
    // Fdefine_key's metized handling.
    if let Some(expanded) = expand_meta_prefix_char_events_in_obarray(eval.obarray(), &events) {
        events = expanded;
    }
    if let Err(msg) =
        list_keymap_define_seq_in_obarray_ex(eval.obarray(), keymap, &events, def, remove)
    {
        return Err(signal("error", vec![Value::string(msg)]));
    }
    Ok(def)
}

/// (lookup-key KEYMAP KEY &optional ACCEPT-DEFAULTS) -> binding or nil
pub(super) fn builtin_lookup_key(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("lookup-key", &args, 2)?;
    expect_max_args("lookup-key", &args, 3)?;
    let t_ok = args.get(2).is_some_and(|v| v.is_truthy());
    let events = expect_key_events(&args[1])?;
    cache_key_event_symbol_properties(eval, &events)?;
    let keymaps = resolve_lookup_keymaps_in_runtime(eval, &args[0])?;

    if events.is_empty() {
        return Ok(keymaps.first().copied().unwrap_or(Value::NIL));
    }

    // The resolved keymaps (possibly read from function cells, not the
    // rooted KEYMAP argument) and heap event conses live only in Rust Vecs
    // across the lookups, which can run Lisp (keymap autoloads, translation
    // functions); thread them onto one rooted holder for the span.
    let mut holder = Value::NIL;
    for value in keymaps.iter().chain(events.iter()).rev() {
        if value.is_heap_object() {
            holder = Value::cons(*value, holder);
        }
    }
    let root_scope = eval.save_specpdl_roots();
    eval.push_specpdl_root(holder);
    let result = lookup_key_with_menu_compat_runtime(eval, &keymaps, &events, t_ok);
    eval.restore_specpdl_roots(root_scope);
    result
}

fn lookup_key_with_menu_compat_runtime(
    eval: &mut super::eval::Context,
    keymaps: &[Value],
    events: &[Value],
    t_ok: bool,
) -> EvalResult {
    let found = lookup_key_in_keymaps_in_obarray_runtime(eval, keymaps, events, t_ok)?;
    if is_defined_lookup_result(&found) || !is_menu_bar_key(events) {
        return Ok(found);
    }

    let lower_events: Vec<Value> = events
        .iter()
        .map(|event| {
            event
                .as_symbol_name()
                .map(|name| Value::symbol(name.to_lowercase()))
                .unwrap_or(*event)
        })
        .collect();
    let lower_found = lookup_key_in_keymaps_in_obarray_runtime(eval, keymaps, &lower_events, t_ok)?;
    if is_defined_lookup_result(&lower_found) {
        return Ok(lower_found);
    }

    let dash_events: Vec<Value> = lower_events
        .iter()
        .map(|event| {
            event
                .as_symbol_name()
                .filter(|name| name.contains(' '))
                .map(|name| Value::symbol(name.replace(' ', "-")))
                .unwrap_or(*event)
        })
        .collect();
    let dash_found = lookup_key_in_keymaps_in_obarray_runtime(eval, keymaps, &dash_events, t_ok)?;
    if is_defined_lookup_result(&dash_found) {
        return Ok(dash_found);
    }

    Ok(found)
}

fn is_defined_lookup_result(value: &Value) -> bool {
    !value.is_nil() && !value.is_fixnum()
}

fn is_menu_bar_key(events: &[Value]) -> bool {
    events
        .first()
        .and_then(|event| event.as_symbol_name())
        .is_some_and(|name| name == "menu-bar")
}

fn resolve_lookup_keymaps_in_runtime(
    eval: &mut super::eval::Context,
    value: &Value,
) -> Result<Vec<Value>, Flow> {
    if is_list_keymap(value) {
        return Ok(vec![*value]);
    }
    if value.is_nil() {
        return Ok(vec![Value::NIL]);
    }
    if value.is_cons()
        && is_list_keymap(&maybe_keymap_in_runtime(eval, &value.cons_car(), true)?)
        && let Some(items) = list_to_vec(value)
    {
        if items.is_empty() {
            return Ok(vec![Value::NIL]);
        }
        let mut resolved = Vec::with_capacity(items.len());
        for item in &items {
            if item.is_nil() {
                resolved.push(Value::NIL);
                continue;
            }
            let keymap = maybe_keymap_in_runtime(eval, item, true)?;
            if keymap.is_nil() {
                resolved.clear();
                break;
            }
            resolved.push(keymap);
        }
        if !resolved.is_empty() {
            return Ok(resolved);
        }
    }
    if value.is_cons() {
        return Ok(vec![*value]);
    }

    Ok(vec![get_keymap_in_runtime(eval, value, true, true)?])
}

/// (global-set-key KEY COMMAND)
pub(super) fn builtin_global_set_key(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("global-set-key", &args, 2)?;
    let selected_global_map = eval.current_global_map();
    let global = get_keymap_in_runtime(eval, &selected_global_map, true, true)?;
    let events = expect_key_events(&args[0])?;
    cache_key_event_symbol_properties(eval, &events)?;
    let def = args[1];
    if let Err(msg) = list_keymap_define_seq_in_obarray(eval.obarray(), global, &events, def) {
        return Err(signal("error", vec![Value::string(msg)]));
    }
    Ok(def)
}

/// (local-set-key KEY COMMAND)
pub(super) fn builtin_local_set_key(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("local-set-key", &args, 2)?;
    let local = if eval.buffers.current_local_map().is_nil() {
        let km = make_sparse_list_keymap();
        let _ = eval.buffers.set_current_local_map(km);
        km
    } else {
        eval.buffers.current_local_map()
    };
    let events = expect_key_events(&args[0])?;
    cache_key_event_symbol_properties(eval, &events)?;
    let def = args[1];
    if let Err(msg) = list_keymap_define_seq_in_obarray(eval.obarray(), local, &events, def) {
        return Err(signal("error", vec![Value::string(msg)]));
    }
    Ok(def)
}

/// (use-local-map KEYMAP)
pub(super) fn builtin_use_local_map(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("use-local-map", &args, 1)?;
    let keymap = if args[0].is_nil() {
        Value::NIL
    } else {
        expect_keymap(eval, &args[0])?
    };
    let _ = eval.buffers.set_current_local_map(keymap);
    Ok(Value::NIL)
}

/// (use-global-map KEYMAP)
pub(super) fn builtin_use_global_map(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("use-global-map", &args, 1)?;
    let keymap = get_keymap_in_runtime(eval, &args[0], true, true)?;
    eval.select_global_map(keymap);
    Ok(Value::NIL)
}

/// (current-local-map) -> keymap or nil
pub(super) fn builtin_current_local_map(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_current_local_map_impl(eval.buffers.current_local_map(), &args)
}

pub(crate) fn builtin_current_local_map_impl(
    current_local_map: Value,
    args: &[Value],
) -> EvalResult {
    expect_args("current-local-map", args, 0)?;
    Ok(current_local_map)
}

/// (current-global-map) -> keymap
pub(super) fn builtin_current_global_map(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("current-global-map", &args, 0)?;
    Ok(eval.current_global_map())
}

pub(super) fn builtin_describe_buffer_bindings(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("describe-buffer-bindings", &args, 1, 3)?;
    if !args[0].is_buffer() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("bufferp"), args[0]],
        ));
    }
    if let Some(prefixes) = args.get(1)
        && !prefixes.is_nil()
        && !(prefixes.is_cons()
            || prefixes.is_vector()
            || prefixes.is_string()
            || prefixes.is_nil())
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("sequencep"), *prefixes],
        ));
    }

    let buffer = args[0];
    let prefix = args.get(1).copied().unwrap_or(Value::NIL);
    let nomenu = if args.get(2).is_some_and(|v| !v.is_nil()) {
        Value::NIL
    } else {
        Value::T
    };

    let Some(buffer_id) = buffer.as_buffer_id() else {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("bufferp"), buffer],
        ));
    };
    let Some(buf) = eval.buffers.get(buffer_id) else {
        return Err(signal(
            "error",
            vec![Value::string("Selecting deleted buffer")],
        ));
    };

    let local_map = buf.local_map();
    let major_mode_name = buf
        .get_buffer_local("major-mode")
        .and_then(|value| value.as_symbol_name())
        .unwrap_or("fundamental-mode");
    let minor_maps =
        collect_minor_mode_map_entries_in_state(&eval.obarray, &[], &eval.buffers, Some(buffer_id));

    // Every keymap and shadow cons below is held in a Rust local across
    // help--describe-map-tree (arbitrary Lisp, can GC); root them, or a
    // collection mid-describe frees them and the later passes walk freed
    // keymaps. GNU's C locals survive via conservative stack scanning. The
    // scope unwinds with the specpdl on nonlocal exit.
    let root_scope = eval.save_specpdl_roots();
    eval.push_specpdl_root(local_map);
    for (_, keymap) in &minor_maps {
        eval.push_specpdl_root(*keymap);
    }

    let mut shadow = Value::NIL;

    if let Some(key_translation_map) = eval.obarray.symbol_value("key-translation-map").copied() {
        call_help_describe_map_tree(
            eval,
            key_translation_map,
            Value::NIL,
            shadow,
            prefix,
            Value::string("Key translations"),
            nomenu,
            Value::T,
            Value::NIL,
            Value::NIL,
            buffer,
        )?;
        shadow = Value::cons(key_translation_map, shadow);
        eval.push_specpdl_root(shadow);
    }

    for (mode, keymap) in minor_maps {
        let title = Value::string(format!(
            "\u{c}\n`{}' Minor Mode Bindings",
            resolve_sym(mode)
        ));
        call_help_describe_map_tree(
            eval,
            keymap,
            Value::T,
            shadow,
            prefix,
            title,
            nomenu,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            buffer,
        )?;
        shadow = Value::cons(keymap, shadow);
        eval.push_specpdl_root(shadow);
    }

    if !local_map.is_nil() {
        let title = Value::string(format!("\u{c}\n`{major_mode_name}' Major Mode Bindings"));
        call_help_describe_map_tree(
            eval,
            local_map,
            Value::T,
            shadow,
            prefix,
            title,
            nomenu,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            buffer,
        )?;
        shadow = Value::cons(local_map, shadow);
        eval.push_specpdl_root(shadow);
    }

    let global_map = eval.current_global_map();
    call_help_describe_map_tree(
        eval,
        global_map,
        Value::T,
        shadow,
        prefix,
        Value::string("\u{c}\nGlobal Bindings"),
        nomenu,
        Value::NIL,
        Value::T,
        Value::NIL,
        buffer,
    )?;

    eval.restore_specpdl_roots(root_scope);
    Ok(Value::NIL)
}

/// `(current-active-maps &optional OLP POSITION)` -> list of active keymaps.
///
/// Returns list of currently active keymaps in priority order.
/// GNU Emacs order: minor-mode maps > local-map > global-map.
pub(super) fn builtin_current_active_maps(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_current_active_maps_impl(eval, &args)
}

pub(crate) fn builtin_current_active_maps_impl(
    ctx: &mut crate::emacs_core::eval::Context,
    args: &[Value],
) -> EvalResult {
    expect_max_args("current-active-maps", args, 2)?;
    let obey_overriding_local_maps = args.first().is_some_and(|v| v.is_truthy());
    let maps = current_active_maps_for_position(ctx, obey_overriding_local_maps, args.get(1))?;
    Ok(Value::list(maps))
}

/// `(current-minor-mode-maps)` -> list of active minor mode keymaps.
pub(super) fn builtin_current_minor_mode_maps(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_current_minor_mode_maps_impl(eval, &args)
}

pub(crate) fn builtin_current_minor_mode_maps_impl(
    ctx: &crate::emacs_core::eval::Context,
    args: &[Value],
) -> EvalResult {
    expect_args("current-minor-mode-maps", args, 0)?;
    let maps = collect_minor_mode_maps_in_state(
        &ctx.obarray,
        &[],
        &ctx.buffers,
        ctx.buffers.current_buffer_id(),
    );
    if maps.is_empty() {
        Ok(Value::NIL)
    } else {
        Ok(Value::list(maps))
    }
}

pub(crate) struct KeymapIterationPlan {
    pub(crate) bindings: Vec<(Value, Value)>,
    pub(crate) parent: Value,
}

pub(crate) fn plan_keymap_iteration(keymap: Value) -> KeymapIterationPlan {
    let mut bindings = Vec::new();
    let mut parent = Value::NIL;
    let mut cursor = if is_list_keymap(&keymap) {
        keymap.cons_cdr()
    } else {
        keymap
    };
    let mut steps = 0usize;

    while cursor.is_cons() {
        steps += 1;
        if steps > 100_000 {
            break;
        }

        if is_list_keymap(&cursor) {
            parent = cursor;
            break;
        }

        let entry = cursor.cons_car();
        if is_list_keymap(&entry) {
            parent = entry;
            break;
        }

        if crate::emacs_core::chartable::is_char_table(&entry) {
            let _ = crate::emacs_core::chartable::for_each_char_table_mapping(
                &entry,
                |event, binding| {
                    bindings.push((event, map_keymap_binding_value(binding)));
                    Ok(())
                },
            );
        } else {
            match entry.kind() {
                ValueKind::Cons => {
                    let pair_car = entry.cons_car();
                    let pair_cdr = entry.cons_cdr();
                    bindings.push((pair_car, map_keymap_binding_value(pair_cdr)));
                }
                ValueKind::Veclike(VecLikeType::Vector) => {
                    let items = entry.as_vector_data().unwrap().clone();
                    for (idx, binding) in items.iter().enumerate() {
                        bindings.push((
                            Value::fixnum(idx as i64),
                            map_keymap_binding_value(*binding),
                        ));
                    }
                }
                _ => {}
            }
        }

        cursor = cursor.cons_cdr();
    }

    KeymapIterationPlan { bindings, parent }
}

pub(crate) fn execute_keymap_iteration_callbacks(
    eval: &mut super::eval::Context,
    function: Value,
    bindings: &[(Value, Value)],
) -> Result<(), Flow> {
    for (event, binding) in bindings {
        eval.apply(function, vec![*event, *binding])?;
    }
    Ok(())
}

/// `(map-keymap FUNCTION KEYMAP &optional SORT-FIRST)` -> nil.
///
/// Call FUNCTION for each binding in KEYMAP and its parents.
/// FUNCTION receives two arguments: the event and the binding definition.
pub(super) fn builtin_map_keymap(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("map-keymap", &args, 2)?;
    expect_max_args("map-keymap", &args, 3)?;
    let function = args[0];
    let mut keymap = expect_keymap(eval, &args[1])?;

    // Traverse this keymap and all parents.
    loop {
        keymap = map_keymap_internal_impl(eval, function, keymap)?;
        if keymap.is_nil() {
            break;
        }
        // keymap is the parent; continue if it's a valid keymap.
        if !is_list_keymap(&keymap) {
            break;
        }
    }
    Ok(Value::NIL)
}

/// `(map-keymap-internal FUNCTION KEYMAP)` -> parent keymap or nil.
///
/// Call FUNCTION for each binding in KEYMAP (not its parents).
/// Returns the parent keymap if it has one.
pub(super) fn builtin_map_keymap_internal(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("map-keymap-internal", &args, 2)?;
    let function = args[0];
    let keymap = expect_keymap(eval, &args[1])?;
    map_keymap_internal_impl(eval, function, keymap)
}

/// Core implementation: iterate over one level of keymap entries,
/// calling `function(event, binding)` for each. Returns the parent
/// keymap (or nil if none).
fn map_keymap_internal_impl(
    eval: &mut super::eval::Context,
    function: Value,
    keymap: Value,
) -> EvalResult {
    let plan = plan_keymap_iteration(keymap);
    // The planned (event, binding) pairs and the parent keymap live in a
    // Rust Vec while FUNCTION runs per entry — arbitrary Lisp that can GC.
    // Unrooted, the first callback's collection frees the remaining
    // entries and the iteration walks freed objects. GNU's map_keymap
    // keeps everything on the conservatively-scanned C stack (keymap.c).
    // Keep every planned value alive under a SINGLE root by threading the
    // pairs onto a heap list — the GC marks the list transitively as part
    // of the ordinary heap walk. (Per-entry specpdl roots would add
    // O(bindings) root-seed work to EVERY collection, which exact-GC
    // stress mode turns into minutes.) No safe point can run inside the
    // build loop, so the partially-built list needs no interim rooting.
    let mut entry_holder = plan.parent;
    for (event, binding) in &plan.bindings {
        entry_holder = Value::cons(Value::cons(*event, *binding), entry_holder);
    }
    let root_scope = eval.save_specpdl_roots();
    eval.push_specpdl_root(entry_holder);
    let result = execute_keymap_iteration_callbacks(eval, function, &plan.bindings);
    eval.restore_specpdl_roots(root_scope);
    result?;
    Ok(plan.parent)
}

/// (keymap-parent KEYMAP) -> keymap or nil
pub(super) fn builtin_keymap_parent(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("keymap-parent", &args, 1)?;
    let keymap = get_keymap_in_runtime(eval, &args[0], true, true)?;
    Ok(list_keymap_parent(&keymap))
}

/// (set-keymap-parent KEYMAP PARENT) -> PARENT
pub(super) fn builtin_set_keymap_parent(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-keymap-parent", &args, 2)?;
    let keymap = get_keymap_in_runtime(eval, &args[0], true, true)?;
    let parent = if args[1].is_nil() {
        Value::NIL
    } else {
        get_keymap_in_runtime(eval, &args[1], true, false)?
    };
    if !parent.is_nil() && list_keymap_inherits_from(&parent, &keymap) {
        return Err(signal(
            "error",
            vec![Value::string("Cyclic keymap inheritance")],
        ));
    }
    list_keymap_set_parent(keymap, parent);
    Ok(parent)
}

/// (keymapp OBJ) -> t or nil
pub(super) fn builtin_keymapp(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_keymapp_impl(eval.obarray(), &args)
}

pub(crate) fn builtin_keymapp_impl(obarray: &Obarray, args: &[Value]) -> EvalResult {
    expect_args("keymapp", args, 1)?;
    Ok(maybe_keymap_in_obarray(obarray, &args[0])
        .map(|_| Value::T)
        .unwrap_or(Value::NIL))
}

/// `(event-convert-list EVENT-DESC)` -> event object or nil
pub(crate) fn builtin_event_convert_list(args: Vec<Value>) -> EvalResult {
    expect_args("event-convert-list", &args, 1)?;
    let Some(items) = list_to_vec(&args[0]) else {
        return Ok(Value::NIL);
    };
    if items.is_empty() {
        return Ok(Value::NIL);
    }
    convert_lucid_event_list(&items)
        .ok_or_else(|| signal("error", vec![Value::string("Invalid event description")]))
}

/// `(text-char-description CHARACTER)` -> printable text description.
pub(super) fn builtin_text_char_description(args: Vec<Value>) -> EvalResult {
    expect_args("text-char-description", &args, 1)?;
    let code = match args[0].kind() {
        ValueKind::Fixnum(n) if (0..=KEY_CHAR_CODE_MASK).contains(&n) => n,
        _ => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("characterp"), args[0]],
            ));
        }
    };
    if (code & !KEY_CHAR_CODE_MASK) != 0 {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("characterp"), args[0]],
        ));
    }

    let rendered = match code {
        0 => "^@".to_string(),
        1..=26 => format!(
            "^{}",
            char::from_u32((code as u32) + 64).expect("control-letter rendering must be ASCII")
        ),
        27 => "^[".to_string(),
        28 => "^\\\\".to_string(),
        29 => "^]".to_string(),
        30 => "^^".to_string(),
        31 => "^_".to_string(),
        127 => "^?".to_string(),
        _ => match char::from_u32(code as u32) {
            Some(ch) => ch.to_string(),
            None => {
                if let Some(encoded) = {
                    use crate::emacs_core::emacs_char;
                    let c = code as u32;
                    if c > emacs_char::MAX_UNICODE_CHAR && c <= emacs_char::MAX_CHAR {
                        let mut buf = [0u8; emacs_char::MAX_MULTIBYTE_LENGTH];
                        let len = emacs_char::char_string(c, &mut buf);
                        Some(emacs_char::to_utf8_lossy(&buf[..len]))
                    } else {
                        None
                    }
                } {
                    encoded
                } else {
                    return Err(signal(
                        LispCondition::WrongTypeArgument,
                        vec![Value::symbol("characterp"), args[0]],
                    ));
                }
            }
        },
    };
    Ok(Value::string(rendered))
}

/// `(single-key-description KEY &optional NO-ANGLES)` -> string
pub(super) fn builtin_single_key_description(args: Vec<Value>) -> EvalResult {
    expect_args_range("single-key-description", &args, 1, 2)?;
    let no_angles = args.get(1).is_some_and(|v| v.is_truthy());
    Ok(Value::string(describe_single_key_value(
        &args[0], no_angles,
    )?))
}

/// `(key-description KEYS &optional PREFIX)` -> string
pub(crate) fn builtin_key_description(args: Vec<Value>) -> EvalResult {
    expect_args_range("key-description", &args, 1, 2)?;
    let mut events = if let Some(prefix) = args.get(1) {
        key_sequence_values(prefix)?
    } else {
        vec![]
    };
    events.extend(key_sequence_values(&args[0])?);

    // Mirror GNU `Fkey_description`: a lone `meta_prefix_char` (ESC, 27) folds
    // the meta bit onto the FOLLOWING event, so e.g. [27 97] -> "M-a".  When the
    // following event cannot absorb the meta bit (a non-fixnum, another ESC, or
    // an already-meta key), the ESC is rendered literally instead.
    const META_PREFIX_CHAR: i64 = 27;
    let mut rendered: Vec<String> = Vec::with_capacity(events.len());
    let mut add_meta = false;
    for event in &events {
        let event_fixnum = event.as_fixnum();
        if add_meta {
            let absorbs_meta = match event_fixnum {
                Some(code) if code != META_PREFIX_CHAR && (code & KEY_CHAR_META) == 0 => Some(code),
                _ => None,
            };
            match absorbs_meta {
                Some(code) => {
                    rendered.push(describe_single_key_value(
                        &Value::fixnum(code | KEY_CHAR_META),
                        false,
                    )?);
                    add_meta = false;
                    continue;
                }
                None => {
                    rendered.push(describe_single_key_value(
                        &Value::fixnum(META_PREFIX_CHAR),
                        false,
                    )?);
                    if event_fixnum == Some(META_PREFIX_CHAR) {
                        // Leave `add_meta` set: the next event still folds.
                        continue;
                    }
                    add_meta = false;
                }
            }
        } else if event_fixnum == Some(META_PREFIX_CHAR) {
            add_meta = true;
            continue;
        }
        rendered.push(describe_single_key_value(event, false)?);
    }
    if add_meta {
        rendered.push(describe_single_key_value(
            &Value::fixnum(META_PREFIX_CHAR),
            false,
        )?);
    }
    Ok(Value::string(rendered.join(" ")))
}

/// `(recent-keys &optional INCLUDE-CMDS)` -> vector of recent input events.
pub(crate) fn builtin_recent_keys(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_recent_keys_impl(eval, args)
}

pub(crate) fn builtin_recent_keys_impl(
    ctx: &crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_max_args("recent-keys", &args, 1)?;
    let include_commands = args.first().is_some_and(|arg| arg.is_truthy());
    let events = ctx
        .recent_input_events()
        .iter()
        .copied()
        .filter(|event| include_commands || !(event.is_cons() && event.cons_car().is_nil()))
        .collect::<Vec<_>>();
    Ok(Value::vector(events))
}

#[cfg(test)]
#[path = "keymaps_test.rs"]
mod tests;
