use super::*;

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
