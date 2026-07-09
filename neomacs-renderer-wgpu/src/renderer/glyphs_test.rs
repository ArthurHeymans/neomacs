use neomacs_display_protocol::types::FaceId;
use super::{
    RenderedCharBounds, char_overlap, cursor_glyph_slot_rect, frame_default_glyph_metrics,
};
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, FrameGlyph, FrameGlyphBuffer, GlyphRowRole, WindowCursor,
};
use neomacs_display_protocol::types::{Color, DisplayWindowId};

fn make_cursor(
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    style: CursorStyle,
) -> WindowCursor {
    WindowCursor {
        window_id: slot_id.window_id,
        slot_id,
        x,
        y,
        width,
        height: 16.0,
        style,
        color: Color::WHITE,
        cursor_fg: Color::BLACK,
        ascent: 12.0,
        active: true,
    }
}

#[test]
fn rtl_bar_cursor_uses_right_edge_of_char_slot() {
    let mut frame = FrameGlyphBuffer::new();
    frame.set_draw_context(DisplayWindowId::new(1), GlyphRowRole::Text, None);
    frame.add_char('א', 10.0, 20.0, 12.0, 16.0, 12.0, false);
    let slot_id = frame.glyphs[0].slot_id().expect("slot id");
    if let FrameGlyph::Char { bidi_level, .. } = &mut frame.glyphs[0] {
        *bidi_level = 1;
    }

    let cursor = make_cursor(slot_id, 10.0, 20.0, 2.0, CursorStyle::Bar(2.0));
    assert_eq!(
        cursor_glyph_slot_rect(&frame, &cursor),
        (20.0, 20.0, 2.0, 16.0)
    );
}

#[test]
fn rtl_hbar_cursor_uses_right_edge_of_stretch_slot() {
    let mut frame = FrameGlyphBuffer::new();
    frame.set_draw_context(DisplayWindowId::new(2), GlyphRowRole::Text, None);
    frame.add_stretch(30.0, 40.0, 24.0, 16.0, Color::BLACK, FaceId::new(0), false);
    let slot_id = frame.glyphs[0].slot_id().expect("slot id");
    if let FrameGlyph::Stretch { bidi_level, .. } = &mut frame.glyphs[0] {
        *bidi_level = 1;
    }

    let cursor = make_cursor(slot_id, 30.0, 40.0, 8.0, CursorStyle::Hbar(2.0));
    assert_eq!(
        cursor_glyph_slot_rect(&frame, &cursor),
        (46.0, 40.0, 8.0, 16.0)
    );
}

#[test]
fn filled_box_cursor_keeps_slot_origin_in_rtl_runs() {
    let mut frame = FrameGlyphBuffer::new();
    frame.set_draw_context(DisplayWindowId::new(3), GlyphRowRole::Text, None);
    frame.add_char('א', 50.0, 60.0, 12.0, 16.0, 12.0, false);
    let slot_id = frame.glyphs[0].slot_id().expect("slot id");
    if let FrameGlyph::Char { bidi_level, .. } = &mut frame.glyphs[0] {
        *bidi_level = 1;
    }

    let cursor = make_cursor(slot_id, 50.0, 60.0, 8.0, CursorStyle::FilledBox);
    assert_eq!(
        cursor_glyph_slot_rect(&frame, &cursor),
        (50.0, 60.0, 12.0, 16.0)
    );
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
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
            row: 0,
            col: 0,
        },
        label: label.to_string(),
        face_id: FaceId::new(0),
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
fn char_overlap_classifies_subpixel_boundary_overhang_separately() {
    let mut slash = char_bounds("/", 775.8, 698.0, 14.0, 31.0);
    slash.glyph_x = 776.0;
    slash.glyph_y = 701.7;
    slash.glyph_w = 14.3;
    slash.glyph_h = 22.3;
    slash.right_overhang = (slash.glyph_x + slash.glyph_w - (slash.cell_x + slash.cell_w)).max(0.0);

    let mut m = char_bounds("m", 789.8, 698.0, 14.0, 31.0);
    m.glyph_x = 789.7;
    m.glyph_y = 708.0;
    m.glyph_w = 13.7;
    m.glyph_h = 13.7;
    m.left_overhang = (m.cell_x - m.glyph_x).max(0.0);

    let overlap = char_overlap(&slash, &m).expect("subpixel boundary overhang");
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
