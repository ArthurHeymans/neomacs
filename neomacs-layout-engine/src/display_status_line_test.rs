use super::*;
use crate::display_row::DisplayRowFace;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;

#[test]
fn display_row_height_for_face_uses_realized_line_height_and_box() {
    let mut font_metrics = None;
    let mut face = ResolvedFace::default();
    face.font_family = "monospace".to_string();
    face.font_size = 14.0;
    face.font_ascent = 9.0;
    face.font_line_height = 12.0;
    face.box_type = 1;
    face.box_line_width = 1;

    assert_eq!(
        window_chrome_row_height_for_face(&mut font_metrics, &face, 8.0, 12.0, 20.0),
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
        DisplayOrigin::ModeLine { selected: true },
        &base_face,
        Value::string("mode"),
    )
    .with_symbol_values(symbol_values)
    .into_source_request_policy();
    let geometry = policy.geometry();

    assert_eq!(policy.role(), GlyphRowRole::ModeLine);
    assert_eq!(geometry.y, 3.0);
    assert_eq!(geometry.width, 80.0);
    assert_eq!(geometry.height, 16.0);
    assert_eq!(geometry.char_width, 8.0);
    assert_eq!(geometry.ascent, 12.0);
    assert_eq!(
        policy.symbol_values().get("align-to").copied(),
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
