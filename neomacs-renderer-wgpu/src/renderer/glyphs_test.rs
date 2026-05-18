use super::{
    RenderedCharBounds, char_overlap, cursor_render_rect, frame_default_glyph_metrics,
    window_cursor_visual_matches_phys,
};
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, FrameGlyph, FrameGlyphBuffer, GlyphRowRole, PhysCursor,
    WindowCursorVisual,
};
use neomacs_display_protocol::types::Color;

fn make_cursor(
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    style: CursorStyle,
) -> PhysCursor {
    PhysCursor {
        window_id: slot_id.window_id,
        charpos: 0,
        row: slot_id.row as usize,
        col: slot_id.col,
        slot_id,
        x,
        y,
        width,
        height: 16.0,
        ascent: 12.0,
        style,
        color: Color::WHITE,
        cursor_fg: Color::BLACK,
    }
}

#[test]
fn rtl_bar_cursor_uses_right_edge_of_char_slot() {
    let mut frame = FrameGlyphBuffer::new();
    frame.set_draw_context(1, GlyphRowRole::Text, None);
    frame.add_char('א', 10.0, 20.0, 12.0, 16.0, 12.0, false);
    let slot_id = frame.glyphs[0].slot_id().expect("slot id");
    if let FrameGlyph::Char { bidi_level, .. } = &mut frame.glyphs[0] {
        *bidi_level = 1;
    }

    let cursor = make_cursor(slot_id, 10.0, 20.0, 2.0, CursorStyle::Bar(2.0));
    assert_eq!(cursor_render_rect(&frame, &cursor), (20.0, 20.0, 2.0, 16.0));
}

#[test]
fn rtl_hbar_cursor_uses_right_edge_of_stretch_slot() {
    let mut frame = FrameGlyphBuffer::new();
    frame.set_draw_context(2, GlyphRowRole::Text, None);
    frame.add_stretch(30.0, 40.0, 24.0, 16.0, Color::BLACK, 0, false);
    let slot_id = frame.glyphs[0].slot_id().expect("slot id");
    if let FrameGlyph::Stretch { bidi_level, .. } = &mut frame.glyphs[0] {
        *bidi_level = 1;
    }

    let cursor = make_cursor(slot_id, 30.0, 40.0, 8.0, CursorStyle::Hbar(2.0));
    assert_eq!(cursor_render_rect(&frame, &cursor), (46.0, 40.0, 8.0, 16.0));
}

#[test]
fn filled_box_cursor_keeps_slot_origin_in_rtl_runs() {
    let mut frame = FrameGlyphBuffer::new();
    frame.set_draw_context(3, GlyphRowRole::Text, None);
    frame.add_char('א', 50.0, 60.0, 12.0, 16.0, 12.0, false);
    let slot_id = frame.glyphs[0].slot_id().expect("slot id");
    if let FrameGlyph::Char { bidi_level, .. } = &mut frame.glyphs[0] {
        *bidi_level = 1;
    }

    let cursor = make_cursor(slot_id, 50.0, 60.0, 8.0, CursorStyle::FilledBox);
    assert_eq!(
        cursor_render_rect(&frame, &cursor),
        (50.0, 60.0, 12.0, 16.0)
    );
}

#[test]
fn window_cursor_visual_match_uses_slot_identity() {
    let slot_id = DisplaySlotId::from_pixels(7, 32.0, 16.0, 8.0, 16.0);
    let phys = make_cursor(slot_id, 32.0, 16.0, 8.0, CursorStyle::FilledBox);
    let matching = WindowCursorVisual {
        window_id: 7,
        slot_id,
        x: 4.0,
        y: 0.0,
        width: 20.0,
        height: 30.0,
        style: CursorStyle::Hollow,
        color: Color::WHITE,
    };
    let mismatched = WindowCursorVisual {
        window_id: 7,
        slot_id: DisplaySlotId::from_pixels(7, 40.0, 16.0, 8.0, 16.0),
        x: 32.0,
        y: 16.0,
        width: 8.0,
        height: 16.0,
        style: CursorStyle::Hollow,
        color: Color::WHITE,
    };

    assert!(window_cursor_visual_matches_phys(&matching, &phys));
    assert!(!window_cursor_visual_matches_phys(&mismatched, &phys));
}

#[test]
fn frame_default_glyph_metrics_use_frame_font_and_line_height() {
    let mut frame = FrameGlyphBuffer::new();
    frame.font_pixel_size = 27.0;
    frame.char_height = 33.0;

    assert_eq!(frame_default_glyph_metrics(&frame), (27.0, 33.0));
}

#[test]
fn frame_default_glyph_metrics_fall_back_to_sane_values() {
    let mut frame = FrameGlyphBuffer::new();
    frame.font_pixel_size = f32::NAN;
    frame.char_height = 0.0;

    let (font_size, line_height) = frame_default_glyph_metrics(&frame);
    assert_eq!(font_size, 14.0);
    assert!((line_height - 16.8).abs() < 0.001);
}

fn char_bounds(label: &str, x: f32, y: f32, width: f32, height: f32) -> RenderedCharBounds {
    RenderedCharBounds {
        glyph_index: 0,
        window_id: 1,
        row_role: GlyphRowRole::Text,
        slot_id: DisplaySlotId {
            window_id: 1,
            row: 0,
            col: 0,
        },
        label: label.to_string(),
        face_id: 0,
        font_size: 14.0,
        cell_x: x,
        cell_y: y,
        cell_w: width,
        cell_h: height,
        glyph_x: x,
        glyph_y: y,
        glyph_w: width,
        glyph_h: height,
        left_overhang: 0.0,
        right_overhang: 0.0,
    }
}

#[test]
fn char_overlap_detects_intersecting_rendered_bitmaps() {
    let a = char_bounds("A", 0.0, 0.0, 10.0, 12.0);
    let b = char_bounds("B", 9.0, 4.0, 10.0, 12.0);

    let overlap = char_overlap(&a, &b).expect("overlap");
    assert_eq!(overlap.x, 9.0);
    assert_eq!(overlap.y, 4.0);
    assert_eq!(overlap.width, 1.0);
    assert_eq!(overlap.height, 8.0);
    assert!(!overlap.expected_by_overhang);
}

#[test]
fn char_overlap_ignores_touching_edges_and_subpixel_noise() {
    let a = char_bounds("A", 0.0, 0.0, 10.0, 12.0);
    let touching = char_bounds("B", 10.0, 0.0, 10.0, 12.0);
    let tiny = char_bounds("C", 9.75, 0.0, 10.0, 12.0);

    assert!(char_overlap(&a, &touching).is_none());
    assert!(char_overlap(&a, &tiny).is_none());
}

#[test]
fn char_overlap_classifies_font_overhang_separately() {
    let mut f = char_bounds("f", 0.0, 0.0, 9.0, 12.0);
    f.glyph_w = 11.0;
    f.right_overhang = 2.0;
    let next = char_bounds("a", 9.0, 0.0, 12.0, 12.0);

    let overlap = char_overlap(&f, &next).expect("overhang overlap");
    assert_eq!(overlap.x, 9.0);
    assert_eq!(overlap.width, 2.0);
    assert!(overlap.expected_by_overhang);
}

#[test]
fn char_overlap_classifies_adjacent_dual_bearing_overhang_separately() {
    let mut w = char_bounds("w", 888.0, 384.0, 16.0, 33.0);
    w.glyph_x = 889.0;
    w.glyph_y = 395.0;
    w.glyph_w = 17.0;
    w.glyph_h = 15.0;
    w.right_overhang = 2.0;

    let mut x = char_bounds("x", 904.0, 384.0, 16.0, 33.0);
    x.glyph_x = 903.0;
    x.glyph_y = 395.0;
    x.glyph_w = 18.0;
    x.glyph_h = 15.0;
    x.left_overhang = 1.0;
    x.right_overhang = 1.0;

    let overlap = char_overlap(&w, &x).expect("dual-bearing overhang overlap");
    assert_eq!(overlap.x, 903.0);
    assert_eq!(overlap.width, 3.0);
    assert!(overlap.expected_by_overhang);
}
