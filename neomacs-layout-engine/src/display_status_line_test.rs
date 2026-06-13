use super::*;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_origin::DisplayOrigin;
use neomacs_display_protocol::face::BasicFaceId;

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
fn window_chrome_display_text_lowers_by_chrome_kind() {
    let _eval = Context::new();

    let tab_line = WindowChromeDisplayText::new(Value::string("tab-line"), true)
        .fragment(WindowChromeKind::TabLine);
    assert_eq!(tab_line.origin, DisplayOrigin::TabLine);
    assert_eq!(
        tab_line.base_face_policy,
        BaseFacePolicy::FixedBasicFace(BasicFaceId::TabLine)
    );

    let active_header = WindowChromeDisplayText::new(Value::string("header"), true)
        .fragment(WindowChromeKind::HeaderLine);
    assert_eq!(active_header.origin, DisplayOrigin::HeaderLine);
    assert_eq!(
        active_header.base_face_policy,
        BaseFacePolicy::FixedBasicFace(BasicFaceId::HeaderLineActive)
    );

    let inactive_mode = WindowChromeDisplayText::new(Value::string("mode"), false)
        .fragment(WindowChromeKind::ModeLine);
    assert_eq!(inactive_mode.origin, DisplayOrigin::ModeLine);
    assert_eq!(
        inactive_mode.base_face_policy,
        BaseFacePolicy::FixedBasicFace(BasicFaceId::ModeLineInactive)
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
