use super::*;
use crate::face::{Face, FaceAttributes, UnderlineStyle};
use crate::frame_glyphs::{CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor};
use crate::glyph_matrix::{
    FaceFillItem, FrameDisplayState, Glyph, GlyphArea, GlyphMatrix, GlyphRow, RowDamage,
    WindowMatrixEntry,
};
use crate::tty_capabilities::TtyNoColorVideo;
use crate::types::Px;
use crate::types::{Color, DisplayFrameId, DisplayWindowId, Rect};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// TtyRif::new
// ---------------------------------------------------------------------------

#[test]
fn new_creates_correct_grid_dimensions() {
    let rif = TtyRif::new(80, 24);
    assert_eq!(rif.width(), 80);
    assert_eq!(rif.height(), 24);
    assert_eq!(rif.current.cells.len(), 80 * 24);
    assert_eq!(rif.desired.cells.len(), 80 * 24);
}

#[test]
fn new_grids_are_blank_spaces() {
    let rif = TtyRif::new(10, 5);
    for cell in &rif.current.cells {
        assert_eq!(cell.ch, ' ');
        assert!(!cell.padding);
    }
}

// ---------------------------------------------------------------------------
// resize
// ---------------------------------------------------------------------------

#[test]
fn resize_updates_dimensions() {
    let mut rif = TtyRif::new(80, 24);
    rif.resize(120, 40);
    assert_eq!(rif.width(), 120);
    assert_eq!(rif.height(), 40);
    assert_eq!(rif.current.cells.len(), 120 * 40);
    assert_eq!(rif.desired.cells.len(), 120 * 40);
}

#[test]
fn resize_clears_grids() {
    let mut rif = TtyRif::new(10, 5);
    // Dirty a cell in current.
    rif.current.set(0, 0, 'X', CellAttrs::default(), false);
    rif.resize(10, 5);
    // After resize, the cell should be blank again.
    assert_eq!(rif.current.cells[0].ch, ' ');
}

// ---------------------------------------------------------------------------
// Face resolution
// ---------------------------------------------------------------------------

#[test]
fn resolve_attrs_uses_face_table() {
    let mut rif = TtyRif::new(80, 24);
    let mut face = Face::new(FaceId::new(1));
    face.foreground = Color::rgb(1.0, 0.0, 0.0);
    face.background = Color::rgb(0.0, 1.0, 0.0);
    face.font_weight = 700;
    face.attributes |= FaceAttributes::ITALIC;
    face.underline_style = UnderlineStyle::Wave;
    face.attributes |= FaceAttributes::STRIKE_THROUGH;

    let mut faces = HashMap::new();
    faces.insert(FaceId::new(1), face);
    rif.set_faces(faces);

    let attrs = rif.resolve_attrs(FaceId::new(1));
    assert_eq!(attrs.fg, Some((255, 0, 0)));
    assert_eq!(attrs.bg, Some((0, 255, 0)));
    assert!(attrs.bold);
    assert!(attrs.italic);
    assert_eq!(attrs.underline, 3); // Wave
    assert!(attrs.strikethrough);
}

#[test]
fn resolve_attrs_falls_back_to_defaults_for_unknown_face() {
    let rif = TtyRif::new(80, 24);
    let attrs = rif.resolve_attrs(FaceId::new(999));
    // Should get default fg/bg.
    assert_eq!(attrs.fg, None);
    assert_eq!(attrs.bg, None);
    assert!(!attrs.bold);
    assert!(!attrs.italic);
}

#[test]
fn resolve_attrs_preserves_terminal_default_face_colors() {
    let mut rif = TtyRif::new(80, 24);
    let mut face = Face::new(FaceId::new(0));
    face.foreground = Color::rgb(0.0, 0.0, 0.0);
    face.background = Color::rgb(1.0, 1.0, 1.0);
    face.use_default_foreground = true;
    face.use_default_background = true;

    let mut faces = HashMap::new();
    faces.insert(FaceId::new(0), face);
    rif.set_faces(faces);

    let attrs = rif.resolve_attrs(FaceId::new(0));
    assert_eq!(attrs.fg, None);
    assert_eq!(attrs.bg, None);
}

// ---------------------------------------------------------------------------
// glyph_to_char
// ---------------------------------------------------------------------------

#[test]
fn glyph_to_char_returns_char_for_char_glyph() {
    let g = Glyph::char('Z', FaceId::new(0), 0);
    assert_eq!(glyph_to_char(&g), 'Z');
}

#[test]
fn glyph_to_char_returns_first_char_for_composite() {
    let g = Glyph {
        glyph_type: GlyphType::Composite { text: "ab".into() },
        face_id: FaceId::new(0),
        charpos: 0,
        bidi_level: 0,
        wide: false,
        pixel_width: 0.0,
        pixel_height: 0.0,
        pixel_ascent: 0.0,
        vertical_offset_px: 0.0,
        padding: false,
        pointer_appearance: None,
    };
    assert_eq!(glyph_to_char(&g), 'a');
}

#[test]
fn glyph_to_char_returns_space_for_stretch() {
    let g = Glyph::stretch(4, FaceId::new(0));
    assert_eq!(glyph_to_char(&g), ' ');
}

#[test]
fn surface_tty_placeholder_labels_and_fills_the_reserved_width() {
    // Exactly the label width: no fill.
    assert_eq!(surface_tty_placeholder(8), "[shader]");
    // Wider: label centered in a light-shade fill, exactly width_cols wide.
    let p = surface_tty_placeholder(12);
    assert_eq!(p.chars().count(), 12);
    assert_eq!(p, "░░[shader]░░");
    // Odd remainder: extra fill goes on the right.
    assert_eq!(surface_tty_placeholder(11), "░[shader]░░");
    // Too narrow for the label: pure fill, still visible (never blank).
    let narrow = surface_tty_placeholder(3);
    assert_eq!(narrow.chars().count(), 3);
    assert!(narrow.chars().all(|c| c == '░'));
    assert_eq!(surface_tty_placeholder(1), "░");
}

#[test]
fn rasterize_shows_placeholder_for_surface_glyph() {
    let cols = 12;
    let mut state = FrameDisplayState::new(cols, 3, 8.0, 16.0);
    state.background = Color::rgb(0.0, 0.0, 0.0);

    let mut matrix = GlyphMatrix::new(3, cols);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    // A shader-surface glyph spanning the full 12 columns (as push_media builds
    // it: a stretch glyph whose type is overwritten to Surface).
    let mut surface = Glyph::stretch(cols as u16, FaceId::new(0));
    surface.glyph_type = GlyphType::Surface {
        surface_id: 0x7000_0001u32 as i32,
        width_cols: cols as u16,
    };
    row.glyphs[GlyphArea::Text as usize].push(surface);
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * 8.0, 3.0 * 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * 8.0, 3.0 * 16.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(cols, 3);
    rif.rasterize(&state);

    // The reserved columns show the labeled placeholder, not blank space.
    let rendered: String = (0..cols).map(|c| desired_char(&rif, 0, c)).collect();
    assert_eq!(rendered, "░░[shader]░░");
}

// ---------------------------------------------------------------------------
// color_to_rgb8
// ---------------------------------------------------------------------------

/// `color_to_rgb8` applies `linear_to_srgb` before quantizing, so
/// a linear input of 0.5 becomes sRGB ~0.735 → 188 (not 127).
#[test]
fn color_to_rgb8_converts_correctly() {
    let c = Color::rgb(1.0, 0.5, 0.0);
    let (r, g, b) = color_to_rgb8(&c);
    assert_eq!(r, 255);
    // linear 0.5 → sRGB: 1.055 * 0.5^(1/2.4) - 0.055 ≈ 0.735 → 188
    assert_eq!(g, 188);
    assert_eq!(b, 0);
}

#[test]
fn color_to_rgb8_clamps_out_of_range() {
    let c = Color::rgb(2.0, -1.0, 0.5);
    let (r, g, b) = color_to_rgb8(&c);
    assert_eq!(r, 255);
    assert_eq!(g, 0);
    // linear 0.5 → sRGB ≈ 188
    assert_eq!(b, 188);
}

/// Round-trip: an sRGB pixel value → Color::from_pixel (srgb→linear)
/// → color_to_rgb8 (linear→srgb) should recover the original byte
/// values. This is the contract that makes TTY face colors match
/// GNU Emacs exactly.
#[test]
fn color_to_rgb8_round_trips_srgb_pixel() {
    // grey75 = sRGB 191 = 0xbfbfbf (GNU mode-line bg)
    let pixel = 0x00bfbfbf_u32;
    let linear = Color::from_pixel(pixel);
    let (r, g, b) = color_to_rgb8(&linear);
    assert_eq!(r, 191, "grey75 round-trip red channel");
    assert_eq!(g, 191, "grey75 round-trip green channel");
    assert_eq!(b, 191, "grey75 round-trip blue channel");

    // grey30 = sRGB 77 = 0x4d4d4d (GNU mode-line-inactive bg, dark)
    let pixel2 = 0x004d4d4d_u32;
    let linear2 = Color::from_pixel(pixel2);
    let (r2, g2, b2) = color_to_rgb8(&linear2);
    assert_eq!(r2, 77, "grey30 round-trip red channel");
    assert_eq!(g2, 77, "grey30 round-trip green channel");
    assert_eq!(b2, 77, "grey30 round-trip blue channel");
}

// ---------------------------------------------------------------------------
// rasterize
// ---------------------------------------------------------------------------

/// Helper: build a simple FrameDisplayState with one window containing
/// the given text on a single row.
fn make_simple_state(text: &str) -> FrameDisplayState {
    let cols = text.len().max(10);
    let mut state = FrameDisplayState::new(cols, 5, 8.0, 16.0);
    state.background = Color::rgb(0.0, 0.0, 0.0);

    let mut matrix = GlyphMatrix::new(5, cols);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in text.chars().enumerate() {
        row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), i));
    }
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * 8.0, 5.0 * 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * 8.0, 5.0 * 16.0),
        text_clip_bounds: None,
        selected: true,
    });
    state
}

fn make_grid_state(
    frame_id: u64,
    parent_id: u64,
    parent_x: f32,
    parent_y: f32,
    cols: usize,
    rows: usize,
    text: &str,
) -> FrameDisplayState {
    let mut state = FrameDisplayState::new(cols, rows, 1.0, 1.0);
    state.frame_placement = crate::PresentedFramePlacement::new(
        DisplayFrameId::new(frame_id),
        state.presentation_id,
        (parent_id != 0).then(|| DisplayFrameId::new(parent_id)),
        crate::ParentFrameRect::new(
            parent_x,
            parent_y,
            state.frame_pixel_width,
            state.frame_pixel_height,
        )
        .unwrap(),
        0,
    );
    state.background = Color::rgb(0.0, 0.0, 0.0);

    let mut matrix = GlyphMatrix::new(rows, cols);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in text.chars().take(cols).enumerate() {
        row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), i));
    }
    if rows > 0 {
        matrix.rows[0] = std::sync::Arc::new(row);
    }

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new((frame_id + 100) as i64),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, cols as f32, rows as f32),
        text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32, rows as f32),
        text_clip_bounds: None,
        selected: true,
    });
    state
}

fn desired_char(rif: &TtyRif, row: usize, col: usize) -> char {
    rif.desired.cells[row * rif.width() + col].ch
}

#[test]
fn rasterize_simple_text() {
    let mut rif = TtyRif::new(10, 5);
    let state = make_simple_state("Hello");
    rif.rasterize(&state);

    // First row should have "Hello" followed by spaces.
    assert_eq!(rif.desired.cells[0].ch, 'H');
    assert_eq!(rif.desired.cells[1].ch, 'e');
    assert_eq!(rif.desired.cells[2].ch, 'l');
    assert_eq!(rif.desired.cells[3].ch, 'l');
    assert_eq!(rif.desired.cells[4].ch, 'o');
    assert_eq!(rif.desired.cells[5].ch, ' '); // cleared to space
}

#[test]
fn rasterize_respects_matrix_position() {
    let mut state = FrameDisplayState::new(20, 10, 8.0, 16.0);
    state.background = Color::rgb(0.0, 0.0, 0.0);

    let mut matrix = GlyphMatrix::new(3, 10);
    matrix.matrix_x = 5;
    matrix.matrix_y = 2;
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('A', FaceId::new(0), 0));
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(40.0, 32.0, 80.0, 48.0),
        text_pixel_bounds: Rect::new(40.0, 32.0, 80.0, 48.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(20, 10);
    rif.rasterize(&state);

    // 'A' should be at row=2, col=5.
    let idx = 2 * 20 + 5;
    assert_eq!(rif.desired.cells[idx].ch, 'A');
    // row=0 col=0 should still be blank.
    assert_eq!(rif.desired.cells[0].ch, ' ');
}

#[test]
fn rasterize_face_fill_paints_blank_cells_before_glyphs() {
    let mut state = FrameDisplayState::new(8, 2, 1.0, 1.0);
    state.background = Color::from_pixel(0x000000);
    let mut fill_face = Face::new(FaceId::new(7));
    fill_face.background = Color::from_pixel(0x112233);
    let mut glyph_face = Face::new(FaceId::new(8));
    glyph_face.background = Color::from_pixel(0x445566);
    state.faces.insert(FaceId::new(7), fill_face);
    state.faces.insert(FaceId::new(8), glyph_face);
    state.face_fills.push(FaceFillItem {
        window_id: DisplayWindowId::new(1),
        row_role: GlyphRowRole::Text,
        clip_rect: Some(Rect::new(0.0, 0.0, 8.0, 2.0)),
        bounds: Rect::new(0.0, 0.0, 8.0, 1.0),
        face_id: FaceId::new(7),
    });

    let mut matrix = GlyphMatrix::new(1, 6);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('X', FaceId::new(8), 0));
    matrix.rows[0] = std::sync::Arc::new(row);
    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 8.0, 1.0),
        text_pixel_bounds: Rect::new(2.0, 0.0, 6.0, 1.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(8, 2);
    rif.rasterize(&state);

    assert_eq!(rif.desired.cells[0].ch, ' ');
    assert_eq!(rif.desired.cells[0].attrs.bg, Some((0x11, 0x22, 0x33)));
    assert_eq!(rif.desired.cells[1].attrs.bg, Some((0x11, 0x22, 0x33)));
    assert_eq!(rif.desired.cells[2].ch, 'X');
    assert_eq!(rif.desired.cells[2].attrs.bg, Some((0x44, 0x55, 0x66)));
    assert_eq!(rif.desired.cells[7].attrs.bg, Some((0x11, 0x22, 0x33)));
    assert_eq!(rif.desired.cells[8].attrs.bg, Some((0, 0, 0)));
}

#[test]
fn rasterize_uses_grid_rows_not_pixel_row_metrics() {
    let mut state = FrameDisplayState::new(12, 5, 1.0, 1.0);
    state.background = Color::rgb(0.0, 0.0, 0.0);

    let mut matrix = GlyphMatrix::new(3, 12);
    for (row_idx, ch) in ['A', 'B', 'C'].into_iter().enumerate() {
        let mut row = GlyphRow::new(GlyphRowRole::Text);
        row.pixel_y = row_idx as f32 * 13.0;
        row.height_px = 13.0;
        row.ascent_px = 10.0;
        row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), row_idx));
        matrix.rows[row_idx] = std::sync::Arc::new(row);
    }

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 12.0, 5.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 12.0, 5.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(12, 5);
    rif.rasterize(&state);

    assert_eq!(desired_char(&rif, 0, 0), 'A');
    assert_eq!(desired_char(&rif, 1, 0), 'B');
    assert_eq!(desired_char(&rif, 2, 0), 'C');
}

#[test]
fn rasterize_text_rows_use_text_pixel_bounds_but_chrome_rows_do_not() {
    let mut state = FrameDisplayState::new(12, 3, 8.0, 16.0);
    state.background = Color::rgb(0.0, 0.0, 0.0);

    let mut matrix = GlyphMatrix::new(2, 9);

    let mut text_row = GlyphRow::new(GlyphRowRole::Text);
    text_row.glyphs[GlyphArea::Text as usize].push(Glyph::char('T', FaceId::new(0), 0));
    matrix.rows[0] = std::sync::Arc::new(text_row);

    let mut mode_line_row = GlyphRow::new(GlyphRowRole::ModeLine);
    mode_line_row.glyphs[GlyphArea::Text as usize].push(Glyph::char('M', FaceId::new(0), 0));
    matrix.rows[1] = std::sync::Arc::new(mode_line_row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 96.0, 48.0),
        text_pixel_bounds: Rect::new(24.0, 0.0, 72.0, 32.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(12, 3);
    rif.rasterize(&state);

    assert_eq!(desired_char(&rif, 0, 0), ' ');
    assert_eq!(desired_char(&rif, 0, 3), 'T');
    assert_eq!(desired_char(&rif, 1, 0), 'M');
}

#[test]
fn rasterize_frame_tree_draws_decorated_child_in_z_order() {
    let root = make_grid_state(1, 0, 0.0, 0.0, 12, 6, "root");
    let child = make_grid_state(2, 1, 4.0, 2.0, 3, 1, "M-x");

    let mut rif = TtyRif::new(12, 6);
    rif.rasterize_frame_tree(&root, &[child]);

    assert_eq!(desired_char(&rif, 1, 3), '+');
    assert_eq!(desired_char(&rif, 1, 4), '-');
    assert_eq!(desired_char(&rif, 1, 6), '-');
    assert_eq!(desired_char(&rif, 1, 7), '+');
    assert_eq!(desired_char(&rif, 2, 3), '|');
    assert_eq!(desired_char(&rif, 2, 4), 'M');
    assert_eq!(desired_char(&rif, 2, 5), '-');
    assert_eq!(desired_char(&rif, 2, 6), 'x');
    assert_eq!(desired_char(&rif, 2, 7), '|');
    assert_eq!(desired_char(&rif, 3, 3), '+');
    assert_eq!(desired_char(&rif, 3, 4), '-');
    assert_eq!(desired_char(&rif, 3, 7), '+');
}

#[test]
fn rasterize_frame_tree_skips_border_for_undecorated_child() {
    let root = make_grid_state(1, 0, 0.0, 0.0, 12, 6, "root");
    let mut child = make_grid_state(2, 1, 4.0, 2.0, 3, 1, "M-x");
    child.undecorated = true;

    let mut rif = TtyRif::new(12, 6);
    rif.rasterize_frame_tree(&root, &[child]);

    assert_eq!(desired_char(&rif, 1, 3), ' ');
    assert_eq!(desired_char(&rif, 1, 4), ' ');
    assert_eq!(desired_char(&rif, 1, 7), ' ');
    assert_eq!(desired_char(&rif, 2, 3), ' ');
    assert_eq!(desired_char(&rif, 2, 4), 'M');
    assert_eq!(desired_char(&rif, 2, 5), '-');
    assert_eq!(desired_char(&rif, 2, 6), 'x');
    assert_eq!(desired_char(&rif, 2, 7), ' ');
    assert_eq!(desired_char(&rif, 3, 3), ' ');
    assert_eq!(desired_char(&rif, 3, 4), ' ');
    assert_eq!(desired_char(&rif, 3, 7), ' ');
}

#[test]
fn rasterize_frame_tree_clips_negative_child_origin_without_shifting_its_content() {
    let root = make_grid_state(1, 0, 0.0, 0.0, 6, 3, "root");
    let mut child = make_grid_state(2, 1, -1.0, 0.0, 3, 1, "ABC");
    child.undecorated = true;

    let mut rif = TtyRif::new(6, 3);
    rif.rasterize_frame_tree(&root, &[child]);

    assert_eq!(desired_char(&rif, 0, 0), 'B');
    assert_eq!(desired_char(&rif, 0, 1), 'C');
    assert_eq!(desired_char(&rif, 0, 2), 'o');
}

#[test]
fn rasterize_frame_tree_clips_negative_child_rows_without_shifting_the_source_row() {
    let root = make_grid_state(1, 0, 0.0, 0.0, 6, 3, "root");
    let mut child = make_grid_state(2, 1, 0.0, -1.0, 2, 2, "AA");
    child.undecorated = true;
    let mut second_row = GlyphRow::new(GlyphRowRole::Text);
    second_row.glyphs[GlyphArea::Text as usize].push(Glyph::char('B', FaceId::new(0), 0));
    second_row.glyphs[GlyphArea::Text as usize].push(Glyph::char('B', FaceId::new(0), 1));
    child.window_matrices[0].matrix.rows[1] = std::sync::Arc::new(second_row);

    let mut rif = TtyRif::new(6, 3);
    rif.rasterize_frame_tree(&root, &[child]);

    assert_eq!(desired_char(&rif, 0, 0), 'B');
    assert_eq!(desired_char(&rif, 0, 1), 'B');
    assert_eq!(desired_char(&rif, 1, 0), ' ');
}

#[test]
fn rasterize_frame_tree_clips_a_decorated_child_at_the_left_edge() {
    let root = make_grid_state(1, 0, 0.0, 0.0, 6, 4, "root");
    let child = make_grid_state(2, 1, -1.0, 1.0, 3, 1, "ABC");

    let mut rif = TtyRif::new(6, 4);
    rif.rasterize_frame_tree(&root, &[child]);

    assert_eq!(
        [
            desired_char(&rif, 0, 0),
            desired_char(&rif, 0, 1),
            desired_char(&rif, 0, 2),
        ],
        ['-', '-', '+']
    );
    assert_eq!(
        [
            desired_char(&rif, 1, 0),
            desired_char(&rif, 1, 1),
            desired_char(&rif, 1, 2),
        ],
        ['B', 'C', '|']
    );
    assert_eq!(
        [
            desired_char(&rif, 2, 0),
            desired_char(&rif, 2, 1),
            desired_char(&rif, 2, 2),
        ],
        ['-', '-', '+']
    );
}

#[test]
fn rasterize_fully_clipped_decorated_child_suppresses_edge_only_border_like_gnu() {
    let root = make_grid_state(1, 0, 0.0, 0.0, 6, 3, "root");
    let child = make_grid_state(2, 1, -3.0, 1.0, 3, 1, "ABC");

    let mut rif = TtyRif::new(6, 3);
    rif.rasterize_frame_tree(&root, &[child]);

    // GNU's copy_child_glyphs returns before drawing borders when the child
    // frame rectangle has no interior intersection with the root.
    assert_eq!(desired_char(&rif, 1, 0), ' ');
}

#[test]
fn rasterize_frame_tree_hides_a_child_cursor_clipped_off_the_left_edge() {
    let root = make_grid_state(1, 0, 0.0, 0.0, 6, 3, "root");
    let mut child = make_grid_state(2, 1, -2.0, 0.0, 3, 1, "ABC");
    child.undecorated = true;
    child.phys_cursor = Some(PhysCursor {
        window_id: DisplayWindowId::new(102),
        charpos: 0,
        row: 0,
        col: 0,
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
        ascent: 1.0,
        style: CursorStyle::FilledBox,
        color: Color::WHITE,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(102),
            row: 0,
            col: 0,
        },
        cursor_fg: Color::BLACK,
    });

    let mut rif = TtyRif::new(6, 3);
    rif.rasterize_frame_tree(&root, &[child]);

    assert!(!rif.cursor_visible);
}

#[test]
fn clipping_a_wide_child_glyph_does_not_leave_an_unrenderable_padding_cell() {
    let root = make_grid_state(1, 0, 0.0, 0.0, 4, 2, "root");
    let mut child = make_grid_state(2, 1, -1.0, 0.0, 2, 1, "");
    child.undecorated = true;
    let row = std::sync::Arc::make_mut(&mut child.window_matrices[0].matrix.rows[0]);
    let mut wide = Glyph::char('\u{4f60}', FaceId::new(0), 0);
    wide.wide = true;
    row.glyphs[GlyphArea::Text as usize].push(wide);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::padding_for(FaceId::new(0), 0));

    let mut rif = TtyRif::new(4, 2);
    rif.rasterize_frame_tree(&root, &[child]);

    assert_eq!(desired_char(&rif, 0, 0), ' ');
    assert!(!rif.desired.cells[0].padding);
}

#[test]
fn rasterize_disabled_rows_are_skipped() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::rgb(0.0, 0.0, 0.0);

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('X', FaceId::new(0), 0));
    row.enabled = false;
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    // Row 0 should be blank because the glyph row is disabled.
    assert_eq!(rif.desired.cells[0].ch, ' ');
}

// ---------------------------------------------------------------------------
// Wide character handling
// ---------------------------------------------------------------------------

#[test]
fn rasterize_wide_char_creates_padding() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    // CJK character, wide=true.
    let mut g = Glyph::char('\u{4e16}', FaceId::new(0), 0); // Unicode: "world" in Chinese
    g.wide = true;
    row.glyphs[GlyphArea::Text as usize].push(g);
    // Followed by a normal char.
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('!', FaceId::new(0), 1));
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    // Col 0: the wide char.
    assert_eq!(rif.desired.cells[0].ch, '\u{4e16}');
    assert!(!rif.desired.cells[0].padding);
    // Col 1: padding cell.
    assert!(rif.desired.cells[1].padding);
    // Col 2: '!'
    assert_eq!(rif.desired.cells[2].ch, '!');
    assert!(!rif.desired.cells[2].padding);
}

#[test]
fn rasterize_explicit_padding_glyph_is_not_duplicated() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    let mut wide = Glyph::char('\u{4f60}', FaceId::new(0), 0);
    wide.wide = true;
    row.glyphs[GlyphArea::Text as usize].push(wide);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::padding_for(FaceId::new(0), 0));
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('!', FaceId::new(0), 1));
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert_eq!(rif.desired.cells[0].ch, '\u{4f60}');
    assert!(rif.desired.cells[1].padding);
    assert_eq!(rif.desired.cells[2].ch, '!');
    assert!(!rif.desired.cells[2].padding);
}

#[test]
fn rasterize_stretch_glyph_uses_declared_width() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('A', FaceId::new(0), 0));
    row.glyphs[GlyphArea::Text as usize].push(Glyph::stretch(4, FaceId::new(0)));
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('B', FaceId::new(0), 1));
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert_eq!(rif.desired.cells[0].ch, 'A');
    assert_eq!(rif.desired.cells[1].ch, ' ');
    assert_eq!(rif.desired.cells[2].ch, ' ');
    assert_eq!(rif.desired.cells[3].ch, ' ');
    assert_eq!(rif.desired.cells[4].ch, ' ');
    assert_eq!(rif.desired.cells[5].ch, 'B');
}

// ---------------------------------------------------------------------------
// Cursor tracking
// ---------------------------------------------------------------------------

#[test]
fn rasterize_tracks_phys_cursor_position() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(5, 10);
    matrix.matrix_x = 0;
    matrix.matrix_y = 0;
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', FaceId::new(0), 0));
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('b', FaceId::new(0), 1));
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });
    state.phys_cursor = Some(PhysCursor {
        window_id: DisplayWindowId::new(1),
        charpos: 1,
        row: 0,
        col: 1,
        x: 8.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: Color::WHITE,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(1),
            row: 0,
            col: 1,
        },
        cursor_fg: Color::BLACK,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert!(rif.cursor_visible);
    assert_eq!(rif.cursor_row, 0);
    assert_eq!(rif.cursor_col, 1);
    assert_eq!(rif.cursor_shape, TerminalCursorShape::Block);
}

#[test]
fn rasterize_prefers_phys_cursor_over_matrix_cursor_columns() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row0 = GlyphRow::new(GlyphRowRole::Text);
    row0.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', FaceId::new(0), 0));
    row0.cursor_col = Some(1);
    row0.cursor_type = Some(CursorStyle::FilledBox);
    matrix.rows[0] = std::sync::Arc::new(row0);

    let mut row1 = GlyphRow::new(GlyphRowRole::Text);
    row1.glyphs[GlyphArea::Text as usize].push(Glyph::char('b', FaceId::new(0), 1));
    matrix.rows[1] = std::sync::Arc::new(row1);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });
    state.phys_cursor = Some(PhysCursor {
        window_id: DisplayWindowId::new(1),
        charpos: 1,
        row: 1,
        col: 4,
        x: 32.0,
        y: 16.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: Color::WHITE,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(1),
            row: 1,
            col: 4,
        },
        cursor_fg: Color::BLACK,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert!(rif.cursor_visible);
    assert_eq!(rif.cursor_row, 1);
    assert_eq!(rif.cursor_col, 4);
}

#[test]
fn tty_frame_chrome_rasterizes_menu_and_tab_bands_in_order() {
    use crate::frame_chrome::{
        BandRect, ChromeAction, ChromeBandRequest, ChromeDisplayRow, FrameChrome,
        FrameChromeContent, FrameChromeKind, FrameSize, MenuBarContent, PositionedChromeItem,
    };
    use crate::ui_types::MenuBarItem;

    let mut state = FrameDisplayState::new(10, 5, 1.0, 1.0);
    state.background = Color::BLACK;

    let mut row = GlyphRow::new(GlyphRowRole::TabBar);
    row.enabled = true;
    row.mode_line = true;
    row.displays_text = true;
    row.height_px = 1.0;
    row.ascent_px = 1.0;
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('T', FaceId::new(0), 0));
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('B', FaceId::new(0), 1));

    let menu = MenuBarContent::new(
        vec![PositionedChromeItem::new(
            BandRect::new(0.0, 0.0, 5.0, 1.0).expect("menu item bounds"),
            MenuBarItem {
                index: 0,
                label: "File".into(),
                key: "file".into(),
            },
            ChromeAction::OpenMenu {
                index: 0,
                key: "file".into(),
            },
        )],
        Color::WHITE,
        Color::BLACK,
    );
    state.frame_chrome = FrameChrome::layout(
        FrameSize::new(10.0, 5.0).expect("frame size"),
        vec![
            ChromeBandRequest::new(
                FrameChromeKind::MenuBar,
                1.0,
                FrameChromeContent::MenuBar(menu),
            ),
            ChromeBandRequest::new(
                FrameChromeKind::TabBar,
                1.0,
                FrameChromeContent::DisplayRow(ChromeDisplayRow::new(row)),
            ),
        ],
    )
    .expect("frame chrome");

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert_eq!(rif.desired.cells[0].ch, 'F');
    assert_eq!(rif.desired.cells[1].ch, 'i');
    assert_eq!(rif.desired.cells[10].ch, 'T');
    assert_eq!(rif.desired.cells[11].ch, 'B');
}

#[test]
fn rasterize_ignores_matrix_cursor_columns_without_phys_cursor() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', FaceId::new(0), 0));
    row.cursor_col = Some(1);
    row.cursor_type = Some(CursorStyle::FilledBox);
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert!(!rif.cursor_visible);
}

#[test]
fn rasterize_keeps_phys_filled_box_cursor_out_of_cell_attrs() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;
    let mut default_face = Face::new(FaceId::new(0));
    default_face.use_default_foreground = true;
    default_face.use_default_background = true;
    state.faces.insert(FaceId::new(0), default_face);

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', FaceId::new(0), 0));
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('b', FaceId::new(0), 1));
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });
    state.phys_cursor = Some(PhysCursor {
        window_id: DisplayWindowId::new(1),
        charpos: 1,
        row: 0,
        col: 1,
        x: 8.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: Color::RED,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(1),
            row: 0,
            col: 1,
        },
        cursor_fg: Color::BLACK,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    let cell = &rif.desired.cells[1];
    assert_eq!(cell.ch, 'b');
    assert_eq!(cell.attrs.bg, None);
    assert_eq!(cell.attrs.fg, None);
    assert!(rif.cursor_visible);
    assert_eq!(rif.cursor_row, 0);
    assert_eq!(rif.cursor_col, 1);
}

#[test]
fn rasterize_ignores_nonselected_hollow_cursor_visual_on_tty() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(2, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('x', FaceId::new(0), 0));
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('y', FaceId::new(0), 1));
    row.cursor_col = Some(1);
    row.cursor_type = Some(CursorStyle::Hollow);
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(9),
        matrix,
        pixel_bounds: Rect::new(0.0, 16.0, 80.0, 32.0),
        text_pixel_bounds: Rect::new(0.0, 16.0, 80.0, 32.0),
        text_clip_bounds: None,
        selected: false,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    let row_start = rif.width();
    let cell = &rif.desired.cells[row_start + 1];
    assert_eq!(cell.ch, 'y');
    assert!(!cell.attrs.inverse);
    assert!(!rif.cursor_visible);
}

#[test]
fn rasterize_uses_hardware_bar_shape_for_phys_bar_cursor() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', FaceId::new(0), 0));
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });
    state.phys_cursor = Some(PhysCursor {
        window_id: DisplayWindowId::new(1),
        charpos: 0,
        row: 0,
        col: 0,
        x: 0.0,
        y: 0.0,
        width: 2.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::Bar(2.0),
        color: Color::WHITE,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(1),
            row: 0,
            col: 0,
        },
        cursor_fg: Color::BLACK,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert!(rif.cursor_visible);
    assert_eq!(rif.cursor_shape, TerminalCursorShape::Bar);
    assert!(!rif.desired.cells[0].attrs.inverse);
}

/// Regression test for a bug observed after `C-x 2` in an
/// interactive `neomacs -nw -Q` session: the physical terminal
/// cursor ended up inside the newly-created (non-selected)
/// bottom window because the TTY RIF iterated both windows'
/// glyph matrices and let the LAST `cursor_col` it saw win,
/// clobbering the selected window's cursor with the hollow
/// cursor hint drawn for the non-selected window.
///
/// GNU Emacs has a dedicated `tty_set_cursor` in
/// `src/dispnew.c:5670-5751` that explicitly uses
/// `FRAME_SELECTED_WINDOW (f)` and only calls `cursor_to` once,
/// with this comment:
///
///   /* We have only one cursor on terminal frames. Use it to
///      display the cursor of the selected window of the
///      frame.  */
///   struct window *w = XWINDOW (FRAME_SELECTED_WINDOW (f));
///   ...
///   cursor_to (f, y, x);
///
/// The `selected: bool` field on `WindowMatrixEntry` is the
/// per-frame-state equivalent of GNU's `FRAME_SELECTED_WINDOW`
/// check: only the selected window contributes the frame-level
/// `phys_cursor` used for the terminal cursor geometry/position.
/// Non-selected windows may still mark `cursor_col`, but on TTY
/// frames GNU has no per-window cursor painting path; only the
/// frame-level terminal cursor is moved.
#[test]
fn rasterize_terminal_cursor_comes_from_selected_window_only() {
    // Two vertically stacked 2-row windows at screen cols 0..10.
    // Top window (w1) is selected; its cursor is in row 0, col 3.
    // Bottom window (w2) is NOT selected but still has a
    // hollow cursor marker in its row 0, col 7. The terminal
    // cursor MUST come from w1.
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut top_matrix = GlyphMatrix::new(2, 10);
    let mut top_row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in "TOP-BUFFER".chars().enumerate() {
        top_row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), i));
    }
    top_row.cursor_col = Some(3);
    top_row.cursor_type = Some(CursorStyle::FilledBox);
    top_matrix.rows[0] = std::sync::Arc::new(top_row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix: top_matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 32.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 32.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut bot_matrix = GlyphMatrix::new(2, 10);
    let mut bot_row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in "BOT-BUFFER".chars().enumerate() {
        bot_row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), i));
    }
    // Non-selected window still marks a hollow cursor column via
    // the same `cursor_col` slot, reflecting the `Hollow` style
    // chosen by `cursor_style_for_window` for windows where
    // `cursor-in-non-selected-windows` is non-nil.
    bot_row.cursor_col = Some(7);
    bot_row.cursor_type = Some(CursorStyle::Hollow);
    bot_matrix.rows[0] = std::sync::Arc::new(bot_row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(2),
        matrix: bot_matrix,
        // Bottom half of the screen.
        pixel_bounds: Rect::new(0.0, 32.0, 80.0, 32.0),
        text_pixel_bounds: Rect::new(0.0, 32.0, 80.0, 32.0),
        text_clip_bounds: None,
        selected: false,
    });
    state.phys_cursor = Some(PhysCursor {
        window_id: DisplayWindowId::new(1),
        charpos: 3,
        row: 0,
        col: 3,
        x: 24.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: Color::WHITE,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(1),
            row: 0,
            col: 3,
        },
        cursor_fg: Color::BLACK,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert!(
        rif.cursor_visible,
        "TTY filled-box cursor should use the hardware cursor"
    );
    assert_eq!(
        rif.cursor_row, 0,
        "cursor row must come from selected (top) window"
    );
    assert_eq!(
        rif.cursor_col, 3,
        "cursor column must come from selected (top) window — \
         the non-selected bottom window's hollow cursor at col 7 \
         must NOT move the frame-level cursor geometry"
    );
}

/// Complementary test: when the frame layout lists the selected
/// window AFTER a non-selected window, the terminal cursor must
/// still come from the selected window. Without the
/// `entry.selected` guard this case happens to succeed by
/// accident (last-writer-wins lands on the selected window), so
/// we verify it explicitly to pin the intent rather than the
/// iteration order.
#[test]
fn rasterize_terminal_cursor_comes_from_selected_window_regardless_of_order() {
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    // First entry: non-selected window with a hollow cursor.
    let mut w1_matrix = GlyphMatrix::new(2, 10);
    let mut w1_row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in "FIRST-WIN".chars().enumerate() {
        w1_row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), i));
    }
    w1_row.cursor_col = Some(9);
    w1_row.cursor_type = Some(CursorStyle::Hollow);
    w1_matrix.rows[0] = std::sync::Arc::new(w1_row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix: w1_matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 32.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 32.0),
        text_clip_bounds: None,
        selected: false,
    });

    // Second entry: the selected window with its real cursor.
    let mut w2_matrix = GlyphMatrix::new(2, 10);
    let mut w2_row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in "SECND-WIN".chars().enumerate() {
        w2_row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), i));
    }
    w2_row.cursor_col = Some(2);
    w2_row.cursor_type = Some(CursorStyle::FilledBox);
    w2_matrix.rows[0] = std::sync::Arc::new(w2_row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(2),
        matrix: w2_matrix,
        pixel_bounds: Rect::new(0.0, 32.0, 80.0, 32.0),
        text_pixel_bounds: Rect::new(0.0, 32.0, 80.0, 32.0),
        text_clip_bounds: None,
        selected: true,
    });
    state.phys_cursor = Some(PhysCursor {
        window_id: DisplayWindowId::new(2),
        charpos: 2,
        row: 2,
        col: 2,
        x: 16.0,
        y: 32.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: Color::WHITE,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(2),
            row: 2,
            col: 2,
        },
        cursor_fg: Color::BLACK,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert!(rif.cursor_visible);
    assert_eq!(rif.cursor_row, 2, "selected window starts at screen row 2");
    assert_eq!(rif.cursor_col, 2, "cursor col from selected window only");
}

// ---------------------------------------------------------------------------
// diff_and_render
// ---------------------------------------------------------------------------

#[test]
fn first_diff_repaints_unknown_terminal() {
    let mut rif = TtyRif::new(10, 5);
    // A fresh terminal's real contents are unknown.  GNU marks new/resized
    // frames garbaged and repaints them before relying on matrix diffs.
    rif.diff_and_render();
    let output = rif.take_output();

    let s = String::from_utf8_lossy(&output);
    assert!(s.contains("\x1b[?25l")); // hide cursor
    assert!(s.contains("\x1b[0m")); // reset

    // The first render repaints every row with one CUP per contiguous row run.
    let cup_count = s.matches("H").count();
    assert!(
        cup_count == 5,
        "Expected 5 CUP moves for initial full repaint, got {}",
        cup_count
    );
}

#[test]
fn diff_and_render_emits_hardware_cursor_shape_for_bar_cursor() {
    let mut rif = TtyRif::new(10, 5);
    rif.desired.set(0, 0, 'A', CellAttrs::default(), false);
    rif.cursor_visible = true;
    rif.cursor_row = 0;
    rif.cursor_col = 0;
    rif.cursor_shape = TerminalCursorShape::Bar;

    rif.diff_and_render();
    let output = String::from_utf8(rif.take_output()).expect("utf8 output");

    assert!(output.contains("\x1b[1;1H"));
    assert!(output.contains("\x1b[6 q"));
    assert!(output.contains("\x1b[?25h"));
}

#[test]
fn diff_with_changes_produces_ansi_sequences() {
    let mut rif = TtyRif::new(10, 5);
    // Write something into the desired grid.
    rif.desired.set(
        0,
        0,
        'A',
        CellAttrs {
            fg: Some((255, 0, 0)),
            ..CellAttrs::default()
        },
        false,
    );
    rif.diff_and_render();
    let output = rif.take_output();
    let s = String::from_utf8_lossy(&output);

    // Should contain CUP to row 1, col 1 (1-based).
    assert!(s.contains("\x1b[1;1H"), "Missing CUP: {}", s);
    // Should contain the character 'A'.
    assert!(s.contains('A'), "Missing character A: {}", s);
    // Should contain true-color foreground sequence for red.
    assert!(s.contains("\x1b[38;2;255;0;0m"), "Missing fg color: {}", s);
}

#[test]
fn diff_and_render_emits_wide_glyphs_as_one_row_run() {
    let mut rif = TtyRif::new(10, 5);
    let attrs = CellAttrs::default();

    rif.desired.set(0, 0, '你', attrs, false);
    rif.desired.set(0, 1, ' ', attrs, true);
    rif.desired.set(0, 2, '好', attrs, false);
    rif.desired.set(0, 3, ' ', attrs, true);
    rif.desired.set(0, 4, ',', attrs, false);

    rif.diff_and_render();
    let output = String::from_utf8(rif.take_output()).expect("utf8 output");

    assert!(output.contains("\x1b[1;1H"));
    assert!(output.contains("你好,"));
    assert!(!output.contains("\x1b[1;3H"));
    assert!(!output.contains("\x1b[1;5H"));
}

#[test]
fn diff_and_render_rewrites_changed_row_span_contiguously() {
    let mut rif = TtyRif::new(10, 5);
    let attrs = CellAttrs::default();

    for (col, ch) in ['A', 'B', 'C', 'D', 'E'].into_iter().enumerate() {
        rif.desired.set(0, col, ch, attrs, false);
    }
    rif.diff_and_render();
    let _ = rif.take_output();

    rif.desired = rif.current.clone();
    rif.desired.set(0, 0, 'X', attrs, false);
    rif.desired.set(0, 4, 'Y', attrs, false);
    rif.diff_and_render();
    let output = String::from_utf8(rif.take_output()).expect("utf8 output");

    assert!(output.contains("\x1b[1;1H"));
    assert!(output.contains("XBCDY"));
    assert_eq!(
        output.matches("\x1b[1;").count(),
        1,
        "expected a single CUP run"
    );
}

#[test]
fn diff_and_render_preclears_composite_cell_rewrites() {
    let mut rif = TtyRif::new(10, 5);
    let attrs = CellAttrs::default();

    rif.desired
        .set_cluster(0, 0, 'A', "\u{0301}\u{0302}", attrs, false);
    rif.desired.set(0, 1, 'B', attrs, false);
    rif.diff_and_render();
    let _ = rif.take_output();

    rif.desired = rif.current.clone();
    rif.desired.set_cluster(0, 0, 'A', "\u{0301}", attrs, false);
    rif.diff_and_render();
    let output = String::from_utf8(rif.take_output()).expect("utf8 output");

    let first_goto = output
        .find("\x1b[1;1H")
        .expect("composite rewrite should move to the changed cell for preclear");
    let clear_space = output[first_goto..]
        .find(' ')
        .map(|offset| first_goto + offset)
        .expect("composite rewrite should emit a clearing space");
    let second_goto = output[clear_space..]
        .find("\x1b[1;1H")
        .map(|offset| clear_space + offset)
        .expect("composite rewrite should move back before repainting");
    assert!(
        first_goto < clear_space && clear_space < second_goto,
        "composite rewrites should clear the changed cell before repainting: {output:?}"
    );
    assert!(
        output.contains("A\u{0301}"),
        "replacement composite should still be painted after preclear: {output:?}"
    );
}

#[test]
fn diff_swaps_current_and_desired() {
    let mut rif = TtyRif::new(10, 5);
    rif.desired.set(0, 0, 'X', CellAttrs::default(), false);
    rif.diff_and_render();

    // After diff, current should have 'X' at (0,0).
    assert_eq!(rif.current.cells[0].ch, 'X');
}

#[test]
fn second_diff_with_same_content_is_minimal() {
    let mut rif = TtyRif::new(10, 5);
    rif.desired.set(0, 0, 'Q', CellAttrs::default(), false);
    rif.diff_and_render();

    // Set the desired to the same content again.
    rif.desired.set(0, 0, 'Q', CellAttrs::default(), false);
    rif.diff_and_render();
    let output = rif.take_output();
    let s = String::from_utf8_lossy(&output);

    // Since desired == current, no cell CUP moves.
    // Only hide cursor + reset + possibly show cursor.
    let cup_count = s.matches("H").count();
    assert!(
        cup_count == 0,
        "Expected 0 CUP for identical frames, got {}",
        cup_count
    );
}

// ---------------------------------------------------------------------------
// Cursor visibility in output
// ---------------------------------------------------------------------------

#[test]
fn cursor_visible_emits_show_cursor_sequence() {
    let mut rif = TtyRif::new(10, 5);
    rif.cursor_visible = true;
    rif.cursor_row = 3;
    rif.cursor_col = 7;
    rif.diff_and_render();
    let output = rif.take_output();
    let s = String::from_utf8_lossy(&output);

    // Should show cursor.
    assert!(s.contains("\x1b[?25h"), "Missing show cursor: {}", s);
    // Should position cursor at (4, 8) (1-based).
    assert!(s.contains("\x1b[4;8H"), "Missing cursor position: {}", s);
}

#[test]
fn cursor_not_visible_omits_show_cursor_sequence() {
    let mut rif = TtyRif::new(10, 5);
    rif.cursor_visible = false;
    rif.diff_and_render();
    let output = rif.take_output();
    let s = String::from_utf8_lossy(&output);

    assert!(
        !s.contains("\x1b[?25h"),
        "Show cursor should not appear: {}",
        s
    );
}

// ---------------------------------------------------------------------------
// SGR sequences
// ---------------------------------------------------------------------------

#[test]
fn write_sgr_bold_italic_underline() {
    let attrs = CellAttrs {
        fg: Some((0, 0, 0)),
        bg: Some((255, 255, 255)),
        bold: true,
        italic: true,
        underline: 1,
        strikethrough: false,
        inverse: false,
    };
    let mut buf = Vec::new();
    write_sgr(&mut buf, &attrs);
    let s = String::from_utf8_lossy(&buf);

    assert!(s.contains("\x1b[0m"), "Missing reset");
    assert!(s.contains("\x1b[1m"), "Missing bold");
    assert!(s.contains("\x1b[3m"), "Missing italic");
    assert!(s.contains("\x1b[4m"), "Missing underline");
}

#[test]
fn write_sgr_underline_styles_match_gnu_smulx_codes() {
    let styles = [
        (UnderlineStyle::Line, "\x1b[4m"),
        (UnderlineStyle::Double, "\x1b[4:2m"),
        (UnderlineStyle::Wave, "\x1b[4:3m"),
        (UnderlineStyle::Dotted, "\x1b[4:4m"),
        (UnderlineStyle::Dashed, "\x1b[4:5m"),
    ];

    for (style, escape) in styles {
        let attrs = CellAttrs {
            underline: style.gnu_code(),
            ..CellAttrs::default()
        };
        let mut buf = Vec::new();
        write_sgr(&mut buf, &attrs);
        let s = String::from_utf8_lossy(&buf);
        assert!(s.contains(escape), "{style:?} did not emit {escape:?}");
    }
}

#[test]
fn write_sgr_strikethrough_inverse() {
    let attrs = CellAttrs {
        fg: Some((0, 0, 0)),
        bg: Some((0, 0, 0)),
        bold: false,
        italic: false,
        underline: 0,
        strikethrough: true,
        inverse: true,
    };
    let mut buf = Vec::new();
    write_sgr(&mut buf, &attrs);
    let s = String::from_utf8_lossy(&buf);

    assert!(s.contains("\x1b[9m"), "Missing strikethrough");
    assert!(s.contains("\x1b[7m"), "Missing inverse");
}

#[test]
fn write_sgr_terminal_default_colors() {
    let attrs = CellAttrs {
        fg: None,
        bg: None,
        bold: false,
        italic: false,
        underline: 0,
        strikethrough: false,
        inverse: false,
    };
    let mut buf = Vec::new();
    write_sgr(&mut buf, &attrs);
    let s = String::from_utf8_lossy(&buf);

    assert!(s.contains("\x1b[39m"), "Missing default foreground reset");
    assert!(s.contains("\x1b[49m"), "Missing default background reset");
    assert!(
        !s.contains("\x1b[38;2;"),
        "Terminal-default foreground should not emit explicit RGB SGR: {s:?}"
    );
    assert!(
        !s.contains("\x1b[48;2;"),
        "Terminal-default background should not emit explicit RGB SGR: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// TtyGrid
// ---------------------------------------------------------------------------

#[test]
fn grid_clear_sets_background() {
    let mut grid = TtyGrid::new(5, 3);
    grid.clear(Some((100, 50, 25)));
    for cell in &grid.cells {
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.attrs.bg, Some((100, 50, 25)));
    }
}

#[test]
fn grid_set_out_of_bounds_is_noop() {
    let mut grid = TtyGrid::new(5, 3);
    // Should not panic.
    grid.set(100, 100, 'X', CellAttrs::default(), false);
    // All cells still blank.
    for cell in &grid.cells {
        assert_eq!(cell.ch, ' ');
    }
}

// ---------------------------------------------------------------------------
// take_output
// ---------------------------------------------------------------------------

#[test]
fn take_output_clears_buffer() {
    let mut rif = TtyRif::new(10, 5);
    rif.desired.set(0, 0, 'A', CellAttrs::default(), false);
    rif.diff_and_render();

    let first = rif.take_output();
    assert!(!first.is_empty());

    let second = rif.take_output();
    assert!(second.is_empty());
}

// ---------------------------------------------------------------------------
// Full round-trip: rasterize + diff_and_render
// ---------------------------------------------------------------------------

#[test]
fn full_round_trip_simple_text() {
    let mut rif = TtyRif::new(10, 5);
    let state = make_simple_state("Hi");
    rif.rasterize(&state);
    rif.diff_and_render();
    let output = rif.take_output();
    let s = String::from_utf8_lossy(&output);

    // Should contain 'H' and 'i' somewhere in the output.
    assert!(s.contains('H'), "Missing H in output");
    assert!(s.contains('i'), "Missing i in output");
}

// ---------------------------------------------------------------------------
// Complex-run (Arabic/Indic) TTY decomposition
// ---------------------------------------------------------------------------

/// Build a one-row state whose text area is `glyphs`.
fn state_with_text_glyphs(cols: usize, glyphs: Vec<Glyph>) -> FrameDisplayState {
    let mut state = FrameDisplayState::new(cols, 5, 8.0, 16.0);
    state.background = Color::rgb(0.0, 0.0, 0.0);
    let mut matrix = GlyphMatrix::new(5, cols);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.glyphs[GlyphArea::Text as usize] = glyphs;
    matrix.rows[0] = std::sync::Arc::new(row);
    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * 8.0, 5.0 * 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * 8.0, 5.0 * 16.0),
        text_clip_bounds: None,
        selected: true,
    });
    state
}

fn run_composite(text: &str, bidi_level: u8) -> Glyph {
    Glyph {
        glyph_type: GlyphType::Composite { text: text.into() },
        face_id: FaceId::new(0),
        charpos: 0,
        bidi_level,
        wide: false,
        pixel_width: 0.0,
        pixel_height: 0.0,
        pixel_ascent: 0.0,
        vertical_offset_px: 0.0,
        padding: false,
        pointer_appearance: None,
    }
}

fn run_member_padding(ch: char, charpos: usize) -> Glyph {
    let mut g = Glyph::char(ch, FaceId::new(0), charpos);
    g.padding = true;
    g
}

#[test]
fn rtl_run_decomposes_into_reversed_per_letter_cells() {
    // Arabic "اب" (alef, beh) as the GUI emits it: one Composite holding the
    // whole run plus one per-letter grapheme padding, flagged right-to-left.
    let glyphs = vec![
        run_composite("\u{0627}\u{0628}", 1),
        run_member_padding('\u{0628}', 1),
    ];
    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state_with_text_glyphs(10, glyphs));
    // Visual order is reversed: beh then alef.
    assert_eq!(desired_char(&rif, 0, 0), '\u{0628}'); // beh
    assert_eq!(desired_char(&rif, 0, 1), '\u{0627}'); // alef
}

#[test]
fn ltr_run_decomposes_in_logical_order() {
    // Same structure but left-to-right (e.g. an Indic run): not reversed.
    let glyphs = vec![
        run_composite("\u{0627}\u{0628}", 0),
        run_member_padding('\u{0628}', 1),
    ];
    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state_with_text_glyphs(10, glyphs));
    assert_eq!(desired_char(&rif, 0, 0), '\u{0627}'); // alef
    assert_eq!(desired_char(&rif, 0, 1), '\u{0628}'); // beh
}

#[test]
fn rtl_run_keeps_combining_mark_on_its_letter() {
    // Run "سّل": seen+shadda forms one grapheme, then lam. The shadda rides on
    // seen's padding cell; reversed visual order is lam, then seen+shadda.
    let glyphs = vec![
        run_composite("\u{0633}\u{0651}\u{0644}", 1),
        run_member_padding_cluster("\u{0633}\u{0651}", 1),
        run_member_padding('\u{0644}', 2),
    ];
    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state_with_text_glyphs(10, glyphs));
    assert_eq!(desired_char(&rif, 0, 0), '\u{0644}'); // lam (leftmost)
    let seen_cell = &rif.desired.cells[1];
    assert_eq!(seen_cell.ch, '\u{0633}'); // seen base
    assert_eq!(seen_cell.extenders.as_deref(), Some("\u{0651}")); // shadda rides along
}

fn run_member_padding_cluster(text: &str, charpos: usize) -> Glyph {
    let mut g = Glyph {
        glyph_type: GlyphType::Composite { text: text.into() },
        face_id: FaceId::new(0),
        charpos,
        bidi_level: 1,
        wide: false,
        pixel_width: 0.0,
        pixel_height: 0.0,
        pixel_ascent: 0.0,
        vertical_offset_px: 0.0,
        padding: false,
        pointer_appearance: None,
    };
    g.padding = true;
    g
}

#[test]
fn plain_cluster_composite_stays_one_cell_and_skips_joiners() {
    // A ZWJ emoji family with no grapheme paddings stays a single cell; the
    // zero-width joiners are not drawn as their own characters.
    let glyphs = vec![run_composite("\u{1F468}\u{200D}\u{1F469}", 0)];
    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state_with_text_glyphs(10, glyphs));
    assert_eq!(rif.desired.cells[0].ch, '\u{1F468}');
    // The ZWJ is dropped; extenders hold only the second emoji.
    assert_eq!(rif.desired.cells[0].extenders.as_deref(), Some("\u{1F469}"));
}

// --- Color downsampling for non-truecolor terminals (issue #154) -----------

#[test]
fn tty_color_rgb_to_256_corners() {
    assert_eq!(rgb_to_256(255, 0, 0), 196);
    assert_eq!(rgb_to_256(0, 255, 0), 46);
    assert_eq!(rgb_to_256(0, 0, 255), 21);
    assert_eq!(rgb_to_256(0, 0, 0), 16);
    assert_eq!(rgb_to_256(255, 255, 255), 231);
    assert!((232..=255).contains(&rgb_to_256(128, 128, 128)));
}

#[test]
fn tty_color_rgb_to_ansi_basic() {
    assert_eq!(rgb_to_ansi_basic(0, 0, 0), (0, false));
    assert_eq!(rgb_to_ansi_basic(200, 0, 0), (1, true));
    assert_eq!(rgb_to_ansi_basic(0, 200, 0), (2, true));
    assert_eq!(rgb_to_ansi_basic(0, 0, 200), (4, true));
    assert_eq!(rgb_to_ansi_basic(255, 255, 255), (7, true));
}

// nextest isolates each test in its own process, so the process-global
// COLOR_TIER set here does not leak into the default-tier tests above.
#[test]
fn tty_write_sgr_downsamples_by_tier() {
    let attrs = CellAttrs {
        fg: Some((255, 0, 0)),
        ..CellAttrs::default()
    };
    // 256-color terminal -> indexed escape, never 24-bit truecolor.
    set_color_tier(256);
    let mut buf = Vec::new();
    write_sgr(&mut buf, &attrs);
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("\x1b[38;5;196m"), "256 tier red: {s:?}");
    assert!(!s.contains("38;2;"), "must not emit truecolor at 256 tier");

    // Basic (8/16) terminal -> bright-red ANSI code.
    set_color_tier(8);
    let mut buf = Vec::new();
    write_sgr(&mut buf, &attrs);
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("\x1b[91m"), "basic tier bright red: {s:?}");
}

// ---------------------------------------------------------------------------
// Terminal attribute capabilities (GNU term.c `turn_on_face` / `tty_capable_p`)
// ---------------------------------------------------------------------------

#[test]
fn italic_falls_back_to_dim_when_the_terminal_has_no_sitm() {
    // GNU term.c turn_on_face:
    //     if (tty->TS_enter_italic_mode) OUTPUT1 (tty, TS_enter_italic_mode);
    //     else  /* Italics not supported, use dim instead. */
    //           OUTPUT1 (tty, tty->TS_enter_dim_mode);
    // TERM=screen has no `sitm', so GNU renders `:slant italic' as SGR 2 there --
    // which is exactly what a GNU-vs-neomacs tty diff showed (GNU `^[[2m' vs
    // neomacs `^[[3m').
    let attrs = CellAttrs {
        italic: true,
        ..CellAttrs::default()
    };

    let with_italics = TtyAttributeCapabilities {
        italic: true,
        dim: true,
        ..TtyAttributeCapabilities::full()
    };
    let mut buf = Vec::new();
    write_sgr_with_capabilities(&mut buf, &attrs, &with_italics);
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("\x1b[3m"), "sitm present -> italic: {s:?}");
    assert!(
        !s.contains("\x1b[2m"),
        "must not dim when italic works: {s:?}"
    );

    let no_italics = TtyAttributeCapabilities {
        italic: false,
        dim: true,
        ..TtyAttributeCapabilities::full()
    };
    let mut buf = Vec::new();
    write_sgr_with_capabilities(&mut buf, &attrs, &no_italics);
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("\x1b[2m"), "no sitm -> dim fallback: {s:?}");
    assert!(!s.contains("\x1b[3m"), "no sitm -> no italic escape: {s:?}");

    // Neither capability: GNU emits nothing for the slant.
    let neither = TtyAttributeCapabilities {
        italic: false,
        dim: false,
        ..TtyAttributeCapabilities::full()
    };
    let mut buf = Vec::new();
    write_sgr_with_capabilities(&mut buf, &attrs, &neither);
    let s = String::from_utf8(buf).unwrap();
    assert!(!s.contains("\x1b[3m") && !s.contains("\x1b[2m"), "{s:?}");
}

#[test]
fn attributes_the_terminal_lacks_are_not_emitted() {
    // Every arm of GNU turn_on_face is gated on its capability string.
    let attrs = CellAttrs {
        bold: true,
        underline: UnderlineStyle::Line.gnu_code(),
        strikethrough: true,
        inverse: true,
        ..CellAttrs::default()
    };
    let none = TtyAttributeCapabilities::none();
    let mut buf = Vec::new();
    write_sgr_with_capabilities(&mut buf, &attrs, &none);
    let s = String::from_utf8(buf).unwrap();
    for escape in ["\x1b[1m", "\x1b[4m", "\x1b[9m", "\x1b[7m"] {
        assert!(
            !s.contains(escape),
            "{escape:?} emitted without support: {s:?}"
        );
    }

    let mut buf = Vec::new();
    write_sgr_with_capabilities(&mut buf, &attrs, &TtyAttributeCapabilities::full());
    let s = String::from_utf8(buf).unwrap();
    for escape in ["\x1b[1m", "\x1b[4m", "\x1b[9m", "\x1b[7m"] {
        assert!(
            s.contains(escape),
            "{escape:?} missing when supported: {s:?}"
        );
    }
}

#[test]
fn a_styled_underline_degrades_to_a_plain_one_without_smulx() {
    // GNU turn_on_face: the styled form is used only `if (tty->TF_set_underline_style)',
    // otherwise the plain `smul' sequence stands in.
    let attrs = CellAttrs {
        underline: UnderlineStyle::Wave.gnu_code(),
        ..CellAttrs::default()
    };
    let no_smulx = TtyAttributeCapabilities {
        underline_styled: false,
        ..TtyAttributeCapabilities::full()
    };
    let mut buf = Vec::new();
    write_sgr_with_capabilities(&mut buf, &attrs, &no_smulx);
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("\x1b[4m"), "plain underline fallback: {s:?}");
    assert!(
        !s.contains("4:3"),
        "no styled underline without Smulx: {s:?}"
    );
}

#[test]
fn color_capable_terminals_honor_the_no_color_video_mask() {
    // GNU MAY_USE_WITH_COLORS_P (term.c): when the terminal has colors, an
    // attribute listed in terminfo `ncv' cannot be combined with them, so
    // turn_on_face skips it entirely.
    let attrs = CellAttrs {
        bold: true,
        underline: UnderlineStyle::Line.gnu_code(),
        ..CellAttrs::default()
    };
    let ncv_bold = TtyAttributeCapabilities {
        color_cells: 256,
        no_color_video: TtyNoColorVideo::BOLD,
        ..TtyAttributeCapabilities::full()
    };
    let mut buf = Vec::new();
    write_sgr_with_capabilities(&mut buf, &attrs, &ncv_bold);
    let s = String::from_utf8(buf).unwrap();
    assert!(!s.contains("\x1b[1m"), "ncv bold must suppress bold: {s:?}");
    assert!(s.contains("\x1b[4m"), "underline is unaffected: {s:?}");

    // A monochrome terminal ignores ncv (GNU: `TN_max_colors > 0 ? … : 1').
    let mono = TtyAttributeCapabilities {
        color_cells: 0,
        no_color_video: TtyNoColorVideo::BOLD,
        ..TtyAttributeCapabilities::full()
    };
    let mut buf = Vec::new();
    write_sgr_with_capabilities(&mut buf, &attrs, &mono);
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("\x1b[1m"), "monochrome ignores ncv: {s:?}");
}

#[test]
fn tty_capable_p_matches_gnu_capability_and_ncv_logic() {
    // GNU tty_capable_p: every requested capability needs its terminfo string
    // AND (when the terminal has colors) an `ncv' bit that is clear.
    let full = TtyAttributeCapabilities::full();
    assert!(full.supports(TtyCapability::Bold));
    assert!(full.supports(TtyCapability::Italic));
    assert!(full.supports(TtyCapability::UnderlineStyled));

    let screen_like = TtyAttributeCapabilities {
        italic: false,
        ..TtyAttributeCapabilities::full()
    };
    assert!(!screen_like.supports(TtyCapability::Italic));
    assert!(screen_like.supports(TtyCapability::Underline));
    assert!(screen_like.supports(TtyCapability::Bold));

    let ncv_underline = TtyAttributeCapabilities {
        color_cells: 8,
        no_color_video: TtyNoColorVideo::UNDERLINE,
        ..TtyAttributeCapabilities::full()
    };
    assert!(!ncv_underline.supports(TtyCapability::Underline));
    assert!(ncv_underline.supports(TtyCapability::Bold));
}

// ---------------------------------------------------------------------------
// Scroll detection + synchronized output (issue #206)
// ---------------------------------------------------------------------------

/// Write `text` into desired row `row`, one char per cell.
fn set_row(rif: &mut TtyRif, row: usize, text: &str) {
    for (col, ch) in text.chars().enumerate() {
        rif.desired.set(row, col, ch, CellAttrs::default(), false);
    }
}

fn render_output(rif: &mut TtyRif) -> Vec<u8> {
    rif.diff_and_render();
    rif.take_output()
}

#[test]
fn scrolled_rows_emit_region_scroll_not_row_rewrites() {
    let mut rif = TtyRif::new(20, 10);
    for r in 0..10 {
        set_row(&mut rif, r, &format!("line-number-{r:02}"));
    }
    let _ = render_output(&mut rif); // establish the screen

    // Scroll down by one: rows shift up, one new line appears at the bottom.
    for r in 0..9 {
        set_row(&mut rif, r, &format!("line-number-{:02}", r + 1));
    }
    set_row(&mut rif, 9, "line-number-10");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);

    assert!(
        text.contains("\x1b[1S"),
        "one-line shift must emit a scroll-up, got: {text:?}"
    );
    assert!(
        text.contains(";10r") || text.contains("[1;10r"),
        "scroll must be bounded by a DECSTBM region, got: {text:?}"
    );
    // The shifted rows must NOT be retransmitted: their text moved, the
    // terminal moved it, so e.g. row content 'line-number-05' appears
    // nowhere in the output.
    assert!(
        !text.contains("line-number-05"),
        "shifted row content was retransmitted: {text:?}"
    );
    // The newly exposed bottom line IS transmitted.
    assert!(
        text.contains("line-number-10"),
        "exposed row must be drawn: {text:?}"
    );
}

#[test]
fn scroll_model_matches_terminal_after_region_scroll() {
    // After the scroll path runs, the internal `current` grid must agree
    // with what the terminal shows: a second render with unchanged desired
    // content emits no further row writes.
    let mut rif = TtyRif::new(20, 10);
    for r in 0..10 {
        set_row(&mut rif, r, &format!("stable-content-{r:02}"));
    }
    let _ = render_output(&mut rif);
    for r in 0..9 {
        set_row(&mut rif, r, &format!("stable-content-{:02}", r + 1));
    }
    set_row(&mut rif, 9, "stable-content-10");
    let _ = render_output(&mut rif);

    // Re-render the same content.
    for r in 0..9 {
        set_row(&mut rif, r, &format!("stable-content-{:02}", r + 1));
    }
    set_row(&mut rif, 9, "stable-content-10");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("stable-content"),
        "steady state after a scroll must be a no-op, got: {text:?}"
    );
}

#[test]
fn small_changes_do_not_trigger_scroll() {
    let mut rif = TtyRif::new(20, 10);
    for r in 0..10 {
        set_row(&mut rif, r, &format!("plain-old-row-{r:02}"));
    }
    let _ = render_output(&mut rif);
    for r in 0..10 {
        set_row(&mut rif, r, &format!("plain-old-row-{r:02}"));
    }
    set_row(&mut rif, 4, "edited-this-row!");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("\x1b[1S"),
        "an edit is not a scroll: {text:?}"
    );
    assert!(text.contains("edited-this-row!"));
}

#[test]
fn every_render_is_wrapped_in_synchronized_output() {
    let mut rif = TtyRif::new(8, 4);
    set_row(&mut rif, 0, "abc");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    let begin = text.find("\x1b[?2026h").expect("begin sync");
    let end = text.find("\x1b[?2026l").expect("end sync");
    assert!(begin < end, "sync begin must precede end");
    assert!(begin == 0, "sync begin must be the first bytes of a frame");
}

#[test]
fn distant_edits_on_one_row_skip_the_untouched_middle() {
    let mut rif = TtyRif::new(60, 4);
    set_row(
        &mut rif,
        1,
        "left-edit-here MIDDLE-STAYS-IDENTICAL right-edit-here",
    );
    let _ = render_output(&mut rif);
    set_row(
        &mut rif,
        1,
        "LEFT-EDIT-HERE MIDDLE-STAYS-IDENTICAL RIGHT-EDIT-HERE",
    );
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("MIDDLE-STAYS-IDENTICAL"),
        "unchanged middle must not be retransmitted: {text:?}"
    );
    assert!(text.contains("LEFT-EDIT-HERE") && text.contains("RIGHT-EDIT-HERE"));
    assert!(
        text.matches("\x1b[2;").count() >= 2,
        "two spans need two cursor motions on row 2: {text:?}"
    );
}

#[test]
fn nearby_edits_coalesce_into_one_span() {
    // Two edits six unchanged cells apart: retransmitting the gap is
    // cheaper than a second cursor motion, so one span covers both.
    let mut rif = TtyRif::new(40, 4);
    set_row(&mut rif, 1, "abcdefgh");
    let _ = render_output(&mut rif);
    set_row(&mut rif, 1, "AbcdefgH");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert_eq!(
        text.matches("\x1b[2;").count(),
        1,
        "a six-cell gap coalesces into one span: {text:?}"
    );
    assert!(text.contains("AbcdefgH"));
}

#[test]
fn scroll_plan_is_one_scroll_op_plus_exposed_row_runs() {
    // Structural assertion on the typed plan: a one-line shift is exactly
    // one ScrollRows op followed by write runs that touch ONLY the exposed
    // row - no retransmission ops for shifted content.
    let mut rif = TtyRif::new(20, 10);
    for r in 0..10 {
        set_row(&mut rif, r, &format!("plan-row-{r:02}"));
    }
    let _ = render_output(&mut rif);
    for r in 0..9 {
        set_row(&mut rif, r, &format!("plan-row-{:02}", r + 1));
    }
    set_row(&mut rif, 9, "plan-row-10");
    let ops = rif.plan_for_test();
    assert!(
        matches!(ops.first(), Some(TermOp::ScrollRows { .. })),
        "first op must be the scroll: {ops:?}"
    );
    for op in &ops[1..] {
        match op {
            TermOp::WriteRun { row, .. }
            | TermOp::ClearThenWriteRun { row, .. }
            | TermOp::EraseToEol { row, .. } => {
                assert_eq!(*row, 9, "only the exposed row may be rewritten: {ops:?}");
            }
            other => panic!("unexpected op after scroll: {other:?}"),
        }
    }
}

#[test]
fn caps_without_scroll_region_never_plan_scroll_ops() {
    let mut rif = TtyRif::new(20, 10);
    rif.set_caps(TermCaps {
        scroll_region: None,
        back_color_erase: false,
        insert_delete_char: false,
        erase_to_eol: false,
        synchronized_output: false,
    });
    for r in 0..10 {
        set_row(&mut rif, r, &format!("dumb-term-{r:02}"));
    }
    let _ = render_output(&mut rif);
    for r in 0..9 {
        set_row(&mut rif, r, &format!("dumb-term-{:02}", r + 1));
    }
    set_row(&mut rif, 9, "dumb-term-10");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(!text.contains("\x1b[?2026"), "sync gated off: {text:?}");
    assert!(
        !text.contains("S\x1b[r"),
        "no region scroll on a dumb terminal"
    );
    // Content still arrives, just via ordinary row diffs (which transmit
    // only the cells that changed, so assert on a changed fragment).
    assert!(
        text.contains("10"),
        "changed content must be drawn: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// Erase-to-EOL (issue #206 phase 4)
// ---------------------------------------------------------------------------

#[test]
fn line_kill_erases_to_eol_instead_of_writing_spaces() {
    let mut rif = TtyRif::new(40, 4);
    set_row(&mut rif, 1, "a-full-line-of-content-to-be-killed!");
    let _ = render_output(&mut rif);
    // Kill the line: desired becomes all blanks (grid default cells).
    for col in 0..40 {
        rif.desired.set(1, col, ' ', CellAttrs::default(), false);
    }
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("\x1b[K"), "line kill must use EL: {text:?}");
    assert!(
        !text.contains("        "),
        "no long space runs when EL is available: {text:?}"
    );
    assert_eq!(rif.frame_stats().erase_ops, 1);

    // Steady state: the model must agree with the erased terminal.
    for col in 0..40 {
        rif.desired.set(1, col, ' ', CellAttrs::default(), false);
    }
    let out = render_output(&mut rif);
    assert_eq!(rif.frame_stats().write_runs, 0, "no-op after erase");
    let _ = out;
}

#[test]
fn inverse_blank_tail_refuses_erase() {
    let mut rif = TtyRif::new(20, 4);
    set_row(&mut rif, 1, "abcdefgh");
    let _ = render_output(&mut rif);
    let inverse_blank = CellAttrs {
        inverse: true,
        ..CellAttrs::default()
    };
    // A standout bar: inverse blanks render as a solid block; ESC[K would
    // erase it to plain background.
    for col in 0..20 {
        rif.desired.set(1, col, ' ', inverse_blank, false);
    }
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("\x1b[K"),
        "inverse blanks must be written, not erased: {text:?}"
    );
    assert_eq!(rif.frame_stats().erase_ops, 0);
}

#[test]
fn colored_tail_without_bce_stays_on_the_write_path() {
    let mut rif = TtyRif::new(20, 4);
    rif.set_caps(TermCaps {
        back_color_erase: false,
        ..TermCaps::default()
    });
    set_row(&mut rif, 1, "abcdefghijkl");
    let _ = render_output(&mut rif);
    let colored_blank = CellAttrs {
        bg: Some((40, 44, 52)),
        ..CellAttrs::default()
    };
    for col in 0..20 {
        rif.desired.set(1, col, ' ', colored_blank, false);
    }
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("\x1b[K"),
        "no BCE means a colored tail cannot be erased: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// Efficiency regression guards (issue #206 evidence)
// ---------------------------------------------------------------------------

#[test]
fn one_line_scroll_costs_a_small_fraction_of_a_full_repaint() {
    // The quantified issue-206 claim, pinned as a regression guard: a
    // one-line scroll of a full 24x80 frame must emit far fewer bytes than
    // repainting it.
    let full_repaint_bytes = {
        let mut rif = TtyRif::new(80, 24);
        for r in 0..24 {
            set_row(
                &mut rif,
                r,
                &format!("{:-<79}{}", format!("content line {r} "), "|"),
            );
        }
        let out = render_output(&mut rif);
        out.len()
    };

    let mut rif = TtyRif::new(80, 24);
    for r in 0..24 {
        set_row(
            &mut rif,
            r,
            &format!("{:-<79}{}", format!("content line {r} "), "|"),
        );
    }
    let _ = render_output(&mut rif);
    for r in 0..23 {
        set_row(
            &mut rif,
            r,
            &format!("{:-<79}{}", format!("content line {} ", r + 1), "|"),
        );
    }
    set_row(&mut rif, 23, &format!("{:-<79}{}", "content line 24 ", "|"));
    let scroll_bytes = render_output(&mut rif).len();

    assert_eq!(rif.frame_stats().scroll_ops, 1);
    assert!(
        scroll_bytes * 5 < full_repaint_bytes,
        "a one-line scroll ({scroll_bytes} bytes) must cost <20% of a full repaint ({full_repaint_bytes} bytes)"
    );
}

#[test]
fn lying_scroll_seed_falls_back_to_correct_inference() {
    // The semantic seed is only a hint: a wrong delta must fail cell
    // verification and the voting path must still find the true scroll.
    let mut rif = TtyRif::new(20, 12);
    for r in 0..12 {
        set_row(&mut rif, r, &format!("seeded-line-{r:02}"));
    }
    let _ = render_output(&mut rif);
    for r in 0..11 {
        set_row(&mut rif, r, &format!("seeded-line-{:02}", r + 1));
    }
    set_row(&mut rif, 11, "seeded-line-12");
    rif.set_scroll_seed_for_test(Some(-3)); // a lie: the real shift is +1
    let ops = rif.plan_for_test();
    match ops.first() {
        Some(TermOp::ScrollRows { dir, .. }) => {
            assert!(
                matches!(dir, ScrollDir::Up(n) if n.get() == 1),
                "voting must recover the true delta despite the lying seed: {ops:?}"
            );
        }
        other => panic!("scroll expected despite lying seed: {other:?}"),
    }
    let stats = rif.frame_stats();
    assert_eq!(stats.scroll_seed_rejected, 1, "the lie must be counted");
    assert_eq!(stats.scroll_seed_accepted, 0);
}

#[test]
fn truthful_scroll_seed_is_accepted_and_counted() {
    let mut rif = TtyRif::new(20, 12);
    for r in 0..12 {
        set_row(&mut rif, r, &format!("seeded-line-{r:02}"));
    }
    let _ = render_output(&mut rif);
    for r in 0..11 {
        set_row(&mut rif, r, &format!("seeded-line-{:02}", r + 1));
    }
    set_row(&mut rif, 11, "seeded-line-12");
    rif.set_scroll_seed_for_test(Some(1));
    let ops = rif.plan_for_test();
    assert!(
        matches!(
            ops.first(),
            Some(TermOp::ScrollRows { dir: ScrollDir::Up(n), .. }) if n.get() == 1
        ),
        "seeded scroll expected: {ops:?}"
    );
    let stats = rif.frame_stats();
    assert_eq!(stats.scroll_seed_accepted, 1);
    assert_eq!(stats.scroll_seed_rejected, 0);
}

// ---------------------------------------------------------------------------
// In-line horizontal shifts (issue #206 phase 4b)
// ---------------------------------------------------------------------------

#[test]
fn typing_one_char_mid_line_uses_insert_cells() {
    let mut rif = TtyRif::new(60, 4);
    set_row(&mut rif, 1, "fn main() { println!(\"hello, world\"); }");
    let _ = render_output(&mut rif);
    set_row(&mut rif, 1, "fn mXain() { println!(\"hello, world\"); }");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains("\x1b[1@"),
        "one typed char is one ICH: {text:?}"
    );
    assert!(
        !text.contains("println"),
        "the shifted tail must not be retransmitted: {text:?}"
    );

    // Steady state: the model matches the terminal after the shift.
    set_row(&mut rif, 1, "fn mXain() { println!(\"hello, world\"); }");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains('X') && !text.contains("main"),
        "no-op after shift: {text:?}"
    );
}

#[test]
fn deleting_one_char_mid_line_uses_delete_cells() {
    let mut rif = TtyRif::new(60, 4);
    set_row(&mut rif, 1, "fn mXain() { println!(\"hello, world\"); }");
    let _ = render_output(&mut rif);
    set_row(&mut rif, 1, "fn main() { println!(\"hello, world\"); } ");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains("\x1b[1P"),
        "one deleted char is one DCH: {text:?}"
    );
    assert!(
        !text.contains("println"),
        "tail not retransmitted: {text:?}"
    );
}

#[test]
fn rows_with_wide_chars_refuse_horizontal_shifts() {
    let mut rif = TtyRif::new(30, 4);
    // A wide char occupies a base cell plus a padding cell.
    set_row(&mut rif, 1, "abc ");
    rif.desired.set(1, 4, '日', CellAttrs::default(), false);
    rif.desired.set(1, 5, ' ', CellAttrs::default(), true);
    set_row_from(&mut rif, 1, 6, "-tail-content-here");
    let _ = render_output(&mut rif);
    rif.desired.set(1, 0, 'a', CellAttrs::default(), false);
    rif.desired.set(1, 1, 'X', CellAttrs::default(), false);
    set_row_from(&mut rif, 1, 2, "bc ");
    rif.desired.set(1, 5, '日', CellAttrs::default(), false);
    rif.desired.set(1, 6, ' ', CellAttrs::default(), true);
    set_row_from(&mut rif, 1, 7, "-tail-content-her");
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains('@') || !text.contains("\x1b[1@"),
        "wide-char rows must not shift: {text:?}"
    );
}

/// Write `text` into desired row `row` starting at `col`.
fn set_row_from(rif: &mut TtyRif, row: usize, col: usize, text: &str) {
    for (i, ch) in text.chars().enumerate() {
        rif.desired
            .set(row, col + i, ch, CellAttrs::default(), false);
    }
}

#[test]
fn typing_echo_bytes_are_a_fraction_of_tail_rewrite() {
    // Efficiency guard for the typing-echo case.
    let line = "let value = compute_the_answer(question, context, options);";
    let mut rif = TtyRif::new(80, 4);
    set_row(&mut rif, 1, line);
    let _ = render_output(&mut rif);
    let edited = format!("{}X{}", &line[..4], &line[4..]);
    set_row(&mut rif, 1, &edited);
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    // Sync wrapper + cursor hide + goto + ICH + goto + SGR + the one glyph.
    assert!(
        out.len() < 64,
        "typing echo must be tens of bytes, not a tail rewrite: {} {text:?}",
        out.len()
    );
    assert!(
        !text.contains("compute_the_answer"),
        "tail must ride the shift, not a rewrite: {text:?}"
    );
}

#[test]
fn index_method_scrolls_with_ind_ri_and_never_su_sd() {
    // vt220 / Linux-console shape: DECSTBM attested, CSI S/T not. The
    // capability carries the method, so the same plan encodes as cursor to
    // the region edge plus IND/RI.
    let caps = TermCaps {
        scroll_region: Some(RegionScrollMethod::Index),
        ..TermCaps::default()
    };
    let mut rif = TtyRif::new_with_caps(20, 10, caps);
    // Production rasterization clears the desired grid every frame; this
    // incremental harness must overwrite full rows for the same effect.
    let full_row = |n: usize| format!("{:<20}", format!("line-number-{n:02}"));
    for r in 0..10 {
        set_row(&mut rif, r, &full_row(r));
    }
    let _ = render_output(&mut rif);

    // Scroll up by one.
    for r in 0..10 {
        set_row(&mut rif, r, &full_row(r + 1));
    }
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("\x1b[1S") && !text.contains("\x1b[1T"),
        "index method must never emit SU/SD: {text:?}"
    );
    assert!(
        text.contains("\x1b[1;10r") && text.contains("\x1bD"),
        "scroll must be DECSTBM + IND at the region bottom: {text:?}"
    );
    assert!(
        !text.contains("line-number-05"),
        "shifted row content was retransmitted: {text:?}"
    );

    // And the reverse direction uses RI at the region top.
    for r in 0..10 {
        set_row(&mut rif, r, &full_row(r));
    }
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains("\x1bM") && !text.contains("\x1b[1T"),
        "reverse scroll must be RI, not SD: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// Wide-char neutralization (GNU dispnew.c neutralize_wide_char parity)
// ---------------------------------------------------------------------------

#[test]
fn overwriting_the_padding_half_of_a_wide_pair_blanks_the_base() {
    // A child-frame border landing on the right half of a CJK char: the
    // terminal blanks the orphaned base, so the model must too, or the
    // divergence is invisible to every later diff.
    let mut grid = TtyGrid::new(10, 2);
    grid.set(1, 4, '\u{65e5}', CellAttrs::default(), false);
    grid.set(1, 5, ' ', CellAttrs::default(), true);

    grid.set(1, 5, '|', CellAttrs::default(), false);

    let base = &grid.cells[10 + 4];
    assert_eq!(base.ch, ' ', "orphaned wide base must become a space");
    assert!(!base.padding);
    assert_eq!(grid.cells[10 + 5].ch, '|');
}

#[test]
fn overwriting_the_base_half_of_a_wide_pair_blanks_the_padding() {
    let mut grid = TtyGrid::new(10, 2);
    grid.set(0, 2, '\u{65e5}', CellAttrs::default(), false);
    grid.set(0, 3, ' ', CellAttrs::default(), true);

    grid.set(0, 2, 'X', CellAttrs::default(), false);

    let orphan = &grid.cells[3];
    assert!(
        !orphan.padding,
        "orphaned padding must become a plain space"
    );
    assert_eq!(orphan.ch, ' ');
    assert_eq!(grid.cells[2].ch, 'X');
}

#[test]
fn rewriting_a_wide_pair_in_place_keeps_it_intact() {
    // The rasterizer writes base then padding every frame; neutralization
    // must not eat the pair it is in the middle of rewriting.
    let mut grid = TtyGrid::new(10, 1);
    grid.set(0, 2, '\u{65e5}', CellAttrs::default(), false);
    grid.set(0, 3, ' ', CellAttrs::default(), true);

    grid.set(0, 2, '\u{672c}', CellAttrs::default(), false);
    grid.set(0, 3, ' ', CellAttrs::default(), true);

    assert_eq!(grid.cells[2].ch, '\u{672c}');
    assert!(
        grid.cells[3].padding,
        "rewritten pair keeps its padding half"
    );
}

#[test]
fn wide_base_in_the_final_column_rasterizes_as_a_space() {
    // No room for the padding half: the terminal would blank the base while
    // the model kept it. GNU never emits a partially visible multi-column
    // glyph.
    let mut state = FrameDisplayState::new(10, 5, 8.0, 16.0);
    state.background = Color::BLACK;

    let mut matrix = GlyphMatrix::new(5, 10);
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in "abcdefghi".chars().enumerate() {
        row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), i));
    }
    let mut wide = Glyph::char('\u{4e16}', FaceId::new(0), 9);
    wide.wide = true;
    row.glyphs[GlyphArea::Text as usize].push(wide);
    matrix.rows[0] = std::sync::Arc::new(row);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: DisplayWindowId::new(1),
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 80.0),
        text_clip_bounds: None,
        selected: true,
    });

    let mut rif = TtyRif::new(10, 5);
    rif.rasterize(&state);

    assert_eq!(rif.desired.cells[9].ch, ' ', "clipped wide base is a space");
    assert!(!rif.desired.cells[9].padding);
    assert!(
        rif.desired.cells[..10].iter().all(|cell| !cell.padding),
        "no padding cell may survive at the row edge"
    );
}

// ---------------------------------------------------------------------------
// Planner hardening + coverage gaps from the adversarial review
// ---------------------------------------------------------------------------

#[test]
fn zero_area_grids_plan_nothing_and_do_not_panic() {
    for (w, h) in [(0, 10), (10, 0), (0, 0)] {
        let mut rif = TtyRif::new(w, h);
        let ops = rif.plan_for_test();
        assert!(ops.is_empty(), "{w}x{h} must plan the empty frame");
        let _ = render_output(&mut rif);
    }
}

#[test]
fn scrolling_backward_emits_region_scroll_down() {
    // Down-direction sibling of scrolled_rows_emit_region_scroll: content
    // moves down (viewport scrolled up), encoded as SD inside the region.
    let mut rif = TtyRif::new(20, 10);
    let full_row = |n: i32| format!("{:<20}", format!("line-number-{n:02}"));
    for r in 0..10i32 {
        set_row(&mut rif, r as usize, &full_row(r + 1));
    }
    let _ = render_output(&mut rif);
    for r in 0..10i32 {
        set_row(&mut rif, r as usize, &full_row(r));
    }
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains("\x1b[1T"),
        "one-line backward shift must emit SD: {text:?}"
    );
    assert!(
        !text.contains("line-number-05"),
        "shifted rows must not be retransmitted: {text:?}"
    );
    assert!(
        text.contains("line-number-00"),
        "the exposed top row must be drawn: {text:?}"
    );

    // Model fidelity: an identical next frame is a no-op.
    for r in 0..10i32 {
        set_row(&mut rif, r as usize, &full_row(r));
    }
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    assert!(
        !text.contains("line-number"),
        "model must match the terminal after SD: {text:?}"
    );
}

#[test]
fn colored_tail_with_bce_erases_and_establishes_the_fill_color() {
    // The accepting BCE path: erase must set the tail's background BEFORE
    // ESC[K with no reset between, or the fill color is wrong.
    let mut rif = TtyRif::new(20, 4);
    set_row(&mut rif, 1, "abcdefghijklmnopqrst");
    let _ = render_output(&mut rif);
    let colored_blank = CellAttrs {
        bg: Some((40, 44, 52)),
        ..CellAttrs::default()
    };
    for col in 0..20 {
        rif.desired.set(1, col, ' ', colored_blank, false);
    }
    let out = render_output(&mut rif);
    let text = String::from_utf8_lossy(&out);
    let erase_at = text.find("\x1b[K").expect("BCE tail must erase");
    let bg_at = text
        .find("48;2;40;44;52m")
        .expect("fill background must be established");
    assert!(bg_at < erase_at, "bg must precede the erase: {text:?}");
    let between = &text[bg_at..erase_at];
    assert!(
        !between.contains("\x1b[0m"),
        "no SGR reset may sit between bg and erase: {text:?}"
    );
}

#[test]
fn layout_reused_shifted_damage_seeds_the_region_scroll() {
    // Producer-path acceptance: the semantic seed travels from
    // RowDamage::ReusedShifted on the glyph rows through rasterize (1x1
    // TTY metrics make dvpos the line delta) into an accepted, verified
    // ScrollRows plan — not via the test-only seed setter.
    let cols = 20usize;
    let rows = 12usize;
    let build_state = |first_line: usize, shifted: bool| {
        let mut state = FrameDisplayState::new(cols, rows, 1.0, 1.0);
        state.background = Color::BLACK;
        let mut matrix = GlyphMatrix::new(rows, cols);
        for r in 0..rows {
            let mut row = GlyphRow::new(GlyphRowRole::Text);
            for (i, ch) in format!("{:<20}", format!("seed-line-{:02}", first_line + r))
                .chars()
                .enumerate()
            {
                row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, FaceId::new(0), i));
            }
            matrix.rows[r] = std::sync::Arc::new(row);
            if shifted {
                matrix.set_row_damage(r, RowDamage::ReusedShifted { dvpos: Px(-1.0) });
            }
        }
        state.window_matrices.push(WindowMatrixEntry {
            window_id: DisplayWindowId::new(1),
            matrix,
            pixel_bounds: Rect::new(0.0, 0.0, cols as f32, rows as f32),
            text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32, rows as f32),
            text_clip_bounds: None,
            selected: true,
        });
        state
    };

    let mut rif = TtyRif::new(cols, rows);
    rif.rasterize(&build_state(0, false));
    let _ = render_output(&mut rif);

    // One line scrolled: content moved up (dvpos = -1px at 1px line height).
    rif.rasterize(&build_state(1, true));
    let ops = rif.plan_for_test();
    assert!(
        matches!(
            ops.first(),
            Some(TermOp::ScrollRows { dir: ScrollDir::Up(n), .. }) if n.get() == 1
        ),
        "shifted-damage seed must plan a verified region scroll: {ops:?}"
    );
    assert_eq!(
        rif.frame_stats().scroll_seed_accepted,
        1,
        "the layout hint, not voting, must carry this scroll"
    );
}
