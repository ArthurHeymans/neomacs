// The previous `window_cursor_visual_match_uses_slot_identity` test covered the
// phys/visual dedup helper, which no longer exists: cursors are unified into a
// single per-window list (the selected window's entry is `active`), so the
// content backend draws every entry without deduplicating against a separate
// phys_cursor. There is nothing backend-specific left to assert here.

use super::stretch_decoration_rects;
use neomacs_display_protocol::face::{Face, FaceAttributes, UnderlineStyle};
use neomacs_display_protocol::{Color, ColorStop, FaceId, Gradient, Rect};

#[test]
fn child_subpixel_gradient_sampling_uses_face_paint_domain() {
    let mut face = Face::new(FaceId::new(3));
    face.background_gradient = Some(Box::new(Gradient::Linear {
        angle: 0.0,
        stops: vec![
            ColorStop::new(0.0, Color::RED),
            ColorStop::new(1.0, Color::BLUE),
        ],
    }));
    let domain = Rect::new(0.0, 0.0, 100.0, 10.0);
    let output_clip = Rect::new(50.0, 0.0, 50.0, 10.0);
    let paint = super::super::pointer_override::FacePaint::new(face.id, domain, Some(output_clip));

    let sampled =
        super::super::WgpuRenderer::sample_face_paint_background(Some(&face), None, paint);
    let domain_sample = super::super::WgpuRenderer::sample_face_background(
        Some(&face),
        None,
        domain.x,
        domain.y,
        domain.width,
        domain.height,
        None,
    );
    let reanchored = super::super::WgpuRenderer::sample_face_background(
        Some(&face),
        None,
        domain.x,
        domain.y,
        domain.width,
        domain.height,
        Some(&output_clip),
    );

    assert_eq!(sampled, domain_sample);
    assert_ne!(sampled, reanchored);
}

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
