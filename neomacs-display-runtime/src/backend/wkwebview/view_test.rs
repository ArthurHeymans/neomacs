//! Geometry tests for the inline-web-view placement.
//!
//! These cover `Placement` only — it is the whole of the ported Emacs
//! arithmetic and the only part that can be exercised without AppKit and a
//! main thread. Cases are written against `x_draw_xwidget_glyph_string`
//! (`xwidget.c:2841-2849`, `:2856`, `:2961`).

use super::Placement;

/// A 200x100 widget at (50, 40) inside a text area covering (0, 0, 400, 300).
fn fully_visible() -> Placement {
    Placement::new(50.0, 40.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)))
}

#[test]
fn unclipped_when_the_frame_glyph_carries_no_clip_rect() {
    let p = Placement::new(10.0, 20.0, 300.0, 150.0, None);
    assert_eq!(p.clip_left, 0.0);
    assert_eq!(p.clip_top, 0.0);
    assert_eq!(p.visible_width(), 300.0);
    assert_eq!(p.visible_height(), 150.0);
    assert!(!p.is_empty());
}

#[test]
fn a_widget_inside_the_text_area_is_not_clipped() {
    let p = fully_visible();
    assert_eq!(p.clip_left, 0.0);
    assert_eq!(p.clip_top, 0.0);
    assert_eq!(p.visible_width(), 200.0);
    assert_eq!(p.visible_height(), 100.0);
}

#[test]
fn scrolling_past_the_top_edge_insets_clip_top() {
    // Widget top is 30pt above the text area's top edge.
    let p = Placement::new(50.0, -30.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert_eq!(p.clip_top, 30.0);
    assert_eq!(p.clip_bottom, 100.0);
    assert_eq!(p.visible_height(), 70.0);
    // The on-screen origin sits at the text area's top edge, not the widget's.
    assert_eq!(p.y + p.clip_top, 0.0);
}

#[test]
fn overflowing_the_bottom_edge_crops_without_moving_the_origin() {
    // 100pt tall widget starting 40pt above the area's bottom edge.
    let p = Placement::new(50.0, 260.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert_eq!(p.clip_top, 0.0);
    assert_eq!(p.clip_bottom, 40.0);
    assert_eq!(p.visible_height(), 40.0);
    assert_eq!(p.y + p.clip_top, 260.0);
}

#[test]
fn horizontal_clipping_matches_the_vertical_case() {
    let p = Placement::new(-25.0, 40.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert_eq!(p.clip_left, 25.0);
    assert_eq!(p.visible_width(), 175.0);
    assert_eq!(p.x + p.clip_left, 0.0);
}

#[test]
fn a_widget_scrolled_entirely_out_is_empty_not_inverted() {
    // Emacs' max() guards keep the rect from inverting here; an inverted rect
    // would otherwise become a negative NSSize.
    let above = Placement::new(50.0, -500.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert!(above.is_empty());
    assert!(above.visible_height() >= 0.0);

    let below = Placement::new(50.0, 900.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert!(below.is_empty());
    assert!(below.visible_height() >= 0.0);

    let left = Placement::new(-900.0, 40.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert!(left.is_empty());
    assert!(left.visible_width() >= 0.0);
}

#[test]
fn movement_is_tracked_on_the_clipped_origin() {
    // Emacs xwidget.c:2856. The widget moves up by 10 while its visible top
    // stays pinned to the text area's edge, so the on-screen area has NOT
    // moved and no reposition should be issued.
    let before = Placement::new(50.0, -30.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    let after = Placement::new(50.0, -40.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert_eq!(before.y + before.clip_top, after.y + after.clip_top);
    assert!(!after.moved_from(&before));
    // ... but the clip did change, so the view still needs reclipping.
    assert!(after.reclipped_from(&before));
}

#[test]
fn a_plain_translation_counts_as_movement() {
    let before = fully_visible();
    let after = Placement::new(90.0, 40.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert!(after.moved_from(&before));
    assert!(!after.reclipped_from(&before));
}

#[test]
fn an_unchanged_placement_is_neither_moved_nor_reclipped() {
    let before = fully_visible();
    let after = fully_visible();
    assert!(!after.moved_from(&before));
    assert!(!after.reclipped_from(&before));
}

/// The inner offset that makes the clip view show the right region.
///
/// Emacs sets the inner origin to `(-clip_left, -clip_top)` in a flipped view
/// (`xwidget.c:2996`). The bottom-up form must place the web view's top edge
/// in exactly the same spot.
#[test]
fn bottom_up_inner_offset_agrees_with_the_flipped_form() {
    let model_height = 100.0_f64;
    for y in [-30.0, 0.0, 260.0] {
        let p = Placement::new(50.0, y, 200.0, model_height, Some((0.0, 0.0, 400.0, 300.0)));
        let (flip_x, flip_y) = p.inner_origin(true);
        let (up_x, up_y) = p.inner_origin(false);
        assert_eq!(flip_x, up_x, "x is unaffected by the flip");

        // Flipped: the web view's top sits `clip_top` above the clip's top,
        // measured downward from the clip origin.
        assert_eq!(flip_y, -p.clip_top);
        // Bottom-up: the same edge, measured from the clip view's bottom.
        let web_top_from_clip_bottom = up_y + model_height;
        assert_eq!(web_top_from_clip_bottom, p.visible_height() + p.clip_top);
    }
}

/// Regression: on a bottom-left host the clip origin depends on the visible
/// *height*, so a pure reclip has to rewrite it. Emacs can skip this because
/// its views are flipped (`nsxwidget.m:554`); we cannot.
///
/// The reachable case is a window resized shorter under a widget whose top
/// has not moved: `moved_from` is false, but the origin still changes.
#[test]
fn a_pure_reclip_moves_the_bottom_up_origin() {
    let host_height = 800.0;
    // Widget top pinned at y=40; the text area shrinks from 300 to 200 tall.
    let tall = Placement::new(50.0, 40.0, 200.0, 300.0, Some((0.0, 0.0, 400.0, 300.0)));
    let short = Placement::new(50.0, 40.0, 200.0, 300.0, Some((0.0, 0.0, 400.0, 200.0)));

    assert!(
        !short.moved_from(&tall),
        "the top-left corner has not moved"
    );
    assert!(short.reclipped_from(&tall), "but the clip has changed");

    // Flipped hosts are unaffected — this is exactly why Emacs gets away with
    // gating the reposition on `moved` alone.
    assert_eq!(
        tall.ns_origin(true, host_height),
        short.ns_origin(true, host_height)
    );

    // Bottom-up hosts are affected, and the delta is the height change.
    let (_, tall_y) = tall.ns_origin(false, host_height);
    let (_, short_y) = short.ns_origin(false, host_height);
    assert_ne!(
        tall_y, short_y,
        "a bottom-up origin must follow the visible height"
    );
    assert_eq!(
        short_y - tall_y,
        tall.visible_height() - short.visible_height()
    );
}

#[test]
fn ns_origin_matches_top_left_on_a_flipped_host() {
    let p = fully_visible();
    assert_eq!(p.ns_origin(true, 800.0), p.top_left());
}
