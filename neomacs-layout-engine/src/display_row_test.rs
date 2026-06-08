use super::*;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView};
use neomacs_display_protocol::Rect;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphRow, GlyphType};
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;

fn base_face() -> crate::neovm_bridge::ResolvedFace {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    resolver.default_face().clone()
}

#[test]
fn display_row_face_realizer_realizes_face_without_layout_engine() {
    let mut font_metrics = None;
    let mut realizer = DisplayRowFaceRealizer::new(&mut font_metrics);
    let mut face = base_face();
    face.font_char_width = 0.0;
    face.font_ascent = 0.0;
    face.font_line_height = 0.0;

    let rendered = realizer.realize_face(7, &face, 8.0, 12.0, 16.0);

    assert_eq!(rendered.face_id, 7);
    assert_eq!(rendered.font_char_width, 1.0);
    assert_eq!(rendered.font_ascent, 1.0);
    assert_eq!(rendered.font_descent, 0);
}

fn row_text_expanding_stretches(row: &GlyphRow) -> String {
    row.glyphs[1]
        .iter()
        .filter(|glyph| !glyph.padding)
        .flat_map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } => std::iter::repeat_n(*ch, 1).collect::<Vec<_>>(),
            GlyphType::Composite { text } => text.chars().collect::<Vec<_>>(),
            GlyphType::Stretch { width_cols } => {
                std::iter::repeat_n(' ', usize::from(*width_cols)).collect::<Vec<_>>()
            }
            _ => Vec::new(),
        })
        .collect()
}

fn render_lisp_display_row(rendered: Value, role: GlyphRowRole) -> GlyphRow {
    render_lisp_display_row_with_symbols(rendered, role, std::collections::HashMap::new())
}

fn render_lisp_display_row_with_symbols(
    rendered: Value,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
) -> GlyphRow {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        resolver.default_face(),
        role,
        symbol_values,
    );
    engine
        .render_display_source_row(spec, rendered, &resolver, &mut next_face_id)
        .expect("display source row")
        .row
}

#[test]
fn render_display_item_source_row_accepts_buffer_text_source() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("A中👨‍👩");
    }
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let mut source = crate::display_source::BufferTextSourceCursor::new(
        buf_id,
        buffer,
        neovm_core::buffer::CharPos0::new(0),
        buffer.layout_point_max_char_pos(),
        RenderFaceRef::FaceId(1),
    );

    let rendered = engine
        .render_display_item_source_row(
            DisplayRowSpec {
                y: 0.0,
                width: 240.0,
                height: 16.0,
                char_width: 8.0,
                ascent: 12.0,
                tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
                base_face_id: 1,
                base_face: resolver.default_face(),
                role: GlyphRowRole::TabLine,
                symbol_values: std::collections::HashMap::new(),
            },
            &mut source,
            &resolver,
            &mut next_face_id,
        )
        .expect("display source row");

    let row = rendered.row;
    let glyphs = &row.glyphs[1];
    let cjk = glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("CJK glyph");

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert_eq!(row_text_expanding_stretches(&row), "A中👨‍👩");
    assert!(cjk.wide);
    assert!(glyphs.iter().any(|glyph| glyph.padding));
    assert!(
        glyphs.iter().any(
            |glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))
        )
    );
}

#[test]
fn render_display_item_source_row_uses_spec_tab_policy() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("\tX");
    }
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let mut source = crate::display_source::BufferTextSourceCursor::new(
        buf_id,
        buffer,
        neovm_core::buffer::CharPos0::new(0),
        buffer.layout_point_max_char_pos(),
        RenderFaceRef::FaceId(1),
    );

    let rendered = engine
        .render_display_item_source_row(
            DisplayRowSpec {
                y: 0.0,
                width: 240.0,
                height: 16.0,
                char_width: 8.0,
                ascent: 12.0,
                base_face_id: 1,
                base_face: resolver.default_face(),
                role: GlyphRowRole::TabLine,
                symbol_values: std::collections::HashMap::new(),
                tab_policy: crate::display_row_builder::DisplayTabPolicy::from_tab_width_and_stops(
                    0.0,
                    4,
                    &[2],
                ),
            },
            &mut source,
            &resolver,
            &mut next_face_id,
        )
        .expect("display source row");

    let glyphs = &rendered.row.glyphs[1];
    assert_eq!(glyphs[0].glyph_type, GlyphType::Stretch { width_cols: 2 });
    let emitted_width: f32 = glyphs.iter().map(|glyph| glyph.pixel_width).sum();
    assert!(
        (rendered.progress.end_x - emitted_width).abs() <= 0.01,
        "row progress should include the emitted tab stretch and following character"
    );
}

#[test]
fn render_display_source_row_uses_explicit_tab_policy() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::from_tab_width_and_stops(
                0.0,
                4,
                &[2],
            ),
        },
        &mut next_face_id,
        resolver.default_face(),
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let rendered = engine
        .render_display_source_row(spec, Value::string("\tX"), &resolver, &mut next_face_id)
        .expect("display source row");

    let glyphs = &rendered.row.glyphs[1];
    assert_eq!(glyphs[0].glyph_type, GlyphType::Stretch { width_cols: 2 });
}

#[test]
fn display_row_glyph_measurer_uses_face_specific_widths() {
    let mut base = base_face();
    base.font_char_width = 5.0;
    let mut wide = base.clone();
    wide.font_char_width = 9.0;
    let faces = vec![
        DisplayRowFace::from_resolved(1, &base),
        DisplayRowFace::from_resolved(2, &wide),
    ];
    let mut measurer = DisplayRowGlyphMeasurer::new(&faces, None, 5.0);

    assert_eq!(measurer.glyph_advance_px('a', 1, 1, 5.0), Some(5.0));
    assert_eq!(measurer.glyph_advance_px('中', 2, 2, 10.0), Some(18.0));
}

#[test]
fn display_row_baseline_tab_bar_preserves_lisp_string_face_properties() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "AB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::TabBar);
    let glyphs = &row.glyphs[1];

    assert_eq!(row.role, GlyphRowRole::TabBar);
    assert_eq!(row_text_expanding_stretches(&row), "AB");
    assert_eq!(glyphs.len(), 2);
    assert_ne!(
        glyphs[0].face_id, glyphs[1].face_id,
        "propertized tab-bar chars should keep separate face ids"
    );
}

#[test]
fn display_row_baseline_mode_line_display_space_align_expands_to_spaces() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "A B",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("align-to"),
                    Value::fixnum(4),
                ]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::ModeLine);

    assert_eq!(row.role, GlyphRowRole::ModeLine);
    assert_eq!(row_text_expanding_stretches(&row), "A   B");
}

#[test]
fn display_row_baseline_header_line_display_space_relative_width_expands_to_stretch() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "C R",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("relative-width"),
                    Value::fixnum(2),
                ]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::HeaderLine);

    assert_eq!(row.role, GlyphRowRole::HeaderLine);
    assert_eq!(row_text_expanding_stretches(&row), "C  R");
    assert!(
        row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 2 })),
        "relative-width display space should become a stretch glyph: {:?}",
        row.glyphs[1]
    );
}

#[test]
fn display_row_baseline_header_line_align_to_symbol_values() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "C ",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("align-to"),
                    Value::list(vec![
                        Value::symbol("+"),
                        Value::symbol("header-line-indent-width"),
                        Value::fixnum(1),
                    ]),
                ]),
            ]),
        }],
    );
    let mut symbol_values = std::collections::HashMap::new();
    symbol_values.insert("header-line-indent-width".to_string(), Value::fixnum(0));

    let row =
        render_lisp_display_row_with_symbols(rendered, GlyphRowRole::HeaderLine, symbol_values);

    assert_eq!(row.role, GlyphRowRole::HeaderLine);
    assert_eq!(row_text_expanding_stretches(&row), "C");
}

#[test]
fn display_row_baseline_header_line_align_to_skips_multi_char_interval() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "X   Y",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 4,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("align-to"),
                    Value::fixnum(4),
                ]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::HeaderLine);

    assert_eq!(row.role, GlyphRowRole::HeaderLine);
    assert_eq!(row_text_expanding_stretches(&row), "X   Y");
    assert!(
        row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 3 })),
        "multi-char display interval should become one stretch glyph: {:?}",
        row.glyphs[1]
    );
}

#[test]
fn display_row_baseline_header_line_align_to_after_multibyte_prefix_uses_character_offsets() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "λC R",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 2,
            end: 3,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("align-to"),
                    Value::fixnum(4),
                ]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::HeaderLine);

    assert_eq!(row.role, GlyphRowRole::HeaderLine);
    assert_eq!(row_text_expanding_stretches(&row), "λC  R");
}

#[test]
fn render_display_source_row_uses_face_specific_glyph_widths() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut base_face = resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let rendered = Value::string_with_text_properties(
        "AB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![
                    Value::keyword("family"),
                    Value::string("JetBrains Mono"),
                    Value::keyword("height"),
                    Value::make_float(2.0),
                ]),
            ]),
        }],
    );
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 32.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        &base_face,
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let row = engine
        .render_display_source_row(spec, rendered, &resolver, &mut next_face_id)
        .expect("display source row")
        .row;
    let glyphs = &row.glyphs[1];

    assert_eq!(glyphs.len(), 2);
    assert!(
        glyphs[1].pixel_width > glyphs[0].pixel_width,
        "face-height run should be measured wider than base run: {glyphs:?}"
    );
}

#[test]
fn display_row_tab_line_wide_char_uses_shared_wide_glyph() {
    let _eval = Context::new();
    let row = render_lisp_display_row(Value::string("A中B"), GlyphRowRole::TabLine);
    let glyphs = &row.glyphs[1];
    let cjk = glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("CJK glyph");

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert_eq!(row_text_expanding_stretches(&row), "A中B");
    assert!(
        cjk.wide,
        "tab-line CJK should use the shared wide glyph path: {glyphs:?}"
    );
    assert!(
        glyphs.iter().any(|glyph| glyph.padding),
        "tab-line CJK should retain a padding cell like main buffer text: {glyphs:?}"
    );
}

#[test]
fn display_row_tab_line_zwj_emoji_sequence_uses_shared_cluster() {
    let _eval = Context::new();
    let row = render_lisp_display_row(Value::string("👨‍👩"), GlyphRowRole::TabLine);
    let glyphs = &row.glyphs[1];

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert_eq!(row_text_expanding_stretches(&row), "👨‍👩");
    assert!(
        glyphs
            .iter()
            .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))),
        "tab-line ZWJ emoji should use the shared cluster path: {glyphs:?}"
    );
}

#[test]
fn display_row_lisp_chrome_roles_share_wide_and_cluster_builder() {
    let _eval = Context::new();

    for role in [
        GlyphRowRole::ModeLine,
        GlyphRowRole::HeaderLine,
        GlyphRowRole::TabLine,
        GlyphRowRole::TabBar,
    ] {
        let row = render_lisp_display_row(Value::string("A中👨‍👩"), role);
        let glyphs = &row.glyphs[1];
        let cjk = glyphs
            .iter()
            .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
            .expect("CJK glyph");

        assert_eq!(row.role, role);
        assert_eq!(row_text_expanding_stretches(&row), "A中👨‍👩");
        assert!(
            cjk.wide,
            "Lisp-string chrome role {role:?} should use the shared wide-glyph path: {glyphs:?}"
        );
        assert!(
            glyphs.iter().any(|glyph| glyph.padding),
            "Lisp-string chrome role {role:?} should retain CJK padding cells: {glyphs:?}"
        );
        assert!(
            glyphs
                .iter()
                .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))),
            "Lisp-string chrome role {role:?} should use the shared cluster path: {glyphs:?}"
        );
    }
}

#[test]
fn display_row_baseline_tab_line_rtl_text_is_reordered_after_row_build() {
    let _eval = Context::new();
    let row = render_lisp_display_row(Value::string("אב"), GlyphRowRole::TabLine);

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert!(
        row.reversed_p,
        "pure RTL chrome row should be marked reversed"
    );
    assert_eq!(row_text_expanding_stretches(&row), "בא");
}

#[test]
fn install_display_source_row_preserves_prebuilt_bidi_metadata() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 80.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        resolver.default_face(),
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );
    let rendered = engine
        .render_display_source_row(spec, Value::string("אב"), &resolver, &mut next_face_id)
        .expect("display source row");

    assert!(rendered.row.reversed_p);
    assert_eq!(row_text_expanding_stretches(&rendered.row), "בא");

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    install_rendered_display_source_row(&mut builder, &rendered, 0);
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];
    assert!(row.reversed_p);
    assert_eq!(row_text_expanding_stretches(row), "בא");
}
