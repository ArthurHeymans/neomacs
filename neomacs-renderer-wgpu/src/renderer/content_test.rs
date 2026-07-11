// The previous `window_cursor_visual_match_uses_slot_identity` test covered the
// phys/visual dedup helper, which no longer exists: cursors are unified into a
// single per-window list (the selected window's entry is `active`), so the
// content backend draws every entry without deduplicating against a separate
// phys_cursor. There is nothing backend-specific left to assert here.

use super::stretch_decoration_rects;
use neomacs_display_protocol::face::{Face, FaceAttributes, UnderlineStyle};
use neomacs_display_protocol::{Color, FaceId};

#[test]
fn child_stretch_decorations_follow_the_effective_face() {
    let mut face = Face::new(FaceId::new(4));
    face.attributes =
        FaceAttributes::UNDERLINE | FaceAttributes::OVERLINE | FaceAttributes::STRIKE_THROUGH;
    face.underline_style = UnderlineStyle::Double;
    face.foreground = Color::RED;
    face.font_ascent = 8;
    face.underline_position = 1;
    face.underline_thickness = 1;

    let rects = stretch_decoration_rects(&face, 10.0, 5.0, 20.0);

    assert_eq!(rects.len(), 4, "double underline plus overline and strike");
    assert!(
        rects
            .iter()
            .all(|rect| rect.x >= 10.0 && rect.x + rect.width <= 30.0)
    );
    assert!(rects.iter().all(|rect| rect.color == Color::RED));
}
