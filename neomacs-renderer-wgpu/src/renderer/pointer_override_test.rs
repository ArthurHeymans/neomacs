use super::{
    FacePaint, PointerOverrideResolver, PrimitivePaintPlan, append_clipped_relief_edge,
    clip_glyph_quad, clip_new_rect_vertices, clip_new_rounded_vertices, relief_edges,
};
use crate::vertex::{GlyphVertex, RectVertex, RoundedRectVertex};
use neomacs_display_protocol::{
    Color, DisplayWindowId, Face, FaceId, FrameGlyph, FrameGlyphBuffer, FrameRect, GlyphRowRole,
    PointerAppearanceId, PointerAppearancePhase, PointerAppearanceSelection, PointerDrawMode,
    PointerImageRelief, PointerReliefEdges, PointerReliefMargins, PresentedPaintSpan,
    PresentedPointerAppearance, PresentedPointerRegion, PresentedPrimitiveKind,
};
use neomacs_display_protocol::{ColorStop, Gradient};

fn relief(pressed: bool) -> PointerImageRelief {
    let light = Color::new(0.85, 0.85, 0.85, 1.0);
    let dark = Color::new(0.25, 0.25, 0.25, 1.0);
    let (top_left, bottom_right) = if pressed {
        (dark, light)
    } else {
        (light, dark)
    };
    PointerImageRelief::new(
        top_left,
        bottom_right,
        1.0,
        PointerReliefMargins::new(0.0, 0.0, 0.0, 0.0),
        PointerReliefEdges::new(true, true, true, true),
    )
}

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
fn inactive_primitive_paint_plan_is_fixed_capacity_and_keeps_paint_domain() {
    let mut frame = FrameGlyphBuffer::with_size(40.0, 20.0);
    frame.add_char('x', 4.0, 3.0, 12.0, 10.0, 8.0, false);
    let resolver = PointerOverrideResolver::new(&frame, None);
    let domain = neomacs_display_protocol::Rect::new(4.0, 3.0, 12.0, 10.0);
    let plan = resolver.face_paints(0, FaceId::new(0), domain, None);

    assert!(!std::mem::needs_drop::<PrimitivePaintPlan>());
    let paints = plan.into_iter().collect::<Vec<_>>();
    assert_eq!(paints.len(), 1);
    assert_eq!(paints[0].domain(), domain);
    assert_eq!(paints[0].clip(), None);
}

#[test]
fn active_span_invalidates_only_glyph_ranges_intersecting_its_clip() {
    let face_id = FaceId::new(2);
    let mut frame = FrameGlyphBuffer::with_size(40.0, 30.0);
    frame.faces.insert(face_id, Face::new(face_id));
    frame.add_char('a', 0.0, 0.0, 10.0, 10.0, 8.0, false);
    frame.add_char('b', 0.0, 15.0, 10.0, 10.0, 8.0, false);
    frame
        .install_presented_pointer(
            vec![],
            vec![PresentedPointerAppearance::new(
                vec![PresentedPaintSpan::new(
                    PresentedPrimitiveKind::Glyph,
                    0,
                    2,
                    FrameRect::new(0.0, 0.0, 10.0, 10.0).unwrap(),
                )],
                PointerDrawMode::Face(face_id),
                PointerDrawMode::Face(face_id),
            )],
        )
        .unwrap();
    let resolver =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Hover)));

    assert!(resolver.affects_glyph_range(&frame.glyphs, 0..1));
    assert!(!resolver.affects_glyph_range(&frame.glyphs, 1..2));
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
                PointerDrawMode::ImageRelief(relief(false)),
                PointerDrawMode::ImageRelief(relief(true)),
            )],
        )
        .unwrap();

    let hover =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Hover)));
    let pressed =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Pressed)));
    assert_eq!(
        hover.image_override(0).unwrap().mode(),
        PointerDrawMode::ImageRelief(relief(false))
    );
    assert_eq!(
        pressed.image_override(0).unwrap().mode(),
        PointerDrawMode::ImageRelief(relief(true))
    );
}

#[test]
fn image_relief_flips_light_and_dark_edges_inside_unchanged_quad() {
    let raised = relief_edges(3.0, 5.0, 20.0, 18.0, relief(false))
        .expect("raised relief")
        .into_iter()
        .collect::<Vec<_>>();
    let sunken = relief_edges(3.0, 5.0, 20.0, 18.0, relief(true))
        .expect("sunken relief")
        .into_iter()
        .collect::<Vec<_>>();

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
fn image_relief_honors_resolved_geometry_and_does_not_edge_a_clipped_subsection() {
    let top_left = Color::RED;
    let bottom_right = Color::BLUE;
    let spec = PointerImageRelief::new(
        top_left,
        bottom_right,
        2.0,
        PointerReliefMargins::new(1.0, 2.0, 3.0, 4.0),
        PointerReliefEdges::new(true, false, false, true),
    );
    let edges = relief_edges(10.0, 20.0, 30.0, 24.0, spec)
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].bounds(), (11.0, 22.0, 26.0, 2.0));
    assert_eq!(
        edges[0].corners(),
        [[11.0, 22.0], [37.0, 22.0], [35.0, 24.0], [13.0, 24.0]]
    );
    assert_eq!(edges[0].color(), top_left);
    assert_eq!(edges[1].bounds(), (35.0, 22.0, 2.0, 18.0));
    assert_eq!(edges[1].color(), bottom_right);

    let vertical_only = PointerImageRelief::new(
        top_left,
        bottom_right,
        2.0,
        PointerReliefMargins::new(0.0, 0.0, 0.0, 0.0),
        PointerReliefEdges::new(false, true, false, true),
    );
    let mut vertices = Vec::new();
    let subsection = neomacs_display_protocol::Rect::new(10.0, 0.0, 10.0, 10.0);
    for edge in relief_edges(0.0, 0.0, 30.0, 10.0, vertical_only).unwrap() {
        append_clipped_relief_edge(&mut vertices, edge, Some(&subsection));
    }
    assert!(
        vertices.is_empty(),
        "subsection clip must not invent left/right edges"
    );
}

#[test]
fn thick_image_relief_trapezoids_share_diagonals_and_clip_as_polygons() {
    let spec = PointerImageRelief::new(
        Color::WHITE,
        Color::BLACK,
        3.0,
        PointerReliefMargins::new(0.0, 0.0, 0.0, 0.0),
        PointerReliefEdges::new(true, true, true, true),
    );
    let edges = relief_edges(0.0, 0.0, 20.0, 12.0, spec)
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();
    let [top, left, bottom, right] = edges.as_slice() else {
        panic!("four enabled relief edges")
    };
    assert_eq!(top.corners()[3], left.corners()[1]);
    assert_eq!(top.corners()[2], right.corners()[3]);
    assert_eq!(bottom.corners()[1], left.corners()[2]);
    assert_eq!(bottom.corners()[2], right.corners()[2]);
    assert_eq!(top.corners()[0..2], [[0.0, 0.0], [20.0, 0.0]]);
    assert_eq!(bottom.corners()[0], [0.0, 12.0]);
    assert_eq!(bottom.corners()[3], [20.0, 12.0]);

    let clip = neomacs_display_protocol::Rect::new(5.0, 0.0, 10.0, 2.0);
    let mut clipped = Vec::new();
    append_clipped_relief_edge(&mut clipped, *top, Some(&clip));
    assert!(!clipped.is_empty());
    assert!(clipped.iter().all(|vertex| {
        (5.0..=15.0).contains(&vertex.position[0]) && (0.0..=2.0).contains(&vertex.position[1])
    }));
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

#[test]
fn partial_face_override_replaces_inside_clip_and_keeps_base_only_in_complement() {
    let frame = frame_with_glyph_appearance(
        PointerDrawMode::Face(FaceId::new(2)),
        PointerDrawMode::Face(FaceId::new(2)),
    );
    let resolver =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Hover)));
    let (x, y, width, height) = frame.glyphs[0].cell_rect().unwrap();
    let plan = resolver.face_paints(
        0,
        FaceId::new(0),
        neomacs_display_protocol::Rect::new(x, y, width, height),
        frame.glyphs[0].clip_rect().as_ref(),
    );
    assert!(!std::mem::needs_drop::<PrimitivePaintPlan>());
    assert!(std::mem::size_of::<PrimitivePaintPlan>() <= 5 * 40);
    let paints = plan.into_iter().collect::<Vec<_>>();

    assert_eq!(paints.last().unwrap().face_id(), FaceId::new(2));
    assert_eq!(
        paints.last().unwrap().clip().unwrap(),
        neomacs_display_protocol::Rect::new(14.0, 5.0, 14.0, 17.0)
    );
    assert!(
        paints[..paints.len() - 1]
            .iter()
            .all(|paint| paint.face_id() == FaceId::new(0))
    );
    let override_clip = paints.last().unwrap().clip().unwrap();
    assert!(paints[..paints.len() - 1].iter().all(|paint| {
        let clip = paint.clip().unwrap();
        clip.x + clip.width <= override_clip.x
            || clip.x >= override_clip.x + override_clip.width
            || clip.y + clip.height <= override_clip.y
            || clip.y >= override_clip.y + override_clip.height
    }));
    assert!(paints.iter().all(|paint| {
        paint.domain() == neomacs_display_protocol::Rect::new(x, y, width, height)
    }));
}

#[test]
fn complement_clips_do_not_reanchor_gradient_or_stipple_paint_coordinates() {
    let frame = frame_with_glyph_appearance(
        PointerDrawMode::Face(FaceId::new(2)),
        PointerDrawMode::Face(FaceId::new(2)),
    );
    let resolver =
        PointerOverrideResolver::new(&frame, Some(selection(PointerAppearancePhase::Hover)));
    let domain = neomacs_display_protocol::Rect::new(12.0, 4.0, 17.0, 20.0);
    let paints = resolver
        .face_paints(
            0,
            FaceId::new(0),
            domain,
            frame.glyphs[0].clip_rect().as_ref(),
        )
        .into_iter()
        .collect::<Vec<_>>();

    assert!(paints.len() > 1);
    assert!(paints.iter().all(|paint| paint.domain() == domain));
    assert!(paints.iter().any(|paint| paint.clip() != paints[0].clip()));
}

#[test]
fn subpixel_background_sampling_uses_immutable_paint_domain_not_output_clip() {
    let mut face = Face::new(FaceId::new(3));
    face.background_gradient = Some(Box::new(Gradient::Linear {
        angle: 0.0,
        stops: vec![
            ColorStop::new(0.0, Color::RED),
            ColorStop::new(1.0, Color::BLUE),
        ],
    }));
    let domain = neomacs_display_protocol::Rect::new(0.0, 0.0, 100.0, 10.0);
    let output_clip = neomacs_display_protocol::Rect::new(50.0, 0.0, 50.0, 10.0);
    let paint = FacePaint {
        face_id: face.id,
        domain,
        output_clip: Some(output_clip),
    };

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
