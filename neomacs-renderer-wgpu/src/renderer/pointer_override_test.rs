use super::{
    PointerOverrideResolver, clip_glyph_quad, clip_new_rect_vertices, clip_new_rounded_vertices,
    relief_edges,
};
use crate::vertex::{GlyphVertex, RectVertex, RoundedRectVertex};
use neomacs_display_protocol::{
    Color, DisplayWindowId, Face, FaceId, FrameGlyph, FrameGlyphBuffer, FrameRect, GlyphRowRole,
    PointerAppearanceId, PointerAppearancePhase, PointerAppearanceSelection, PointerDrawMode,
    PresentedPaintSpan, PresentedPointerAppearance, PresentedPointerRegion, PresentedPrimitiveKind,
};

fn frame_with_glyph_appearance(
    hover: PointerDrawMode,
    pressed: PointerDrawMode,
) -> FrameGlyphBuffer {
    let mut frame = FrameGlyphBuffer::with_size(100.0, 30.0);
    frame
        .faces
        .insert(FaceId::new(0), Face::new(FaceId::new(0)));
    frame
        .faces
        .insert(FaceId::new(1), Face::new(FaceId::new(1)));
    let mut alternate = Face::new(FaceId::new(2));
    alternate.foreground = Color::RED;
    alternate.font_size = 24.0;
    frame.faces.insert(FaceId::new(2), alternate);
    frame.set_draw_context(
        DisplayWindowId::new(8),
        GlyphRowRole::Text,
        Some(neomacs_display_protocol::Rect::new(14.0, 2.0, 14.0, 20.0)),
    );
    frame.add_char('x', 12.0, 4.0, 17.0, 20.0, 15.0, false);
    frame
        .install_presented_pointer(
            vec![PresentedPointerRegion::new(
                FrameRect::new(12.0, 4.0, 17.0, 20.0).unwrap(),
                None,
                Some(PointerAppearanceId::try_from(0usize).unwrap()),
            )],
            vec![PresentedPointerAppearance::new(
                vec![PresentedPaintSpan::new(
                    PresentedPrimitiveKind::Glyph,
                    0,
                    1,
                    FrameRect::new(13.0, 5.0, 15.0, 18.0).unwrap(),
                )],
                hover,
                pressed,
            )],
        )
        .unwrap();
    frame
}

fn selection(phase: PointerAppearancePhase) -> PointerAppearanceSelection {
    PointerAppearanceSelection::new(PointerAppearanceId::try_from(0usize).unwrap(), phase)
}

#[test]
fn pointer_override_selects_hover_and_pressed_draw_modes() {
    let frame = frame_with_glyph_appearance(
        PointerDrawMode::Face(FaceId::new(1)),
        PointerDrawMode::Face(FaceId::new(2)),
    );

    let hover =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Hover)));
    let pressed =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Pressed)));

    assert_eq!(
        hover.glyph_override(0).unwrap().mode(),
        PointerDrawMode::Face(FaceId::new(1))
    );
    assert_eq!(
        pressed.glyph_override(0).unwrap().mode(),
        PointerDrawMode::Face(FaceId::new(2))
    );
}

#[test]
fn face_override_changes_materialized_face_without_changing_glyph_geometry() {
    let frame = frame_with_glyph_appearance(
        PointerDrawMode::Face(FaceId::new(2)),
        PointerDrawMode::Face(FaceId::new(2)),
    );
    let resolver =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Hover)));
    let original = &frame.glyphs[0];
    let resolved = resolver.resolve_glyph(&frame, 0).expect("glyph");

    assert_eq!(resolved.face_id(), Some(FaceId::new(2)));
    assert_eq!(resolved.materialized_face().font_size, 24.0);
    assert!(std::ptr::eq(resolved.primitive(), original));
    assert_eq!(resolved.primitive().cell_rect(), original.cell_rect());
    assert_eq!(resolved.primitive().geometry(), original.geometry());
    assert_eq!(resolved.primitive().clip_rect(), original.clip_rect());
    assert_eq!(resolved.primitive().row_role(), original.row_role());
    assert_eq!(resolved.primitive().window_id(), original.window_id());
    assert_eq!(resolved.primitive().slot_id(), original.slot_id());
    assert_eq!(
        resolved.clip(),
        FrameRect::new(14.0, 5.0, 14.0, 17.0).unwrap()
    );
    let FrameGlyph::Char {
        x,
        y,
        baseline,
        width,
        height,
        ascent,
        ..
    } = resolved.primitive()
    else {
        panic!("char")
    };
    assert_eq!(
        (*x, *y, *baseline, *width, *height, *ascent),
        (12.0, 4.0, 19.0, 17.0, 20.0, 15.0)
    );
}

#[test]
fn image_override_selects_raised_and_sunken_relief() {
    let mut frame = FrameGlyphBuffer::with_size(40.0, 40.0);
    frame.add_image(
        neomacs_display_protocol::ImageId::new(4),
        3.0,
        5.0,
        20.0,
        18.0,
    );
    frame
        .install_presented_pointer(
            vec![PresentedPointerRegion::new(
                FrameRect::new(3.0, 5.0, 20.0, 18.0).unwrap(),
                None,
                Some(PointerAppearanceId::try_from(0usize).unwrap()),
            )],
            vec![PresentedPointerAppearance::new(
                vec![PresentedPaintSpan::new(
                    PresentedPrimitiveKind::Image,
                    0,
                    1,
                    FrameRect::new(3.0, 5.0, 20.0, 18.0).unwrap(),
                )],
                PointerDrawMode::ImageRaised,
                PointerDrawMode::ImageSunken,
            )],
        )
        .unwrap();

    let hover =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Hover)));
    let pressed =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Pressed)));
    assert_eq!(
        hover.image_override(0).unwrap().mode(),
        PointerDrawMode::ImageRaised
    );
    assert_eq!(
        pressed.image_override(0).unwrap().mode(),
        PointerDrawMode::ImageSunken
    );
}

#[test]
fn image_relief_flips_light_and_dark_edges_inside_unchanged_quad() {
    let raised =
        relief_edges(3.0, 5.0, 20.0, 18.0, PointerDrawMode::ImageRaised).expect("raised relief");
    let sunken =
        relief_edges(3.0, 5.0, 20.0, 18.0, PointerDrawMode::ImageSunken).expect("sunken relief");

    assert_eq!(raised[0].bounds(), (3.0, 5.0, 20.0, 1.0)); // top
    assert_eq!(raised[1].bounds(), (3.0, 5.0, 1.0, 18.0)); // left
    assert_eq!(raised[2].bounds(), (3.0, 22.0, 20.0, 1.0)); // bottom
    assert_eq!(raised[3].bounds(), (22.0, 5.0, 1.0, 18.0)); // right
    assert_eq!(raised[0].color(), raised[1].color());
    assert_eq!(raised[2].color(), raised[3].color());
    assert_ne!(raised[0].color(), raised[2].color());
    assert_eq!(sunken[0].color(), raised[2].color());
    assert_eq!(sunken[2].color(), raised[0].color());

    let before = super::super::layer_media::textured_quad_vertices(3.0, 5.0, 20.0, 18.0, 0.0, 1.0);
    let after = super::super::layer_media::textured_quad_vertices(3.0, 5.0, 20.0, 18.0, 0.0, 1.0);
    assert_eq!(
        before.map(|vertex| (vertex.position, vertex.tex_coords)),
        after.map(|vertex| (vertex.position, vertex.tex_coords)),
        "relief never changes the image quad",
    );
}

#[test]
fn partial_box_clip_contains_sharp_and_rounded_vertices() {
    let clip = neomacs_display_protocol::Rect::new(12.0, 7.0, 6.0, 5.0);
    let color = [1.0; 4];
    let mut sharp = vec![
        RectVertex {
            position: [10.0, 5.0],
            color,
        },
        RectVertex {
            position: [20.0, 5.0],
            color,
        },
        RectVertex {
            position: [20.0, 15.0],
            color,
        },
        RectVertex {
            position: [10.0, 5.0],
            color,
        },
        RectVertex {
            position: [20.0, 15.0],
            color,
        },
        RectVertex {
            position: [10.0, 15.0],
            color,
        },
    ];
    clip_new_rect_vertices(&mut sharp, 0, Some(&clip));
    assert!(sharp.iter().all(|v| (12.0..=18.0).contains(&v.position[0])));
    assert!(sharp.iter().all(|v| (7.0..=12.0).contains(&v.position[1])));

    let template = RoundedRectVertex {
        position: [0.0, 0.0],
        color,
        rect_min: [10.0, 5.0],
        rect_max: [20.0, 15.0],
        params: [1.0, 3.0],
        style_params: [0.0; 4],
        color2: color,
    };
    let mut rounded = [
        [10.0, 5.0],
        [20.0, 5.0],
        [20.0, 15.0],
        [10.0, 5.0],
        [20.0, 15.0],
        [10.0, 15.0],
    ]
    .map(|position| RoundedRectVertex {
        position,
        ..template
    })
    .to_vec();
    clip_new_rounded_vertices(&mut rounded, 0, Some(&clip));
    assert!(
        rounded
            .iter()
            .all(|v| (12.0..=18.0).contains(&v.position[0]))
    );
    assert!(
        rounded
            .iter()
            .all(|v| (7.0..=12.0).contains(&v.position[1]))
    );
    assert!(
        rounded
            .iter()
            .all(|v| v.rect_min == [10.0, 5.0] && v.rect_max == [20.0, 15.0])
    );
}

#[test]
fn shifted_overstrike_is_reclipped_with_uv_interpolation() {
    let color = [1.0; 4];
    let quad = [
        ([10.0, 5.0], [0.0, 0.0]),
        ([20.0, 5.0], [1.0, 0.0]),
        ([20.0, 15.0], [1.0, 1.0]),
        ([10.0, 5.0], [0.0, 0.0]),
        ([20.0, 15.0], [1.0, 1.0]),
        ([10.0, 15.0], [0.0, 1.0]),
    ]
    .map(|(position, tex_coords)| GlyphVertex {
        position,
        tex_coords,
        color,
    });
    let clip = neomacs_display_protocol::Rect::new(12.0, 5.0, 6.0, 10.0);
    let base = clip_glyph_quad(quad, Some(&clip)).expect("base clipped");
    let shifted = base.map(|mut vertex| {
        vertex.position[0] += 1.0;
        vertex
    });
    let reclipped = clip_glyph_quad(shifted, Some(&clip)).expect("overstrike clipped");

    assert!(reclipped.iter().all(|v| v.position[0] <= 18.0));
    assert!(reclipped.iter().any(|v| v.tex_coords[0] < 1.0));
}
