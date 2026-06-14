use super::*;

fn setup_request() -> BufferTextWindowWalkSetupRequest<'static> {
    BufferTextWindowWalkSetupRequest::new(
        17,
        24.0,
        16.0,
        160.0,
        32.0,
        8.0,
        12.0,
        5,
        8.0,
        16.0,
        11.0,
        true,
        3,
        true,
        true,
        true,
        true,
        false,
        4,
        &[4, 12],
        true,
        0x00ff00,
    )
}

#[test]
fn walk_setup_initializes_source_position_and_geometry_state() {
    let setup = setup_request().into_setup();

    assert_eq!(setup.byte_idx, 0);
    assert_eq!(setup.charpos, 17);
    assert_eq!(setup.x, 24.0);
    assert_eq!(setup.col, 0);
    assert_eq!(setup.text_area_left, 16.0);
    assert_eq!(setup.window_top, 8.0);
    assert_eq!(setup.row_flags.len(), 5);
    assert_eq!(setup.row_geometry.row(), 0);
    assert_eq!(setup.row_geometry.y(), 32.0);
    assert_eq!(setup.row_geometry.height(), 16.0);
    assert_eq!(setup.row_geometry.ascent(), 11.0);
    assert_eq!(setup.hit_row_range.start(), 17);
}

#[test]
fn walk_setup_applies_hscroll_prefix_and_reserved_surface_policy() {
    let setup = setup_request().into_setup();

    assert!(setup.hscroll_skip.should_skip());
    assert_eq!(setup.hscroll_skip.consumed_columns(), 0);
    assert!(setup.prefix_request.is_requested());
    assert_eq!(setup.text_append_surface.content_x(), 24.0);
    assert_eq!(setup.text_append_surface.right_edge(), 164.0);
    assert!(setup.trailing_whitespace.background().is_some());
}
