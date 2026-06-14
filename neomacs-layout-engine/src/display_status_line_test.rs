use super::*;
use crate::display_row::DisplayRowFace;
use neomacs_display_protocol::glyph_matrix::GlyphArea;

fn row_text(row: &GlyphRow) -> String {
    row.glyphs[GlyphArea::Text.index()]
        .iter()
        .filter_map(|glyph| match &glyph.glyph_type {
            neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(ch.to_owned()),
            neomacs_display_protocol::glyph_matrix::GlyphType::Composite { text } => {
                text.chars().next()
            }
            neomacs_display_protocol::glyph_matrix::GlyphType::Stretch { .. }
            | neomacs_display_protocol::glyph_matrix::GlyphType::Glyphless { .. }
            | neomacs_display_protocol::glyph_matrix::GlyphType::Image { .. } => None,
        })
        .collect()
}

#[test]
fn display_row_height_for_face_uses_realized_line_height_and_box() {
    let mut engine = LayoutEngine::new();
    let mut face = ResolvedFace::default();
    face.font_family = "monospace".to_string();
    face.font_size = 14.0;
    face.font_ascent = 9.0;
    face.font_line_height = 12.0;
    face.box_type = 1;
    face.box_line_width = 1;

    assert_eq!(
        engine.display_row_height_for_face(&face, 8.0, 12.0, 20.0),
        20.0
    );
}

#[test]
fn window_chrome_display_text_preserves_lisp_value_for_source_renderer() {
    let _eval = Context::new();

    assert_eq!(
        WindowChromeDisplayText::new(Value::string("tab-line"), true)
            .value()
            .as_utf8_str(),
        Some("tab-line")
    );
    assert_eq!(
        WindowChromeDisplayText::new(Value::string("header"), true)
            .value()
            .as_utf8_str(),
        Some("header")
    );
    assert_eq!(
        WindowChromeDisplayText::new(Value::string("mode"), false)
            .value()
            .as_utf8_str(),
        Some("mode")
    );
}

#[test]
fn chrome_lisp_string_row_request_preserves_policy_inputs() {
    let _eval = Context::new();
    let base_face = ResolvedFace::default();
    let mut symbol_values = std::collections::HashMap::new();
    let align_value = Value::make_int(12);
    symbol_values.insert("align-to".to_string(), align_value);

    let policy = ChromeLispStringRowRequest::new(
        3.0,
        80.0,
        16.0,
        8.0,
        12.0,
        DisplayTabPolicy::every(4),
        GlyphRowRole::ModeLine,
        &base_face,
        Value::string("mode"),
    )
    .with_symbol_values(symbol_values)
    .into_source_request_policy();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let request = policy.source_request_from_base_face(&mut face_ids, &base_face);

    assert_eq!(request.role(), GlyphRowRole::ModeLine);
    assert_eq!(request.geometry().y, 3.0);
    assert_eq!(request.geometry().width, 80.0);
    assert_eq!(request.geometry().height, 16.0);
    assert_eq!(request.geometry().char_width, 8.0);
    assert_eq!(request.geometry().ascent, 12.0);
    assert_eq!(
        request.symbol_values().get("align-to").copied(),
        Some(align_value)
    );
}

#[test]
fn window_chrome_target_cols_reserves_right_border_column() {
    assert_eq!(window_chrome_target_cols(80.0, 8.0, false), 10);
    assert_eq!(window_chrome_target_cols(80.0, 8.0, true), 9);
    assert_eq!(window_chrome_target_cols(3.0, 8.0, true), 1);
    assert_eq!(window_chrome_target_cols(80.0, 0.0, false), 80);
}

#[test]
fn echo_minibuffer_source_row_request_builds_session_row_request() {
    let _eval = Context::new();
    let mut base_face = ResolvedFace::default();
    base_face.face_id = 7;
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let session_request = DisplayRowLispStringSourceSessionRequest::from_base_face(
        Value::string("echo"),
        &mut face_ids,
        &base_face,
    );
    let source_session =
        DisplayRowLispStringSourceSession::new(session_request).expect("source session");

    let request = EchoMinibufferSourceRowRequest::new(2, 4.0, 40.0, 16.0, 8.0, 12.0, &base_face)
        .source_session_row_request(&source_session);

    assert_eq!(request.role(), GlyphRowRole::Minibuffer);
    assert_eq!(request.base_face_id(), 7);
    assert_eq!(request.geometry().y, 36.0);
    assert_eq!(request.geometry().width, 40.0);
    assert_eq!(request.geometry().height, 16.0);
    assert_eq!(request.geometry().char_width, 8.0);
    assert_eq!(request.geometry().ascent, 12.0);
}

#[test]
fn echo_minibuffer_clipped_row_appends_reserved_marker_through_text_row() {
    let _eval = Context::new();
    let table = neovm_core::face::FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut base_face = resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    base_face.font_line_height = 16.0;
    let mut builder = GlyphMatrixBuilder::new();
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(1);

    let rows = EchoMinibufferRowsRenderRequest {
        y: 0.0,
        text_width: 24.0,
        char_width: 8.0,
        ascent: 12.0,
        row_height: 16.0,
        base_face: &base_face,
        message: Value::string("ABCD"),
        max_rows: 1,
        truncate_lines: false,
        reserve_right_special_col: true,
    }
    .render_rows(&mut MinibufferDisplayRenderState {
        builder: &mut builder,
        font_metrics: &mut font_metrics,
        face_resolver: &resolver,
        display_host: None,
        face_ids: &mut face_ids,
    });

    assert_eq!(rows.len(), 1);
    assert_eq!(row_text(&rows[0].row), "AB\\");
    assert_eq!(rows[0].progress.end_col, 3);
    assert_eq!(
        rows[0].row.glyphs[GlyphArea::Text.index()][2].pixel_width,
        8.0
    );
}

#[test]
fn display_row_face_preserves_gnu_box_type_codes() {
    let mut resolved = ResolvedFace::default();
    let boxes = [
        (0, BoxType::None),
        (1, BoxType::Line),
        (2, BoxType::Raised3D),
        (3, BoxType::Sunken3D),
    ];

    for (code, box_type) in boxes {
        resolved.box_type = code;
        let row_face = DisplayRowFace::from_resolved(1, &resolved);
        assert_eq!(row_face.box_type, box_type);
        assert_eq!(row_face.render_face().box_type, box_type);
    }
}
