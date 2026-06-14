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
