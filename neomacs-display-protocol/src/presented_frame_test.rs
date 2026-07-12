use super::*;

fn rect(x: f32, y: f32, width: f32, height: f32) -> FrameRect {
    FrameRect::new(x, y, width, height).unwrap()
}

fn placement(
    id: u64,
    presentation: u64,
    parent: Option<u64>,
    rect: FrameRect,
    z: i32,
) -> PresentedFramePlacement {
    PresentedFramePlacement::new(
        DisplayFrameId::new(id),
        PresentationId::new(presentation),
        parent.map(DisplayFrameId::new),
        rect,
        z,
    )
}

#[test]
fn place_child_preserves_immediate_parent_coordinates_and_composes_nested_ancestry_once() {
    let scene = PresentedFrameScene::from_placements([
        placement(1, 10, None, rect(0.0, 0.0, 800.0, 600.0), 0),
        placement(2, 11, Some(1), rect(120.0, 48.0, 300.0, 200.0), 2),
        placement(3, 12, Some(2), rect(7.0, 9.0, 100.0, 80.0), 4),
    ])
    .unwrap();

    let placed = scene
        .place(PlaceChildQuery::new(
            DisplayFrameId::new(3),
            PresentationId::new(12),
        ))
        .unwrap();
    assert_eq!(placed.parent_relative(), rect(7.0, 9.0, 100.0, 80.0));
    assert_eq!(placed.root(), DisplayFrameId::new(1));
    assert_eq!(placed.root_relative(), rect(127.0, 57.0, 100.0, 80.0));
    assert_eq!(
        placed.clip_in_root(),
        PresentedClip::Rect(rect(127.0, 57.0, 100.0, 80.0))
    );
    assert_eq!(placed.z_path(), &[0, 2, 4]);
}

#[test]
fn place_child_clips_to_each_ancestor_and_rejects_stale_missing_and_cycles() {
    let scene = PresentedFrameScene::from_placements([
        placement(1, 10, None, rect(0.0, 0.0, 100.0, 100.0), 0),
        placement(2, 11, Some(1), rect(80.0, 80.0, 50.0, 50.0), 1),
    ])
    .unwrap();
    let placed = scene
        .place(PlaceChildQuery::new(
            DisplayFrameId::new(2),
            PresentationId::new(11),
        ))
        .unwrap();
    assert_eq!(placed.root_relative(), rect(80.0, 80.0, 50.0, 50.0));
    assert_eq!(
        placed.clip_in_root(),
        PresentedClip::Rect(rect(80.0, 80.0, 20.0, 20.0))
    );
    assert!(matches!(
        scene.place(PlaceChildQuery::new(
            DisplayFrameId::new(2),
            PresentationId::new(9)
        )),
        Err(PlaceChildError::StalePresentation { .. })
    ));
    assert_eq!(
        scene.place(PlaceChildQuery::new(
            DisplayFrameId::new(9),
            PresentationId::new(9)
        )),
        Err(PlaceChildError::MissingFrame(DisplayFrameId::new(9)))
    );
    assert!(matches!(
        PresentedFrameScene::from_placements([
            placement(1, 1, Some(2), rect(0.0, 0.0, 10.0, 10.0), 0),
            placement(2, 2, Some(1), rect(0.0, 0.0, 10.0, 10.0), 0),
        ]),
        Err(PlaceChildError::AncestryCycle(_))
    ));
}

#[test]
fn fully_clipped_descendant_stays_empty_through_remaining_ancestry() {
    let scene = PresentedFrameScene::from_placements([
        placement(1, 10, None, rect(0.0, 0.0, 100.0, 100.0), 0),
        placement(2, 11, Some(1), rect(150.0, 0.0, 40.0, 40.0), 1),
        placement(3, 12, Some(2), rect(0.0, 0.0, 20.0, 20.0), 2),
    ])
    .unwrap();

    let placed = scene
        .place(PlaceChildQuery::new(
            DisplayFrameId::new(3),
            PresentationId::new(12),
        ))
        .unwrap();
    assert_eq!(placed.clip_in_root(), PresentedClip::Empty);
}
