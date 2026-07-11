use crate::{
    Color, FaceId, FrameRect, InteractionId, PointerAppearanceId, PointerAppearancePhase,
    PointerAppearanceSelection, PointerDrawMode, PointerImageRelief, PointerReliefCornerErase,
    PointerReliefEdges, PointerReliefMargins, PresentedPaintSpan, PresentedPointerAppearance,
    PresentedPointerMap, PresentedPointerMapError, PresentedPointerRegion,
    PresentedPointerSourceAppearance, PresentedPointerSourceMap, PresentedPrimitiveKind,
    PresentedSourcePaintSpan,
};

#[test]
fn pointer_appearance_selection_carries_only_renderer_safe_identity_and_phase() {
    let id = PointerAppearanceId::try_from(3usize).expect("small id");
    let selection = PointerAppearanceSelection::new(id, PointerAppearancePhase::Pressed);

    assert_eq!(selection.appearance(), id);
    assert_eq!(selection.phase(), PointerAppearancePhase::Pressed);
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> FrameRect {
    FrameRect::new(x, y, width, height).expect("valid test rectangle")
}

fn image_relief_mode(pressed: bool) -> PointerDrawMode {
    let light = Color::new(0.85, 0.85, 0.85, 1.0);
    let dark = Color::new(0.25, 0.25, 0.25, 1.0);
    let (top_left, bottom_right) = if pressed {
        (dark, light)
    } else {
        (light, dark)
    };
    PointerDrawMode::ImageRelief(PointerImageRelief::new(
        top_left,
        bottom_right,
        1.0,
        PointerReliefMargins::new(0.0, 0.0, 0.0, 0.0),
        PointerReliefEdges::new(true, true, true, true),
        PointerReliefCornerErase::new(Color::rgb(0.1, 0.2, 0.3), 6.0, 1.0),
    ))
}

#[test]
fn pointer_image_relief_carries_only_resolved_renderer_parameters() {
    let top_left = Color::new(0.9, 0.8, 0.7, 1.0);
    let bottom_right = Color::new(0.2, 0.3, 0.4, 1.0);
    let margins = PointerReliefMargins::new(2.0, 3.0, 4.0, 5.0);
    let edges = PointerReliefEdges::new(true, false, true, false);
    let corner_erase = PointerReliefCornerErase::new(Color::BLUE, 6.0, 1.0);
    let relief = PointerImageRelief::new(top_left, bottom_right, 2.5, margins, edges, corner_erase);

    assert_eq!(relief.top_left_color(), top_left);
    assert_eq!(relief.bottom_right_color(), bottom_right);
    assert_eq!(relief.thickness(), 2.5);
    assert_eq!(relief.margins(), margins);
    assert_eq!(relief.edges(), edges);
    assert_eq!(relief.corner_erase(), corner_erase);
    assert!(edges.top());
    assert!(!edges.left());
    assert!(edges.bottom());
    assert!(!edges.right());

    let mode = PointerDrawMode::ImageRelief(relief);
    let decoded: PointerDrawMode =
        serde_json::from_str(&serde_json::to_string(&mode).unwrap()).unwrap();
    assert_eq!(decoded, mode);
}

#[test]
fn presented_pointer_map_rejects_non_renderer_safe_relief_geometry() {
    let invalid = PointerDrawMode::ImageRelief(PointerImageRelief::new(
        Color::WHITE,
        Color::BLACK,
        f32::NAN,
        PointerReliefMargins::new(0.0, 0.0, 0.0, 0.0),
        PointerReliefEdges::new(true, true, true, true),
        PointerReliefCornerErase::new(Color::WHITE, 6.0, 1.0),
    ));
    let appearance = PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Image,
            8,
            1,
            rect(8.0, 0.0, 1.0, 10.0),
        )],
        invalid,
        invalid,
    );

    assert_eq!(
        try_map(&[], vec![], vec![appearance]),
        Err(PresentedPointerMapError::InvalidImageRelief)
    );
}

#[test]
fn presented_pointer_map_accepts_zero_thickness_image_relief() {
    let zero = PointerDrawMode::ImageRelief(PointerImageRelief::new(
        Color::WHITE,
        Color::BLACK,
        0.0,
        PointerReliefMargins::new(0.0, 0.0, 0.0, 0.0),
        PointerReliefEdges::new(true, true, true, true),
        PointerReliefCornerErase::new(Color::WHITE, 6.0, 1.0),
    ));
    let appearance = PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Image,
            8,
            1,
            rect(8.0, 0.0, 1.0, 10.0),
        )],
        zero,
        zero,
    );

    assert!(try_map(&[], vec![], vec![appearance]).is_ok());
}

#[test]
fn source_pointer_map_disambiguates_equal_chrome_slots_by_row_role() {
    let face_id = FaceId::new(7);
    let mut frame = crate::FrameGlyphBuffer::with_size(40.0, 20.0);
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
    let slot = crate::DisplaySlotId {
        window_id: crate::DisplayWindowId::new(0),
        row: 0,
        col: 0,
    };
    let glyph = |row_role| crate::FrameGlyph::Char {
        window_id: crate::DisplayWindowId::new(0),
        row_role,
        clip_rect: None,
        slot_id: slot,
        bidi_level: 0,
        char: 'x',
        composed: None,
        x: 0.0,
        y: 0.0,
        baseline: 8.0,
        width: 8.0,
        height: 10.0,
        ascent: 8.0,
        face_id,
    };
    frame.glyphs.push(glyph(crate::GlyphRowRole::Text));
    frame.glyphs.push(glyph(crate::GlyphRowRole::TabBar));
    let source = PresentedPointerSourceMap::new(
        vec![PresentedPointerRegion::new(
            rect(0.0, 0.0, 8.0, 10.0),
            Some(InteractionId::new(1)),
            Some(PointerAppearanceId::try_from(0usize).unwrap()),
        )],
        vec![PresentedPointerSourceAppearance::new(
            vec![PresentedSourcePaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                crate::GlyphRowRole::TabBar,
                slot,
                rect(0.0, 0.0, 8.0, 10.0),
            )],
            PointerDrawMode::Face(face_id),
            PointerDrawMode::Face(face_id),
        )],
    );

    frame.install_presented_pointer_source_map(&source).unwrap();

    assert_eq!(
        frame.presented_pointer().appearances()[0].paint_spans()[0].first(),
        1
    );
}

fn try_map(
    valid_face_ids: &[FaceId],
    regions: Vec<PresentedPointerRegion>,
    appearances: Vec<PresentedPointerAppearance>,
) -> Result<PresentedPointerMap, PresentedPointerMapError> {
    let mut buffer = crate::FrameGlyphBuffer::with_size(100.0, 50.0);
    for index in 0..8 {
        buffer.add_char('a', index as f32, 0.0, 1.0, 10.0, 8.0, false);
    }
    for index in 0..2 {
        buffer.add_image(
            crate::ImageId::new(index),
            8.0 + index as f32,
            0.0,
            1.0,
            10.0,
        );
    }
    for face_id in valid_face_ids {
        buffer.faces.insert(*face_id, crate::Face::new(*face_id));
    }
    let context =
        crate::presented_pointer::PointerMapValidationContext::from_frame_buffer(&buffer)?;
    let map = PresentedPointerMap::from_parts(regions, appearances)?;
    map.validate_against(context)?;
    Ok(map)
}

fn appearance(face_id: FaceId) -> PresentedPointerAppearance {
    PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Glyph,
            1,
            3,
            rect(0.0, 0.0, 60.0, 20.0),
        )],
        PointerDrawMode::Face(face_id),
        PointerDrawMode::Face(face_id),
    )
}

#[test]
fn presented_pointer_map_rejects_overlapping_paint_spans() {
    let face_id = FaceId::new(7);
    let overlapping = PresentedPointerAppearance::new(
        vec![
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                1,
                3,
                rect(0.0, 0.0, 20.0, 10.0),
            ),
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                3,
                2,
                rect(20.0, 0.0, 20.0, 10.0),
            ),
        ],
        PointerDrawMode::Face(face_id),
        PointerDrawMode::Face(face_id),
    );

    assert_eq!(
        try_map(&[face_id], vec![], vec![overlapping]),
        Err(PresentedPointerMapError::OverlappingPaintSpans)
    );
}

#[test]
fn presented_pointer_regions_keep_click_meaning_separate_from_shared_appearance() {
    let face_id = FaceId::new(7);
    let appearance_id = PointerAppearanceId::try_from(0usize).expect("representable id");
    let map = try_map(
        &[face_id],
        vec![
            PresentedPointerRegion::new(
                rect(0.0, 0.0, 50.0, 20.0),
                Some(InteractionId::new(10)),
                Some(appearance_id),
            ),
            PresentedPointerRegion::new(
                rect(50.0, 0.0, 10.0, 20.0),
                Some(InteractionId::new(11)),
                Some(appearance_id),
            ),
        ],
        vec![appearance(face_id)],
    )
    .expect("valid pointer map");

    let body = map.hit_test(25.0, 10.0).expect("tab body hit");
    let close = map.hit_test(55.0, 10.0).expect("tab close hit");

    assert_eq!(body.interaction(), Some(InteractionId::new(10)));
    assert_eq!(close.interaction(), Some(InteractionId::new(11)));
    assert_eq!(body.appearance(), close.appearance());
    assert_eq!(
        map.appearance(body.appearance().expect("body appearance")),
        Some(&appearance(face_id))
    );
}

#[test]
fn presented_pointer_region_can_publish_click_meaning_without_an_appearance() {
    let map = try_map(
        &[],
        vec![PresentedPointerRegion::new(
            rect(0.0, 0.0, 10.0, 10.0),
            Some(InteractionId::new(12)),
            None,
        )],
        vec![],
    )
    .expect("click-only region needs no appearance table entry");

    let json = serde_json::to_string(&map).expect("serialize click-only map");
    let decoded: PresentedPointerMap =
        serde_json::from_str(&json).expect("deserialize click-only map");
    let hit = decoded.hit_test(5.0, 5.0).expect("click-only region hit");

    assert_eq!(hit.interaction(), Some(InteractionId::new(12)));
    assert_eq!(hit.appearance(), None);
    assert!(decoded.appearances().is_empty());
}

#[test]
fn presented_pointer_region_can_publish_visual_appearance_without_click_meaning() {
    let face_id = FaceId::new(7);
    let appearance_id = PointerAppearanceId::try_from(0usize).expect("representable id");
    let map = try_map(
        &[face_id],
        vec![PresentedPointerRegion::new(
            rect(0.0, 0.0, 10.0, 10.0),
            None,
            Some(appearance_id),
        )],
        vec![appearance(face_id)],
    )
    .expect("visual-only region is valid");

    let json = serde_json::to_string(&map).expect("serialize visual-only map");
    let decoded: PresentedPointerMap =
        serde_json::from_str(&json).expect("deserialize visual-only map");
    let hit = decoded.hit_test(5.0, 5.0).expect("visual-only region hit");

    assert_eq!(hit.interaction(), None);
    assert_eq!(hit.appearance(), Some(appearance_id));
}

#[test]
fn presented_pointer_region_rejects_neither_click_nor_visual_meaning() {
    let error = try_map(
        &[],
        vec![PresentedPointerRegion::new(
            rect(0.0, 0.0, 10.0, 10.0),
            None,
            None,
        )],
        vec![],
    )
    .expect_err("a region with no pointer behavior is invalid");

    assert_eq!(error, PresentedPointerMapError::MissingRegionBehavior);
}

#[test]
fn presented_pointer_map_rejects_unknown_appearance_references() {
    let error = try_map(
        &[],
        vec![PresentedPointerRegion::new(
            rect(0.0, 0.0, 10.0, 10.0),
            Some(InteractionId::new(1)),
            Some(PointerAppearanceId::try_from(3usize).expect("representable id")),
        )],
        vec![],
    )
    .expect_err("appearance index is not present");

    assert_eq!(
        error,
        PresentedPointerMapError::UnknownAppearance(PointerAppearanceId::try_from(3usize).unwrap())
    );
}

#[test]
fn presented_pointer_map_rejects_spans_outside_the_matching_primitive_table() {
    let face_id = FaceId::new(7);
    for (kind, first, len) in [
        (PresentedPrimitiveKind::Glyph, 10, 1),
        (PresentedPrimitiveKind::Image, 9, 2),
        (PresentedPrimitiveKind::Glyph, u32::MAX, 2),
    ] {
        let invalid = PresentedPointerAppearance::new(
            vec![PresentedPaintSpan::new(
                kind,
                first,
                len,
                rect(0.0, 0.0, 10.0, 10.0),
            )],
            PointerDrawMode::Face(face_id),
            PointerDrawMode::Face(face_id),
        );

        assert_eq!(
            try_map(&[face_id], vec![], vec![invalid]),
            Err(PresentedPointerMapError::PaintSpanOutOfRange)
        );
    }
}

#[test]
fn presented_pointer_map_rejects_empty_spans_and_unknown_faces() {
    let face_id = FaceId::new(7);
    let empty_span = PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Glyph,
            0,
            0,
            rect(0.0, 0.0, 10.0, 10.0),
        )],
        PointerDrawMode::Face(face_id),
        PointerDrawMode::Face(face_id),
    );
    assert_eq!(
        try_map(&[face_id], vec![], vec![empty_span]),
        Err(PresentedPointerMapError::EmptyPaintSpan)
    );

    assert_eq!(
        try_map(&[], vec![], vec![appearance(face_id)]),
        Err(PresentedPointerMapError::UnknownFace(face_id))
    );
}

#[test]
fn presented_pointer_map_rejects_regions_and_clips_outside_the_frame() {
    let face_id = FaceId::new(7);
    let outside_region = PresentedPointerRegion::new(
        rect(90.0, 0.0, 11.0, 10.0),
        Some(InteractionId::new(1)),
        Some(PointerAppearanceId::try_from(0usize).unwrap()),
    );
    assert_eq!(
        try_map(&[face_id], vec![outside_region], vec![appearance(face_id)]),
        Err(PresentedPointerMapError::RegionOutsideFrame)
    );

    let outside_clip = PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Image,
            8,
            1,
            rect(0.0, 45.0, 10.0, 6.0),
        )],
        image_relief_mode(false),
        image_relief_mode(true),
    );
    assert_eq!(
        try_map(&[], vec![], vec![outside_clip]),
        Err(PresentedPointerMapError::ClipOutsideFrame)
    );
}

#[test]
fn presented_pointer_protocol_values_round_trip_through_serde() {
    let face_id = FaceId::new(7);
    let value = appearance(face_id);
    let json = serde_json::to_string(&value).expect("serialize appearance");
    let decoded: PresentedPointerAppearance =
        serde_json::from_str(&json).expect("deserialize appearance");
    assert_eq!(decoded, value);

    let id = PointerAppearanceId::try_from(42usize).expect("representable id");
    let json = serde_json::to_string(&id).expect("serialize transparent id");
    assert_eq!(json, "42");
    assert_eq!(
        serde_json::from_str::<PointerAppearanceId>(&json).expect("deserialize id"),
        id
    );

    if usize::BITS > u32::BITS {
        assert!(PointerAppearanceId::try_from(usize::MAX).is_err());
    }

    let map = try_map(
        &[face_id],
        vec![PresentedPointerRegion::new(
            rect(0.0, 0.0, 10.0, 10.0),
            Some(InteractionId::new(1)),
            Some(PointerAppearanceId::try_from(0usize).unwrap()),
        )],
        vec![value],
    )
    .unwrap();
    let json = serde_json::to_string(&map).expect("serialize validated map");
    let decoded: PresentedPointerMap =
        serde_json::from_str(&json).expect("deserialize validated map");
    assert_eq!(decoded, map);
}

#[test]
fn presented_pointer_map_deserialization_rejects_intrinsically_invalid_data() {
    let missing_region_behavior = r#"{
        "regions":[{
            "bounds":{"x":0.0,"y":0.0,"width":10.0,"height":10.0},
            "interaction":null,
            "appearance":null
        }],
        "appearances":[]
    }"#;
    assert!(serde_json::from_str::<PresentedPointerMap>(missing_region_behavior).is_err());

    let unknown_appearance = r#"{
        "regions":[{
            "bounds":{"x":0.0,"y":0.0,"width":10.0,"height":10.0},
            "interaction":1,
            "appearance":4
        }],
        "appearances":[]
    }"#;
    assert!(serde_json::from_str::<PresentedPointerMap>(unknown_appearance).is_err());

    let empty_span = r#"{
        "regions":[],
        "appearances":[{
            "paint_spans":[{
                "kind":"Glyph",
                "first":0,
                "len":0,
                "clip":{"x":0.0,"y":0.0,"width":10.0,"height":10.0}
            }],
            "hover":{"Face":7},
            "pressed":{"Face":7}
        }]
    }"#;
    assert!(serde_json::from_str::<PresentedPointerMap>(empty_span).is_err());

    let non_finite_region = r#"{
        "regions":[{
            "bounds":{"x":1e400,"y":0.0,"width":10.0,"height":10.0},
            "interaction":1,
            "appearance":null
        }],
        "appearances":[]
    }"#;
    assert!(serde_json::from_str::<PresentedPointerMap>(non_finite_region).is_err());

    let empty_appearance = r#"{
        "regions":[],
        "appearances":[{
            "paint_spans":[],
            "hover":{"Face":7},
            "pressed":{"Face":7}
        }]
    }"#;
    assert!(serde_json::from_str::<PresentedPointerMap>(empty_appearance).is_err());

    let overflowing_span = r#"{
        "regions":[],
        "appearances":[{
            "paint_spans":[{
                "kind":"Glyph",
                "first":4294967295,
                "len":2,
                "clip":{"x":0.0,"y":0.0,"width":10.0,"height":10.0}
            }],
            "hover":{"Face":7},
            "pressed":{"Face":7}
        }]
    }"#;
    assert!(serde_json::from_str::<PresentedPointerMap>(overflowing_span).is_err());
}

#[test]
fn presented_pointer_hit_testing_is_half_open_and_stable_for_overlaps() {
    let face_id = FaceId::new(7);
    let appearance_id = PointerAppearanceId::try_from(0usize).unwrap();
    let first = PresentedPointerRegion::new(
        rect(10.0, 10.0, 20.0, 10.0),
        Some(InteractionId::new(1)),
        Some(appearance_id),
    );
    let overlapping = PresentedPointerRegion::new(
        rect(15.0, 10.0, 20.0, 10.0),
        Some(InteractionId::new(2)),
        Some(appearance_id),
    );
    let map = try_map(
        &[face_id],
        vec![first, overlapping],
        vec![appearance(face_id)],
    )
    .unwrap();

    assert_eq!(
        map.hit_test(15.0, 10.0)
            .map(PresentedPointerRegion::interaction),
        Some(Some(InteractionId::new(1)))
    );
    assert!(map.hit_test(9.99, 10.0).is_none());
    assert!(map.hit_test(10.0, 20.0).is_none());
    assert!(map.hit_test(35.0, 10.0).is_none());
}

#[test]
fn presented_pointer_hit_testing_examines_only_the_selected_y_band() {
    let mut regions = vec![PresentedPointerRegion::new(
        rect(50.0, 20.0, 20.0, 1.0),
        Some(InteractionId::new(1)),
        None,
    )];
    for row in 0..50 {
        if row != 20 {
            regions.push(PresentedPointerRegion::new(
                rect(0.0, row as f32, 10.0, 1.0),
                Some(InteractionId::new(100 + row)),
                None,
            ));
        }
    }
    regions.push(PresentedPointerRegion::new(
        rect(40.0, 20.0, 40.0, 1.0),
        Some(InteractionId::new(2)),
        None,
    ));
    let map = try_map(&[], regions, vec![]).unwrap();

    assert_eq!(map.hit_test_candidate_count(20.5), 2);
    assert_eq!(
        map.hit_test(55.0, 20.5)
            .map(PresentedPointerRegion::interaction),
        Some(Some(InteractionId::new(1)))
    );
}

#[test]
fn presented_pointer_hit_index_stores_each_staggered_region_once() {
    let mut regions = vec![PresentedPointerRegion::new(
        rect(50.0, 0.0, 30.0, 40.0),
        Some(InteractionId::new(1)),
        None,
    )];
    for index in 1..100 {
        regions.push(PresentedPointerRegion::new(
            rect(40.0, index as f32 * 0.4, 50.0, 10.0),
            Some(InteractionId::new(100 + index)),
            None,
        ));
    }
    let map = try_map(&[], regions, vec![]).unwrap();

    assert_eq!(map.hit_index_entry_count(), 100);
    assert_eq!(
        map.hit_test(55.0, 20.0)
            .map(PresentedPointerRegion::interaction),
        Some(Some(InteractionId::new(1)))
    );

    let wire = serde_json::to_string(&map).unwrap();
    let decoded: PresentedPointerMap = serde_json::from_str(&wire).unwrap();
    assert_eq!(decoded.hit_index_entry_count(), 100);
    assert_eq!(
        decoded
            .hit_test(55.0, 20.0)
            .map(PresentedPointerRegion::interaction),
        Some(Some(InteractionId::new(1)))
    );
}

#[test]
fn frame_glyph_buffers_start_with_an_empty_presented_pointer_map() {
    let default_buffer = crate::FrameGlyphBuffer::default();
    let constructed_buffer = crate::FrameGlyphBuffer::new();
    let sized_buffer = crate::FrameGlyphBuffer::with_size(100.0, 50.0);

    assert!(default_buffer.presented_pointer().is_empty());
    assert!(constructed_buffer.presented_pointer().is_empty());
    assert!(sized_buffer.presented_pointer().is_empty());
}

#[test]
fn frame_glyph_buffer_installs_pointer_parts_against_its_actual_snapshot() {
    let face_id = FaceId::new(7);
    let mut buffer = crate::FrameGlyphBuffer::with_size(100.0, 50.0);
    buffer.set_face(
        face_id,
        crate::Color::WHITE,
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
    buffer.add_char('a', 0.0, 0.0, 10.0, 10.0, 8.0, false);
    buffer.add_image(crate::ImageId::new(1), 10.0, 0.0, 10.0, 10.0);

    let out_of_range = PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Glyph,
            2,
            1,
            rect(0.0, 0.0, 10.0, 10.0),
        )],
        image_relief_mode(false),
        image_relief_mode(true),
    );
    assert_eq!(
        buffer.install_presented_pointer(vec![], vec![out_of_range]),
        Err(PresentedPointerMapError::PaintSpanOutOfRange)
    );

    let unknown_face = PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Glyph,
            0,
            1,
            rect(0.0, 0.0, 10.0, 10.0),
        )],
        PointerDrawMode::Face(FaceId::new(99)),
        PointerDrawMode::Face(FaceId::new(99)),
    );
    assert_eq!(
        buffer.install_presented_pointer(vec![], vec![unknown_face]),
        Err(PresentedPointerMapError::UnknownFace(FaceId::new(99)))
    );

    let wrong_kind = PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Image,
            0,
            1,
            rect(0.0, 0.0, 10.0, 10.0),
        )],
        image_relief_mode(false),
        image_relief_mode(true),
    );
    assert_eq!(
        buffer.install_presented_pointer(vec![], vec![wrong_kind]),
        Err(PresentedPointerMapError::PrimitiveKindMismatch)
    );

    let exact_boundary = PresentedPointerAppearance::new(
        vec![
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                0,
                1,
                rect(0.0, 0.0, 10.0, 10.0),
            ),
            PresentedPaintSpan::new(
                PresentedPrimitiveKind::Image,
                1,
                1,
                rect(10.0, 0.0, 10.0, 10.0),
            ),
        ],
        image_relief_mode(false),
        image_relief_mode(true),
    );
    buffer
        .install_presented_pointer(vec![], vec![exact_boundary])
        .expect("span ending at the actual glyph boundary is valid");
    assert_eq!(buffer.presented_pointer().appearances().len(), 1);

    buffer.clear_all();
    assert!(buffer.presented_pointer().is_empty());
}

#[test]
fn frame_glyph_buffer_contextually_validates_deserialized_pointer_maps_before_installing() {
    let face_id = FaceId::new(7);
    let appearance_id = PointerAppearanceId::try_from(0usize).unwrap();
    let transported = try_map(
        &[face_id],
        vec![PresentedPointerRegion::new(
            rect(0.0, 0.0, 10.0, 10.0),
            Some(InteractionId::new(77)),
            Some(appearance_id),
        )],
        vec![PresentedPointerAppearance::new(
            vec![PresentedPaintSpan::new(
                PresentedPrimitiveKind::Glyph,
                0,
                1,
                rect(0.0, 0.0, 10.0, 10.0),
            )],
            PointerDrawMode::Face(face_id),
            PointerDrawMode::Face(face_id),
        )],
    )
    .unwrap();
    let wire = serde_json::to_string(&transported).unwrap();

    let mut mismatched = crate::FrameGlyphBuffer::with_size(100.0, 50.0);
    mismatched
        .install_presented_pointer(
            vec![PresentedPointerRegion::new(
                rect(20.0, 0.0, 10.0, 10.0),
                Some(InteractionId::new(55)),
                None,
            )],
            vec![],
        )
        .unwrap();
    let invalid_for_snapshot: PresentedPointerMap = serde_json::from_str(&wire).unwrap();
    assert!(
        mismatched
            .install_presented_pointer_map(invalid_for_snapshot)
            .is_err()
    );
    assert_eq!(
        mismatched
            .presented_pointer()
            .hit_test(25.0, 5.0)
            .map(PresentedPointerRegion::interaction),
        Some(Some(InteractionId::new(55)))
    );

    let mut matching = crate::FrameGlyphBuffer::with_size(100.0, 50.0);
    matching.faces.insert(face_id, crate::Face::new(face_id));
    matching.add_char('a', 0.0, 0.0, 10.0, 10.0, 8.0, false);
    let valid_for_snapshot: PresentedPointerMap = serde_json::from_str(&wire).unwrap();
    matching
        .install_presented_pointer_map(valid_for_snapshot)
        .expect("matching transported map installs");
    assert_eq!(
        matching
            .presented_pointer()
            .hit_test(5.0, 5.0)
            .map(PresentedPointerRegion::interaction),
        Some(Some(InteractionId::new(77)))
    );
}

#[test]
fn presented_pointer_glyph_spans_reject_non_text_frame_primitives() {
    let mut buffer = crate::FrameGlyphBuffer::with_size(100.0, 50.0);
    buffer.add_background(0.0, 0.0, 10.0, 10.0, crate::Color::BLACK);
    let appearance = PresentedPointerAppearance::new(
        vec![PresentedPaintSpan::new(
            PresentedPrimitiveKind::Glyph,
            0,
            1,
            rect(0.0, 0.0, 10.0, 10.0),
        )],
        image_relief_mode(false),
        image_relief_mode(true),
    );

    assert_eq!(
        buffer.install_presented_pointer(vec![], vec![appearance]),
        Err(PresentedPointerMapError::PrimitiveKindMismatch)
    );
}
