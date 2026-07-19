//! Bootstrap-facing subset of GNU Emacs's `xfaces.c`.
//!
//! Face-related builtins are still mostly implemented in `face.rs` and
//! `font.rs`, but GNU startup also relies on a small set of C-level
//! variables from `xfaces.c` being bound before Lisp runs. Keep those
//! defaults here so Rust startup matches the same ownership boundary.

use crate::emacs_core::error::EvalResult;
use crate::emacs_core::intern::resolve_sym;
use crate::emacs_core::symbol::Obarray;
use crate::emacs_core::value::{HashKey, HashTableTest, Value, ValueKind, list_to_vec};
use crate::face::{LFACE_VECTOR_SIZE, LFaceAttr};

/// Register bootstrap variables owned by the face subsystem.
pub fn register_bootstrap_vars(obarray: &mut Obarray) {
    obarray.set_symbol_value("face-filters-always-match", Value::NIL);
    obarray.set_symbol_value(
        "face--new-frame-defaults",
        bootstrap_face_new_frame_defaults_table(),
    );
    stamp_face_id_properties(obarray);
    obarray.set_symbol_value("face-default-stipple", Value::string("gray3"));
    obarray.set_symbol_value("tty-defined-color-alist", Value::NIL);
    obarray.set_symbol_value("scalable-fonts-allowed", Value::NIL);
    obarray.set_symbol_value("face-ignored-fonts", Value::NIL);
    obarray.set_symbol_value("face-remapping-alist", Value::NIL);
    obarray.set_symbol_value("face-font-rescale-alist", Value::NIL);
    obarray.set_symbol_value("face-near-same-color-threshold", Value::fixnum(30_000));
    obarray.set_symbol_value("face-font-lax-matched-attributes", Value::T);
}

/// Backfill xfaces-owned bootstrap variables after loading a dump or partial
/// source bootstrap. GNU owns these in xfaces.c, so load/bootstrap glue should
/// delegate here instead of duplicating the values itself.
pub(crate) fn ensure_startup_compat_variables(eval: &mut crate::emacs_core::eval::Context) {
    match eval
        .obarray()
        .symbol_value("face--new-frame-defaults")
        .copied()
    {
        Some(table) if table.is_hash_table() => seed_face_new_frame_defaults_table(table),
        _ => eval.set_variable(
            "face--new-frame-defaults",
            bootstrap_face_new_frame_defaults_table(),
        ),
    }
    // Establish the `face` id property alongside the registry seed above, now
    // that every defface has run and all faces are known. This is the startup
    // sync point that fixes `face-id` for bootstrap faces (see
    // stamp_face_id_properties).
    stamp_face_id_properties(eval.obarray_mut());

    let defaults = [
        ("face-filters-always-match", Value::NIL),
        ("face-default-stipple", Value::string("gray3")),
        ("scalable-fonts-allowed", Value::NIL),
        ("face-ignored-fonts", Value::NIL),
        ("face-remapping-alist", Value::NIL),
        ("face-font-rescale-alist", Value::NIL),
        ("face-near-same-color-threshold", Value::fixnum(30_000)),
        ("face-font-lax-matched-attributes", Value::T),
    ];
    for (name, value) in defaults {
        if eval.obarray().symbol_value(name).is_none() {
            eval.set_variable(name, value);
        }
    }
}

pub(crate) fn builtin_frame_face_hash_table(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    crate::emacs_core::display::expect_range_args("frame--face-hash-table", &args, 0, 1)?;
    let frame_id = crate::emacs_core::window_cmds::resolve_frame_id_in_state(
        &mut eval.frames,
        &mut eval.buffers,
        args.first(),
        "frame-live-p",
    )?;

    Ok(eval
        .frames
        .get(frame_id)
        .map(|frame| frame.face_hash_table())
        .unwrap_or(Value::hash_table(HashTableTest::Eq)))
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn unspecified_face_attributes_vector() -> Value {
    Value::vector(vec![Value::symbol("unspecified"); LFACE_VECTOR_SIZE])
}

fn face_attr_key_name(value: &Value) -> Option<&str> {
    match value.kind() {
        ValueKind::Symbol(id) => Some(resolve_sym(id)),
        _ => None,
    }
}

pub(crate) fn builtin_face_attributes_as_vector(args: Vec<Value>) -> EvalResult {
    crate::emacs_core::display::expect_args("face-attributes-as-vector", &args, 1)?;

    let mut attrs = vec![Value::symbol("unspecified"); LFACE_VECTOR_SIZE];
    let Some(plist) = list_to_vec(&args[0]) else {
        return Ok(Value::vector(attrs));
    };

    let mut i = 0;
    while i + 1 < plist.len() {
        let Some(attr) = face_attr_key_name(&plist[i]).and_then(LFaceAttr::from_keyword) else {
            i += 2;
            continue;
        };
        let slot = attr.index();

        let value = plist[i + 1];
        match attr {
            LFaceAttr::Foreground | LFaceAttr::Background | LFaceAttr::DistantForeground
                if value.is_nil() => {}
            LFaceAttr::Stipple | LFaceAttr::Font | LFaceAttr::Inherit | LFaceAttr::Fontset => {}
            LFaceAttr::Box if value.is_t() => attrs[slot] = Value::fixnum(1),
            _ => attrs[slot] = value,
        }

        i += 2;
    }

    Ok(Value::vector(attrs))
}

pub(crate) fn init_frame_lisp_faces(frame: &mut crate::window::Frame) {
    let table = frame.face_hash_table();
    for face_name in crate::emacs_core::font::all_defined_face_names_sorted_by_id_desc().iter() {
        insert_frame_face_hash_entry_if_absent(
            table,
            Value::symbol(face_name.as_str()),
            crate::emacs_core::font::make_lisp_face_vector_for_frame(face_name.as_str()),
        );
    }
}

/// Stamp every defined face's numeric id onto its `face` symbol property, the
/// store that `face-id` / `(get FACE 'face)` read (faces.el `face-id`).
///
/// GNU assigns this property in `internal-make-lisp-face`, which `make-face`
/// invokes from its `(dolist (frame (frame-list)) ...)` loop. Neomacs registers
/// the standard faces during the bootstrap image build, before any frame
/// exists, so that loop is a no-op there and only the `face--new-frame-defaults`
/// registry entry (whose CAR holds the id) got populated -- leaving `face-id` to
/// signal "Not a face: nil" for every bootstrap face. Establishing the property
/// alongside the registry seed, from the same face set and id source
/// (`face_id_for_name`), keeps the entry and the id property from ever drifting.
pub(crate) fn stamp_face_id_properties(obarray: &mut Obarray) {
    for face_name in crate::emacs_core::font::all_defined_face_names_sorted_by_id_desc().iter() {
        if let Some(face_id) = crate::emacs_core::font::face_id_for_name(face_name.as_str()) {
            let _ = obarray.put_property(face_name.as_str(), "face", Value::fixnum(face_id));
        }
    }
}

pub(crate) fn seed_face_new_frame_defaults_table(table: Value) {
    let face_names = crate::emacs_core::font::all_defined_face_names_sorted_by_id_desc();
    let face_entries: Vec<(Value, Value)> = face_names
        .iter()
        .filter_map(|face_name| {
            let face_id = crate::emacs_core::font::face_id_for_name(face_name.as_str())?;
            Some((
                Value::symbol(face_name.as_str()),
                Value::cons(
                    Value::fixnum(face_id),
                    crate::emacs_core::font::make_lisp_face_vector(),
                ),
            ))
        })
        .collect();

    for (key, value) in face_entries {
        insert_frame_face_hash_entry_if_absent(table, key, value);
    }
}

fn bootstrap_face_new_frame_defaults_table() -> Value {
    let table = Value::hash_table(HashTableTest::Eq);
    seed_face_new_frame_defaults_table(table);
    table
}

pub(crate) fn ensure_face_new_frame_defaults_entry(
    eval: &mut crate::emacs_core::eval::Context,
    face_name: &str,
) -> Option<Value> {
    let table = eval
        .obarray()
        .symbol_value("face--new-frame-defaults")
        .copied()?;
    // The table is fully seeded once at bootstrap (`register_bootstrap_vars`)
    // and again at startup (`ensure_startup_compat_variables`). Re-seeding here
    // -- on every ensure/lookup -- rebuilt every face's cons + lface vector only
    // to discard them in `insert_..._if_absent`, an O(faces) allocation storm
    // that made `internal-lisp-face-p` the top self-time hotspot. A genuine miss
    // for the single requested face is still handled by the create-on-miss path
    // below, so dropping the blanket re-seed changes no results.
    let key = Value::symbol(face_name);
    if let Some(entry) = lookup_frame_face_hash_entry(table, key) {
        return Some(entry);
    }

    let face_id = crate::emacs_core::font::face_id_for_name(face_name)?;
    // Restore GNU's invariant that a known face carries its numeric id as the
    // `face` symbol property -- this is what `face-id` and `(get FACE 'face)`
    // read (faces.el `face-id`). GNU assigns it in `internal-make-lisp-face`,
    // invoked by `make-face`'s `(dolist (frame (frame-list)) ...)` loop. Neomacs
    // defines the standard faces during the bootstrap image build, before any
    // frame exists, so that loop is a no-op there and only the entry CAR held
    // the id. Stamp the property at the point the face first becomes known so
    // the id survives for bootstrap faces too, not just runtime `defface`s.
    eval.obarray_mut()
        .put_property(face_name, "face", Value::fixnum(face_id))
        .ok();
    let entry = Value::cons(
        Value::fixnum(face_id),
        crate::emacs_core::font::make_lisp_face_vector(),
    );
    upsert_frame_face_hash_entry(table, key, entry);
    Some(entry)
}

/// Remove a face's entry from `face--new-frame-defaults`.
///
/// The table is the canonical existence store that `internal-lisp-face-p`'s fast
/// path reads (via [`lookup_face_new_frame_defaults_vector`]). Every OTHER face
/// predicate decides existence from the known/created-face set
/// (`is_known_lisp_face_name` UNION `CREATED_LISP_FACES`). Those two stores must
/// agree, so any code that removes a face from the created-face set
/// (`clear_created_lisp_face`, e.g. on source unload) MUST also call this, or the
/// predicate would keep reporting a stale face (a hit short-circuits the
/// known-set gate). Keeping creation (`ensure_face_new_frame_defaults_entry`) and
/// removal here is the single source of truth for table membership.
pub(crate) fn remove_face_new_frame_defaults_entry(
    eval: &crate::emacs_core::eval::Context,
    face_name: &str,
) {
    let Some(table) = eval
        .obarray()
        .symbol_value("face--new-frame-defaults")
        .copied()
    else {
        return;
    };
    if table.is_hash_table() {
        // Face keys are plain interned symbols; symbols_with_pos is irrelevant.
        let _ = crate::emacs_core::builtins::collections::builtin_remhash_values(
            Value::symbol(face_name),
            table,
            false,
        );
    }
}

pub(crate) fn face_new_frame_defaults_vector(
    eval: &mut crate::emacs_core::eval::Context,
    face_name: &str,
) -> Option<Value> {
    let entry = ensure_face_new_frame_defaults_entry(eval, face_name)?;
    if entry.is_cons() {
        Some(entry.cons_cdr())
    } else {
        None
    }
}

/// Pure, allocation-free read of the global `face--new-frame-defaults` table,
/// mirroring GNU's `lface_from_face_name` for the null-frame case: one
/// symbol-keyed hash lookup, no seeding and no create-on-miss. `key` must be an
/// already-interned symbol `Value`. Returns the lface vector (entry CDR), or
/// None when the face is absent. Callers that need create-on-miss (defface,
/// copy-face, make-lisp-face) use `face_new_frame_defaults_vector` instead.
pub(crate) fn lookup_face_new_frame_defaults_vector(
    eval: &crate::emacs_core::eval::Context,
    key: Value,
) -> Option<Value> {
    let table = eval
        .obarray()
        .symbol_value("face--new-frame-defaults")
        .copied()?;
    let entry = lookup_frame_face_hash_entry(table, key)?;
    if entry.is_cons() {
        Some(entry.cons_cdr())
    } else {
        None
    }
}

pub(crate) fn lookup_frame_face_hash_entry(table: Value, key: Value) -> Option<Value> {
    if !table.is_hash_table() {
        return None;
    }
    let hash_table = table.as_hash_table()?;
    let hash_key = match key.kind() {
        ValueKind::Symbol(id) => HashKey::Symbol(id),
        _ => return None,
    };
    hash_table.data.get(&hash_key).copied()
}

fn insert_frame_face_hash_entry_if_absent(table: Value, key: Value, value: Value) {
    if lookup_frame_face_hash_entry(table, key).is_none() {
        upsert_frame_face_hash_entry(table, key, value);
    } else {
        let _ = table.with_hash_table_mut(|hash_table| {
            let hash_key = match key.kind() {
                ValueKind::Symbol(id) => HashKey::Symbol(id),
                _ => unreachable!("face hash keys are symbols"),
            };
            hash_table.replace_key_snapshot(&hash_key, key);
        });
    }
}

pub(crate) fn upsert_frame_face_hash_entry(table: Value, key: Value, value: Value) {
    if !table.is_hash_table() {
        unreachable!("frame face hash table must be a hash table");
    };
    let _ = table.with_hash_table_mut(|hash_table| {
        let hash_key = match key.kind() {
            ValueKind::Symbol(id) => HashKey::Symbol(id),
            _ => unreachable!("face hash keys are symbols"),
        };
        // Use the O(1) puthash-style upsert; `ensure_hash_key_iterable`'s
        // duplicate scan is O(n) and made face realisation O(n^2).
        hash_table.upsert_iterable(hash_key, key, value);
    });
}

#[cfg(test)]
#[path = "xfaces_test.rs"]
mod tests;
