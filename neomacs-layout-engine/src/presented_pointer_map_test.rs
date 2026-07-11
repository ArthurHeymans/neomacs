use neomacs_display_protocol::{
    Color, FaceId, FrameGlyphBuffer, FrameRect, InteractionId, PointerDrawMode, PresentedPaintSpan,
    PresentedPointerMapError, PresentedPrimitiveKind,
};

use super::{
    PointerAppearanceRangeId, PresentedPointerMapBuildError, PresentedPointerMapBuilder,
    RenderedPointerAppearance, RenderedPointerRun,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> FrameRect {
    FrameRect::new(x, y, width, height).expect("valid test rectangle")
}

fn glyph_observation(
    identity: u64,
    face_id: FaceId,
    paint_span: PresentedPaintSpan,
) -> RenderedPointerAppearance {
    RenderedPointerAppearance::new(
        PointerAppearanceRangeId::new(identity),
        paint_span,
        PointerDrawMode::Face(face_id),
        PointerDrawMode::Face(face_id),
    )
}

fn frame_with_glyphs(face_id: FaceId, glyph_count: usize) -> FrameGlyphBuffer {
    let mut frame = FrameGlyphBuffer::with_size(100.0, 50.0);
    frame.set_face(
        face_id,
        Color::WHITE,
        None,
        400,
        false,
        0,
        None,
        0,
        None,
        0,
        None,
    );
    for index in 0..glyph_count {
        frame.add_char('a', index as f32 * 10.0, 0.0, 10.0, 10.0, 8.0, false);
    }
    frame
}

#[test]
fn presented_pointer_map_coalesces_adjacent_equivalent_regions() {
    let mut builder = PresentedPointerMapBuilder::new();
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(0.0, 0.0, 10.0, 10.0),
        Some(InteractionId::new(7)),
        None,
    ));
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(10.0, 0.0, 15.0, 10.0),
        Some(InteractionId::new(7)),
        None,
    ));
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(30.0, 0.0, 5.0, 10.0),
        Some(InteractionId::new(7)),
        None,
    ));

    let mut frame = FrameGlyphBuffer::with_size(100.0, 50.0);
    builder
        .finish_into(&mut frame)
        .expect("valid pointer metadata");

    assert_eq!(frame.presented_pointer().regions().len(), 2);
    assert_eq!(
        frame.presented_pointer().regions()[0].bounds(),
        rect(0.0, 0.0, 25.0, 10.0)
    );
}

#[test]
fn presented_pointer_map_keeps_first_seen_appearance_order_and_identity_boundaries() {
    let face_id = FaceId::new(8);
    let mut builder = PresentedPointerMapBuilder::new();
    for (identity, first, x) in [(900, 0, 0.0), (100, 1, 10.0)] {
        builder.observe_rendered_run(RenderedPointerRun::new(
            rect(x, 0.0, 10.0, 10.0),
            Some(InteractionId::new(7)),
            Some(glyph_observation(
                identity,
                face_id,
                PresentedPaintSpan::new(
                    PresentedPrimitiveKind::Glyph,
                    first,
                    1,
                    rect(x, 0.0, 10.0, 10.0),
                ),
            )),
        ));
    }

    let mut frame = frame_with_glyphs(face_id, 2);
    builder
        .finish_into(&mut frame)
        .expect("distinct appearance ranges are valid");

    let map = frame.presented_pointer();
    assert_eq!(map.regions().len(), 2);
    assert_eq!(map.regions()[0].appearance().expect("first range").get(), 0);
    assert_eq!(
        map.regions()[1].appearance().expect("second range").get(),
        1
    );
    assert_eq!(map.appearances()[0].paint_spans()[0].first(), 0);
    assert_eq!(map.appearances()[1].paint_spans()[0].first(), 1);
}

#[test]
fn presented_pointer_map_deduplicates_appearance_shared_by_distinct_interactions() {
    let face_id = FaceId::new(9);
    let mut builder = PresentedPointerMapBuilder::new();
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(0.0, 0.0, 15.0, 10.0),
        Some(InteractionId::new(10)),
        Some(glyph_observation(
            50,
            face_id,
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                0,
                1,
                rect(0.0, 0.0, 15.0, 10.0),
            ),
        )),
    ));
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(15.0, 0.0, 5.0, 10.0),
        Some(InteractionId::new(11)),
        Some(glyph_observation(
            50,
            face_id,
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                1,
                1,
                rect(15.0, 0.0, 5.0, 10.0),
            ),
        )),
    ));

    let mut frame = frame_with_glyphs(face_id, 2);
    builder
        .finish_into(&mut frame)
        .expect("valid pointer metadata");

    let map = frame.presented_pointer();
    assert_eq!(map.appearances().len(), 1);
    assert_eq!(map.appearances()[0].paint_spans().len(), 2);
    assert_eq!(map.regions().len(), 2);
    assert_eq!(map.regions()[0].appearance(), map.regions()[1].appearance());
    assert_ne!(
        map.regions()[0].interaction(),
        map.regions()[1].interaction()
    );
}

#[test]
fn presented_pointer_map_keeps_wrapped_paint_spans_in_one_appearance() {
    let face_id = FaceId::new(12);
    let mut builder = PresentedPointerMapBuilder::new();
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(0.0, 0.0, 10.0, 10.0),
        Some(InteractionId::new(20)),
        Some(glyph_observation(
            100,
            face_id,
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                0,
                1,
                rect(0.0, 0.0, 10.0, 10.0),
            ),
        )),
    ));
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(0.0, 10.0, 10.0, 10.0),
        Some(InteractionId::new(20)),
        Some(glyph_observation(
            100,
            face_id,
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                1,
                1,
                rect(0.0, 10.0, 10.0, 10.0),
            ),
        )),
    ));

    let mut frame = frame_with_glyphs(face_id, 2);
    builder
        .finish_into(&mut frame)
        .expect("valid wrapped pointer appearance");

    let appearances = frame.presented_pointer().appearances();
    assert_eq!(appearances.len(), 1);
    assert_eq!(frame.presented_pointer().regions().len(), 2);
    assert_eq!(appearances[0].paint_spans().len(), 2);
    assert_eq!(appearances[0].paint_spans()[0].first(), 0);
    assert_eq!(appearances[0].paint_spans()[1].first(), 1);
}

#[test]
fn presented_pointer_map_rejects_conflicting_modes_for_one_appearance_range() {
    let identity = PointerAppearanceRangeId::new(101);
    let face_id = FaceId::new(12);
    let mut builder = PresentedPointerMapBuilder::new();
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(0.0, 0.0, 10.0, 10.0),
        None,
        Some(RenderedPointerAppearance::new(
            identity,
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                0,
                1,
                rect(0.0, 0.0, 10.0, 10.0),
            ),
            PointerDrawMode::Face(face_id),
            PointerDrawMode::Face(face_id),
        )),
    ));
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(0.0, 10.0, 10.0, 10.0),
        None,
        Some(RenderedPointerAppearance::new(
            identity,
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                1,
                1,
                rect(0.0, 10.0, 10.0, 10.0),
            ),
            PointerDrawMode::Face(FaceId::new(13)),
            PointerDrawMode::Face(FaceId::new(14)),
        )),
    ));

    let mut frame = frame_with_glyphs(face_id, 2);
    let error = builder
        .finish_into(&mut frame)
        .expect_err("one semantic appearance range must have stable draw modes");

    assert_eq!(
        error,
        PresentedPointerMapBuildError::ConflictingAppearanceModes(identity)
    );
    assert!(frame.presented_pointer().is_empty());
}

#[test]
fn presented_pointer_map_leaves_bounds_validation_to_the_install_seam() {
    let mut builder = PresentedPointerMapBuilder::new();
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(95.0, 0.0, 10.0, 10.0),
        Some(InteractionId::new(30)),
        None,
    ));

    let mut frame = FrameGlyphBuffer::with_size(100.0, 50.0);
    let error = builder
        .finish_into(&mut frame)
        .expect_err("region extending beyond the frame must be rejected");

    assert_eq!(
        error,
        PresentedPointerMapBuildError::Protocol(PresentedPointerMapError::RegionOutsideFrame)
    );
    assert!(frame.presented_pointer().is_empty());
}

#[test]
fn presented_pointer_map_publishes_unresolved_visuals_as_click_only_regions() {
    let mut builder = PresentedPointerMapBuilder::new();
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(5.0, 5.0, 10.0, 10.0),
        Some(InteractionId::new(40)),
        None,
    ));

    let mut frame = FrameGlyphBuffer::with_size(100.0, 50.0);
    builder
        .finish_into(&mut frame)
        .expect("click-only pointer metadata");

    let region = &frame.presented_pointer().regions()[0];
    assert_eq!(region.interaction(), Some(InteractionId::new(40)));
    assert_eq!(region.appearance(), None);
    assert!(frame.presented_pointer().appearances().is_empty());
}

#[test]
fn presented_pointer_map_publishes_visual_only_runs() {
    let face_id = FaceId::new(15);
    let mut builder = PresentedPointerMapBuilder::new();
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(0.0, 0.0, 10.0, 10.0),
        None,
        Some(glyph_observation(
            200,
            face_id,
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                0,
                1,
                rect(0.0, 0.0, 10.0, 10.0),
            ),
        )),
    ));

    let mut frame = frame_with_glyphs(face_id, 1);
    builder
        .finish_into(&mut frame)
        .expect("visual-only pointer metadata");

    let region = &frame.presented_pointer().regions()[0];
    assert_eq!(region.interaction(), None);
    assert!(region.appearance().is_some());
}

#[test]
fn presented_pointer_map_skips_runs_without_pointer_behavior() {
    let mut builder = PresentedPointerMapBuilder::new();
    builder.observe_rendered_run(RenderedPointerRun::new(
        rect(0.0, 0.0, 10.0, 10.0),
        None,
        None,
    ));

    let mut frame = FrameGlyphBuffer::with_size(100.0, 50.0);
    builder
        .finish_into(&mut frame)
        .expect("ordinary run produces an empty valid map");

    assert!(frame.presented_pointer().is_empty());
}
