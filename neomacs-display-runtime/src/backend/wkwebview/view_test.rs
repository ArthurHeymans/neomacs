//! Geometry tests for the inline-web-view placement.
//!
//! These cover `Placement` only — it is the whole of the ported Emacs
//! arithmetic and the only part that can be exercised without AppKit and a
//! main thread. Cases are written against `x_draw_xwidget_glyph_string`
//! (`xwidget.c:2841-2849`, `:2856`, `:2961`).

use super::{Placement, needs_reposition};

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
/// It is Emacs' `(-clip_left, -clip_top)` (`xwidget.c:2996`) with nothing
/// added, because the frame it positions lives in `XwidgetClipView`, which is
/// flipped for precisely this reason.
#[test]
fn the_inner_offset_is_the_negated_clip_inset() {
    for y in [-30.0, 0.0, 260.0] {
        let p = Placement::new(50.0, y, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
        assert_eq!(p.inner_origin(), (-p.clip_left, -p.clip_top));
    }
}

/// Review catch (PR #297): the inner origin was computed from the *host's*
/// orientation, but the frame it sets is in the clip view's coordinate system.
///
/// The reviewer's own numbers: a 100-tall widget scrolled 20 points under the
/// top of the text area shows its bottom 80. In a flipped clip the web view's
/// origin is -20; the old bottom-up branch answered
/// `visible_height + clip_top - height` = `80 + 20 - 100` = 0, which pins the
/// wrong edge and shows the widget's *top* 80 points instead.
#[test]
fn the_inner_offset_pins_the_top_edge_not_the_bottom() {
    // Widget 100 tall at y = -20, so clip_top = 20 and visible_height = 80.
    let p = Placement::new(0.0, -20.0, 200.0, 100.0, Some((0.0, 0.0, 400.0, 300.0)));
    assert_eq!(p.clip_top, 20.0);
    assert_eq!(p.visible_height(), 80.0);

    let (_, inner_y) = p.inner_origin();
    assert_eq!(inner_y, -20.0);
    assert_ne!(
        inner_y,
        p.visible_height() + p.clip_top - p.height,
        "the bottom-up formula answers 0 here and clips the opposite edge"
    );
}

/// Regression: on a bottom-left host the clip origin depends on the visible
/// *height*, so a pure reclip has to rewrite it. Emacs can skip this because
/// its host view is flipped (`nsterm.m:8540`); we may not be able to.
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

/// The bottom-up clip origin also depends on the *host's* height, which no
/// placement diff can see. A window resized under a widget that did not itself
/// move changes the origin while `moved_from` and `reclipped_from` both answer
/// false, so `apply` has to watch the host separately.
#[test]
fn the_bottom_up_origin_follows_the_host_height_alone() {
    let p = fully_visible();
    let (_, tall) = p.ns_origin(false, 800.0);
    let (_, short) = p.ns_origin(false, 600.0);
    assert_eq!(tall - short, 200.0);

    // A flipped host is immune, which is why the gate in `apply` is
    // `!host_flipped && ...` rather than unconditional.
    assert_eq!(p.ns_origin(true, 800.0), p.ns_origin(true, 600.0));
}

/// Emacs' own rule, and the only one a flipped host needs: nothing but a move
/// rewrites the origin (`xwidget.c:2951`).
#[test]
fn a_flipped_host_repositions_only_on_a_move() {
    assert!(needs_reposition(true, false, false, true));
    assert!(!needs_reposition(false, true, false, true));
    assert!(!needs_reposition(false, false, true, true));
    assert!(!needs_reposition(false, false, false, true));
}

/// A bottom-left host folds the visible height into the origin, so a pure
/// reclip has to rewrite it even though the top-left corner has not moved.
#[test]
fn a_bottom_up_host_repositions_on_a_pure_reclip() {
    assert!(needs_reposition(false, true, false, false));
}

/// Review-adjacent catch (PR #297): the origin also folds in the *host's*
/// height, which no placement diff can see. A window resized under a widget
/// that neither moved nor reclipped left the view at a stale origin, because
/// nothing marked it dirty.
#[test]
fn a_bottom_up_host_repositions_when_only_the_host_changed() {
    assert!(needs_reposition(false, false, true, false));
}

/// ...and an idle frame still writes nothing, which is the point of having a
/// gate at all.
#[test]
fn an_idle_frame_repositions_nothing() {
    assert!(!needs_reposition(false, false, false, false));
}
