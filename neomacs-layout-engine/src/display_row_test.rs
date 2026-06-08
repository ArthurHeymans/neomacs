use super::*;
use crate::neovm_bridge::FaceResolver;
use neomacs_display_protocol::Rect;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::buffer::{BufferId, CharPos0};
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;

fn base_face() -> crate::neovm_bridge::ResolvedFace {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    resolver.default_face().clone()
}

#[test]
fn display_row_request_accepts_window_chrome_roles() {
    let _eval = Context::new();
    for role in [
        GlyphRowRole::ModeLine,
        GlyphRowRole::HeaderLine,
        GlyphRowRole::TabLine,
    ] {
        let request = DisplayRowRequest {
            role,
            x: 1.0,
            y: 2.0,
            width: 80.0,
            height: 16.0,
            window_id: 7,
            matrix_row: Some(0),
            base_face: base_face(),
            source: LispStringSource {
                string: Value::string("row"),
            },
        };

        assert_eq!(request.role, role);
        assert_eq!(request.window_id, 7);
        assert_eq!(request.matrix_row, Some(0));
    }
}

#[test]
fn display_row_request_accepts_frame_and_minibuffer_roles() {
    let _eval = Context::new();
    for role in [GlyphRowRole::TabBar, GlyphRowRole::Minibuffer] {
        let request = DisplayRowRequest {
            role,
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 20.0,
            window_id: 0,
            matrix_row: None,
            base_face: base_face(),
            source: LispStringSource {
                string: Value::string("plain"),
            },
        };

        assert_eq!(request.role, role);
        assert!(request.matrix_row.is_none());
    }
}

#[test]
fn display_row_request_represents_plain_text_as_lisp_string_without_properties() {
    let _eval = Context::new();
    let request = DisplayRowRequest {
        role: GlyphRowRole::TabBar,
        x: 0.0,
        y: 0.0,
        width: 80.0,
        height: 16.0,
        window_id: 0,
        matrix_row: None,
        base_face: base_face(),
        source: LispStringSource {
            string: Value::string("plain"),
        },
    };

    assert_eq!(
        request.source.string.as_runtime_string_owned().as_deref(),
        Some("plain")
    );
    assert!(
        neovm_core::emacs_core::value::get_string_text_properties_table_for_value(
            request.source.string,
        )
        .is_none()
    );
}

#[test]
fn display_row_source_lisp_string_represents_unpropertized_text_as_lisp_string() {
    let _eval = Context::new();
    let source = LispStringSource {
        string: Value::string("plain"),
    };

    assert_eq!(source.text().as_deref(), Some("plain"));
    assert!(
        neovm_core::emacs_core::value::get_string_text_properties_table_for_value(source.string)
            .is_none()
    );
}

#[test]
fn display_row_source_lisp_string_builds_existing_row_spec() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let source = LispStringSource {
        string: Value::string("row"),
    };
    let mut base_face = resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let request = DisplayRowRequest {
        role: GlyphRowRole::TabLine,
        x: 0.0,
        y: 2.0,
        width: 80.0,
        height: 16.0,
        window_id: 7,
        matrix_row: Some(0),
        base_face,
        source,
    };
    let mut next_face_id = 1;

    let spec = source
        .build_row_spec(
            &mut engine,
            request,
            &mut next_face_id,
            &resolver,
            std::collections::HashMap::new(),
        )
        .expect("row spec from lisp source");

    assert_eq!(spec.role, GlyphRowRole::TabLine);
    assert_eq!(spec.text, b"row");
}

#[test]
fn display_row_request_uses_lisp_string_source_for_text_properties() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut base_face = resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let request = DisplayRowRequest {
        role: GlyphRowRole::TabBar,
        x: 0.0,
        y: 0.0,
        width: 80.0,
        height: 16.0,
        window_id: 0,
        matrix_row: None,
        base_face,
        source: LispStringSource {
            string: Value::string_with_text_properties(
                "AB",
                vec![neovm_core::emacs_core::value::StringTextPropertyRun {
                    start: 1,
                    end: 2,
                    plist: Value::list(vec![
                        Value::symbol("face"),
                        Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
                    ]),
                }],
            ),
        },
    };
    let mut next_face_id = 1;

    let spec = engine
        .build_display_row_spec_from_request(
            request,
            &mut next_face_id,
            &resolver,
            std::collections::HashMap::new(),
        )
        .expect("display row request spec");

    assert_eq!(spec.face_runs.len(), 1);
    assert_eq!(spec.face_runs[0].byte_offset, 1);
}

#[test]
fn display_row_source_buffer_text_is_typed_separately_from_lisp_string() {
    let source = BufferTextSource {
        buffer_id: BufferId(3),
        window_id: 9,
        start: CharPos0::new(4),
        end: CharPos0::new(12),
    };
    let kind = DisplayRowSourceKind::BufferText(source.clone());

    assert_eq!(source.buffer_id, BufferId(3));
    assert_eq!(source.window_id, 9);
    assert_eq!(source.start, CharPos0::new(4));
    assert_eq!(source.end, CharPos0::new(12));
    assert!(matches!(kind, DisplayRowSourceKind::BufferText(_)));
}

#[test]
fn render_propertized_display_row_preserves_pixel_widths() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let rendered = Value::string("AB");
    let mut next_face_id = 1;
    let spec: DisplayRowSpec = engine
        .build_propertized_display_row_spec(
            0.0,
            0.0,
            80.0,
            16.0,
            1,
            8.0,
            12.0,
            &mut next_face_id,
            resolver.default_face(),
            rendered,
            &resolver,
            std::collections::HashMap::new(),
            GlyphRowRole::TabLine,
        )
        .expect("display row spec");

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    let _ = engine.render_display_row_spec_via_backend(&spec, Some(0), Some(&mut builder), None);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let glyphs = &state.window_matrices[0].matrix.rows[0].glyphs[1];

    assert_eq!(glyphs.len(), 2);
    assert!(
        glyphs.iter().all(|glyph| glyph.pixel_width > 0.0),
        "display row glyphs should carry measured/fallback pixel widths: {glyphs:?}"
    );
}

#[test]
fn render_display_row_spec_to_glyph_row_returns_role_and_pixels() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let rendered = Value::string("TB");
    let mut next_face_id = 1;
    let spec = engine
        .build_propertized_display_row_spec(
            0.0,
            0.0,
            80.0,
            16.0,
            1,
            8.0,
            12.0,
            &mut next_face_id,
            resolver.default_face(),
            rendered,
            &resolver,
            std::collections::HashMap::new(),
            GlyphRowRole::TabBar,
        )
        .expect("display row spec");

    let (row, progress) = engine
        .render_display_row_spec_to_glyph_row(&spec, None)
        .expect("glyph row");

    assert_eq!(row.role, GlyphRowRole::TabBar);
    assert!(row.enabled);
    assert_eq!(row.glyphs[1].len(), 2);
    assert!(
        row.glyphs[1].iter().all(|glyph| glyph.pixel_width > 0.0),
        "direct display row glyphs should carry pixel widths: {:?}",
        row.glyphs[1]
    );
    assert!(progress.end_x > 0.0);
}

#[test]
fn render_plain_display_row_request_returns_glyph_row() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut base_face = resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let request = DisplayRowRequest {
        role: GlyphRowRole::TabBar,
        x: 0.0,
        y: 0.0,
        width: 80.0,
        height: 16.0,
        window_id: 0,
        matrix_row: None,
        base_face,
        source: LispStringSource {
            string: Value::string("tab"),
        },
    };
    let mut next_face_id = 1;
    let spec = engine
        .build_display_row_spec_from_request(
            request,
            &mut next_face_id,
            &resolver,
            std::collections::HashMap::new(),
        )
        .expect("display row request spec");
    let (row, progress) = engine
        .render_display_row_spec_to_glyph_row(&spec, None)
        .expect("glyph row");

    assert_eq!(row.role, GlyphRowRole::TabBar);
    assert_eq!(row.glyphs[1].len(), 3);
    assert!(row.glyphs[1].iter().all(|glyph| glyph.pixel_width > 0.0));
    assert!(progress.end_x > 0.0);
}
