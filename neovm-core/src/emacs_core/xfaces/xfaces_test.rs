use super::*;

// --- FaceColorResolver::realize (spec -> realized color bridge) ---

#[test]
fn realize_standard_named_and_hex_specs() {
    crate::test_utils::init_test_tracing();
    use crate::face::{Color, SpecifiedColor};
    let r = FaceColorResolver::Standard;
    assert_eq!(
        r.realize(&SpecifiedColor::parse("red")),
        Some(Color::rgb(255, 0, 0))
    );
    assert_eq!(
        r.realize(&SpecifiedColor::parse("#abc")),
        Some(Color::rgb(170, 187, 204))
    );
    assert_eq!(r.realize(&SpecifiedColor::parse("no-such-color")), None);
    assert_eq!(
        r.realize(&SpecifiedColor::Rgb(9, 8, 7)),
        Some(Color::rgb(9, 8, 7))
    );
}

#[test]
fn realize_unspecified_and_frame_default_specs_stay_unrealized() {
    crate::test_utils::init_test_tracing();
    use crate::face::SpecifiedColor;
    // GNU realize_tty_face maps unspecified-fg/-bg to the frame defaults at
    // realization; in neomacs the frame default substitution happens earlier
    // (realize_default_lisp_face_for_frame rewrites the default face vector),
    // so at this boundary the specs realize to None and downstream frame
    // defaults apply — identical to the pre-split string behavior where
    // "unspecified-fg" failed the name lookup.
    for spec in [
        SpecifiedColor::Unspecified,
        SpecifiedColor::FrameForeground,
        SpecifiedColor::FrameBackground,
    ] {
        assert_eq!(FaceColorResolver::Standard.realize(&spec), None);
        assert_eq!(
            FaceColorResolver::TtyPalette(&TtyColorMap::default()).realize(&spec),
            None
        );
    }
}

#[test]
fn realize_tty_palette_wins_over_standard_parse() {
    crate::test_utils::init_test_tracing();
    use crate::face::{Color, SpecifiedColor};
    let mut palette = TtyColorMap::default();
    // xterm registers "white" as 229,229,229 — the palette must beat rgb.txt.
    palette.insert("white".to_owned(), Color::rgb(229, 229, 229));
    // A tty without 24-bit color approximates hex through the palette too
    // (GNU tty-color-desc -> tty-color-approximate), keyed by the exact
    // lface string.
    palette.insert("#ff0000".to_owned(), Color::rgb(205, 0, 0));
    let r = FaceColorResolver::TtyPalette(&palette);
    assert_eq!(
        r.realize(&SpecifiedColor::parse("white")),
        Some(Color::rgb(229, 229, 229))
    );
    assert_eq!(
        r.realize(&SpecifiedColor::parse("#ff0000")),
        Some(Color::rgb(205, 0, 0))
    );
    // Not in the palette: fall back to the standard parse, GNU's
    // failed-tty_lookup_color fallback.
    assert_eq!(
        r.realize(&SpecifiedColor::parse("gold")),
        Some(Color::rgb(255, 215, 0))
    );
    assert_eq!(r.realize(&SpecifiedColor::parse("no-such-color")), None);
}

#[test]
fn register_bootstrap_vars_matches_gnu_defaults() {
    crate::test_utils::init_test_tracing();
    let mut obarray = Obarray::new();
    register_bootstrap_vars(&mut obarray);

    assert_eq!(
        obarray.symbol_value("face-default-stipple").copied(),
        Some(Value::string("gray3"))
    );
    assert_eq!(
        obarray
            .symbol_value("face-near-same-color-threshold")
            .copied(),
        Some(Value::fixnum(30_000))
    );
    assert_eq!(
        obarray
            .symbol_value("face-font-lax-matched-attributes")
            .copied(),
        Some(Value::T)
    );

    let table = obarray
        .symbol_value("face--new-frame-defaults")
        .copied()
        .expect("face--new-frame-defaults");
    if !table.is_hash_table() {
        panic!("face--new-frame-defaults must be a hash table");
    };
    let test = table.as_hash_table().unwrap().test.clone();
    assert_eq!(test, HashTableTest::Eq);
}

#[test]
fn frame_face_hash_table_eval_has_initialized_default_face() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let out = builtin_frame_face_hash_table(&mut eval, vec![Value::NIL])
        .expect("live frame face hash table");
    if !out.is_hash_table() {
        panic!("expected hash table");
    };
    let default = lookup_frame_face_hash_entry(out, Value::symbol("default"))
        .expect("selected frame should have a default Lisp face vector");
    assert!(default.is_vector());
}

#[test]
fn frame_face_hash_table_eval_returns_stable_frame_owned_table() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let first =
        builtin_frame_face_hash_table(&mut eval, vec![Value::NIL]).expect("first face hash table");
    let second =
        builtin_frame_face_hash_table(&mut eval, vec![Value::NIL]).expect("second face hash table");
    assert_eq!(first, second);
}

#[test]
fn ensure_startup_compat_variables_backfills_missing_xfaces_state() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    for name in [
        "face-filters-always-match",
        "face--new-frame-defaults",
        "face-default-stipple",
        "scalable-fonts-allowed",
        "face-ignored-fonts",
        "face-remapping-alist",
        "face-font-rescale-alist",
        "face-near-same-color-threshold",
        "face-font-lax-matched-attributes",
    ] {
        eval.obarray_mut().makunbound(name);
    }

    ensure_startup_compat_variables(&mut eval);

    assert_eq!(
        eval.obarray().symbol_value("face-default-stipple").copied(),
        Some(Value::string("gray3"))
    );
    let table = eval
        .obarray()
        .symbol_value("face--new-frame-defaults")
        .copied()
        .expect("face hash table backfilled");
    if !table.is_hash_table() {
        panic!("face--new-frame-defaults must be a hash table");
    };
    let has_seeded_faces =
        {
            let hash_table = table.as_hash_table().unwrap();
            hash_table
                .data
                .contains_key(&HashKey::Symbol(crate::emacs_core::intern::intern(
                    "default",
                )))
                && hash_table.data.contains_key(&HashKey::Symbol(
                    crate::emacs_core::intern::intern("mode-line"),
                ))
        };
    assert!(
        has_seeded_faces,
        "face--new-frame-defaults should be preseeded with GNU face entries"
    );
}

#[test]
fn ensure_startup_compat_variables_reseeds_existing_face_defaults_table() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let table = Value::hash_table(HashTableTest::Eq);
    eval.set_variable("face--new-frame-defaults", table);

    ensure_startup_compat_variables(&mut eval);

    let table = eval
        .obarray()
        .symbol_value("face--new-frame-defaults")
        .copied()
        .expect("face hash table should stay bound");
    let hash_table = table
        .as_hash_table()
        .expect("face--new-frame-defaults must remain a hash table");
    assert!(
        hash_table
            .data
            .contains_key(&HashKey::Symbol(crate::emacs_core::intern::intern(
                "mode-line",
            ))),
        "existing face--new-frame-defaults tables must be reseeded after dump load"
    );
}
