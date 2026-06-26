use super::*;
use crate::face::Face;
use crate::frame_glyphs::{DisplaySlotId, PhysCursor};

#[test]
fn glyph_type_kind_codes_match_gnu_glyph_type() {
    let cases = [
        (GlyphTypeKind::Char, 0),
        (GlyphTypeKind::Composite, 1),
        (GlyphTypeKind::Glyphless, 2),
        (GlyphTypeKind::Image, 3),
        (GlyphTypeKind::Stretch, 4),
        (GlyphTypeKind::Xwidget, 5),
    ];

    for (kind, code) in cases {
        assert_eq!(kind.gnu_code(), code);
        assert_eq!(GlyphTypeKind::from_gnu_code(code), Some(kind));
    }

    assert_eq!(GlyphTypeKind::from_gnu_code(6), None);
    assert_eq!(
        Glyph::char('x', 0, 0).glyph_type.gnu_kind(),
        GlyphTypeKind::Char
    );
    assert_eq!(
        GlyphType::Composite { text: "xy".into() }.gnu_kind(),
        GlyphTypeKind::Composite
    );
    assert_eq!(
        GlyphType::Glyphless { ch: '\u{fffd}' }.gnu_kind(),
        GlyphTypeKind::Glyphless
    );
    assert_eq!(
        GlyphType::Image { image_id: 7 }.gnu_kind(),
        GlyphTypeKind::Image
    );
    assert_eq!(
        Glyph::stretch(2, 0).glyph_type.gnu_kind(),
        GlyphTypeKind::Stretch
    );
}

#[test]
fn glyph_area_codes_match_gnu_glyph_row_area() {
    let cases = [
        (GlyphArea::LeftMargin, 0),
        (GlyphArea::Text, 1),
        (GlyphArea::RightMargin, 2),
    ];

    for (area, code) in cases {
        assert_eq!(area.gnu_code(), code);
        assert_eq!(area.index(), usize::from(code));
        assert_eq!(GlyphArea::from_gnu_code(code), Some(area));
    }

    assert_eq!(GlyphArea::from_gnu_code(3), None);
}

#[test]
fn empty_row_has_zero_hash() {
    let row = GlyphRow::new(GlyphRowRole::Text);
    assert_eq!(row.compute_hash(), 0);
}

#[test]
fn row_hash_changes_with_content() {
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    let hash_empty = row.compute_hash();
    row.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 0, 0));
    let hash_a = row.compute_hash();
    assert_ne!(hash_empty, hash_a);
}

#[test]
fn row_hash_differs_for_different_chars() {
    let mut row_a = GlyphRow::new(GlyphRowRole::Text);
    row_a.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 0, 0));

    let mut row_b = GlyphRow::new(GlyphRowRole::Text);
    row_b.glyphs[GlyphArea::Text as usize].push(Glyph::char('b', 0, 0));

    assert_ne!(row_a.compute_hash(), row_b.compute_hash());
}

#[test]
fn row_hash_differs_for_different_faces() {
    let mut row_a = GlyphRow::new(GlyphRowRole::Text);
    row_a.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 0, 0));

    let mut row_b = GlyphRow::new(GlyphRowRole::Text);
    row_b.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 1, 0));

    assert_ne!(row_a.compute_hash(), row_b.compute_hash());
}

#[test]
fn row_hash_differs_for_different_pixel_widths() {
    let mut row_a = GlyphRow::new(GlyphRowRole::Text);
    row_a.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 0, 0).with_pixel_width(8.0));

    let mut row_b = GlyphRow::new(GlyphRowRole::Text);
    row_b.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 0, 0).with_pixel_width(13.0));

    assert_ne!(row_a.compute_hash(), row_b.compute_hash());
}

#[test]
fn row_hash_differs_for_different_vertical_offsets() {
    let mut row_a = GlyphRow::new(GlyphRowRole::Text);
    row_a.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 0, 0).with_vertical_offset(-4.0));

    let mut row_b = GlyphRow::new(GlyphRowRole::Text);
    row_b.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 0, 0));

    assert_ne!(row_a.compute_hash(), row_b.compute_hash());
}

#[test]
fn identical_rows_have_same_hash() {
    let mut row_a = GlyphRow::new(GlyphRowRole::Text);
    row_a.glyphs[GlyphArea::Text as usize].push(Glyph::char('x', 5, 100));

    let mut row_b = GlyphRow::new(GlyphRowRole::Text);
    row_b.glyphs[GlyphArea::Text as usize].push(Glyph::char('x', 5, 100));

    assert_eq!(row_a.compute_hash(), row_b.compute_hash());
}

#[test]
fn row_equal_uses_hash_fast_path() {
    let mut row_a = GlyphRow::new(GlyphRowRole::Text);
    row_a.glyphs[GlyphArea::Text as usize].push(Glyph::char('a', 0, 0));
    row_a.hash = row_a.compute_hash();

    let mut row_b = GlyphRow::new(GlyphRowRole::Text);
    row_b.glyphs[GlyphArea::Text as usize].push(Glyph::char('b', 0, 0));
    row_b.hash = row_b.compute_hash();

    // Different hashes → rows are not equal (fast path, no cell comparison)
    assert!(!row_a.row_equal(&row_b));

    // Same content → equal
    let row_c = row_a.clone();
    assert!(row_a.row_equal(&row_c));
}

#[test]
fn new_row_has_empty_glyph_areas() {
    let row = GlyphRow::new(GlyphRowRole::ModeLine);
    assert!(row.glyphs[GlyphArea::LeftMargin as usize].is_empty());
    assert!(row.glyphs[GlyphArea::Text as usize].is_empty());
    assert!(row.glyphs[GlyphArea::RightMargin as usize].is_empty());
    assert_eq!(row.role, GlyphRowRole::ModeLine);
    assert!(row.enabled);
}

#[test]
fn matrix_new_has_correct_dimensions() {
    let matrix = GlyphMatrix::new(24, 80);
    assert_eq!(matrix.nrows, 24);
    assert_eq!(matrix.ncols, 80);
    assert_eq!(matrix.rows.len(), 24);
}

#[test]
fn matrix_rows_are_disabled_by_default() {
    // Rows in a freshly constructed GlyphMatrix start disabled,
    // matching GNU's MATRIX_ROW_ENABLED_P discipline: the walker
    // marks rows enabled as it populates them, and rows never
    // populated stay inert so scroll / clear-to-eob helpers
    // (e.g. overwrite_last_window_right_border) skip them.
    let matrix = GlyphMatrix::new(3, 10);
    for row in &matrix.rows {
        assert!(!row.enabled);
        assert_eq!(row.role, GlyphRowRole::Text);
    }
}

#[test]
fn matrix_clear_resets_all_rows() {
    let mut matrix = GlyphMatrix::new(2, 10);
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('x', 0, 0));
    matrix.rows[0].hash = 12345;
    matrix.rows[0].cursor_col = Some(5);

    matrix.clear();

    assert!(matrix.rows[0].glyphs[GlyphArea::Text as usize].is_empty());
    assert_eq!(matrix.rows[0].hash, 0);
    assert_eq!(matrix.rows[0].cursor_col, None);
}

#[test]
fn matrix_resize_grows_and_shrinks() {
    let mut matrix = GlyphMatrix::new(10, 80);
    assert_eq!(matrix.rows.len(), 10);

    matrix.resize(20, 100);
    assert_eq!(matrix.nrows, 20);
    assert_eq!(matrix.ncols, 100);
    assert_eq!(matrix.rows.len(), 20);

    matrix.resize(5, 40);
    assert_eq!(matrix.nrows, 5);
    assert_eq!(matrix.ncols, 40);
    assert_eq!(matrix.rows.len(), 5);
}

#[test]
fn frame_display_state_new_has_correct_defaults() {
    let state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    assert_eq!(state.frame_cols, 80);
    assert_eq!(state.frame_rows, 24);
    assert_eq!(state.char_width, 8.0);
    assert_eq!(state.char_height, 16.0);
    assert!(state.window_matrices.is_empty());
    assert!(state.faces.is_empty());
}

#[test]
fn frame_display_state_add_window_matrix() {
    let mut state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    let matrix = GlyphMatrix::new(20, 80);
    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 640.0, 320.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 640.0, 320.0),
        selected: true,
    });
    assert_eq!(state.window_matrices.len(), 1);
    assert_eq!(state.window_matrices[0].window_id, 1);
    assert_eq!(state.window_matrices[0].matrix.nrows, 20);
}

// ---------------------------------------------------------------------------
// FrameDisplayState::materialize() tests
// ---------------------------------------------------------------------------

/// Helper: build a FrameDisplayState with one window containing `text` on row 0.
fn state_with_text(text: &str) -> FrameDisplayState {
    let cols = text.len().max(1);
    let rows = 1;
    let char_w = 8.0f32;
    let char_h = 16.0f32;
    let mut state = FrameDisplayState::new(cols, rows, char_w, char_h);

    // Insert a default face (id 0)
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(1, cols);
    matrix.rows[0].enabled = true;
    for (i, ch) in text.chars().enumerate() {
        matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, 0, i));
    }

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, char_h),
        text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, char_h),
        selected: true,
    });
    state
}

#[test]
fn materialize_produces_correct_glyph_count_from_grid() {
    let state = state_with_text("Hello");
    let buf = state.materialize();
    // 5 characters -> 5 FrameGlyph::Char entries
    assert_eq!(buf.glyphs.len(), 5);
    for g in &buf.glyphs {
        assert!(matches!(g, FrameGlyph::Char { .. }));
    }
}

#[test]
fn materialize_emits_tab_line_row_at_window_top() {
    // Regression guard for the GUI "empty tab-line" bug: a window with a
    // tab-line (row 0, role TabLine) above its text (row 1, role Text) must
    // materialize the tab-line glyphs at the window's TOP edge, tagged with
    // GlyphRowRole::TabLine so the renderer treats them as top chrome.
    let char_w = 8.0f32;
    let char_h = 16.0f32;
    let cols = 4;
    let win = Rect::new(10.0, 20.0, cols as f32 * char_w, 2.0 * char_h);
    let text_area = Rect::new(10.0, 20.0 + char_h, cols as f32 * char_w, char_h);

    let mut state = FrameDisplayState::new(cols, 2, char_w, char_h);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(2, cols);
    matrix.rows[0].role = GlyphRowRole::TabLine;
    matrix.rows[0].enabled = true;
    matrix.rows[0].height_px = char_h;
    matrix.rows[0].pixel_y = 0.0;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('T', 0, 0));
    matrix.rows[1].role = GlyphRowRole::Text;
    matrix.rows[1].enabled = true;
    matrix.rows[1].height_px = char_h;
    matrix.rows[1].pixel_y = 0.0;
    matrix.rows[1].glyphs[GlyphArea::Text as usize].push(Glyph::char('B', 0, 0));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: win,
        text_pixel_bounds: text_area,
        selected: true,
    });

    let buf = state.materialize();

    let tab_glyphs: Vec<_> = buf
        .glyphs
        .iter()
        .filter_map(|g| match g {
            FrameGlyph::Char {
                row_role,
                char: c,
                y,
                ..
            } if *row_role == GlyphRowRole::TabLine => Some((*c, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(tab_glyphs.len(), 1, "tab-line glyph must be materialized");
    assert_eq!(tab_glyphs[0].0, 'T');
    assert!(
        (tab_glyphs[0].1 - win.y).abs() < 0.5,
        "tab-line glyph y={} must sit at window top {}",
        tab_glyphs[0].1,
        win.y
    );

    let body: Vec<_> = buf
        .glyphs
        .iter()
        .filter_map(|g| match g {
            FrameGlyph::Char {
                row_role,
                char: c,
                y,
                ..
            } if *row_role == GlyphRowRole::Text => Some((*c, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0].0, 'B');
    assert!(
        (body[0].1 - text_area.y).abs() < 0.5,
        "body glyph y={} must sit in the text area at {}",
        body[0].1,
        text_area.y
    );
}

#[test]
fn materialize_right_aligns_reversed_row() {
    // A reversed_p (right-to-left) row is flush to the right margin: its glyphs
    // start where the content ends at the right edge, not at column 0.
    let char_w = 8.0f32;
    let char_h = 16.0f32;
    let cols = 10; // 80px-wide text area
    let mut state = FrameDisplayState::new(cols, 1, char_w, char_h);
    state.faces.insert(0, Face::new(0));
    let mut matrix = GlyphMatrix::new(1, cols);
    matrix.rows[0].enabled = true;
    matrix.rows[0].reversed_p = true;
    // Two cells, no recorded pixel width -> one column (8px) each => 16px used.
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('\u{05d0}', 0, 0));
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('\u{05d1}', 0, 1));
    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, char_h),
        text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, char_h),
        selected: true,
    });
    let buf = state.materialize();
    let first_x = buf
        .glyphs
        .iter()
        .find_map(|g| match g {
            FrameGlyph::Char { x, .. } => Some(*x),
            _ => None,
        })
        .expect("a char glyph");
    // 80px area minus 16px content => content flush-right starting at x=64.
    assert!(
        (first_x - 64.0).abs() < 0.01,
        "expected flush-right x=64, got {first_x}"
    );
}

#[test]
fn materialize_empty_grid_produces_no_glyphs() {
    let state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    let buf = state.materialize();
    assert!(buf.glyphs.is_empty());
}

#[test]
fn materialize_includes_backgrounds() {
    let mut state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    state.backgrounds.push(BackgroundItem {
        bounds: Rect::new(0.0, 0.0, 640.0, 384.0),
        color: Color::RED,
    });
    let buf = state.materialize();
    assert_eq!(buf.glyphs.len(), 1);
    match &buf.glyphs[0] {
        FrameGlyph::Background { bounds, color } => {
            assert_eq!(bounds.x, 0.0);
            assert_eq!(bounds.width, 640.0);
            assert_eq!(*color, Color::RED);
        }
        other => panic!("expected Background, got {:?}", other),
    }
}

#[test]
fn materialize_includes_borders() {
    let mut state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    state.borders.push(BorderItem {
        window_id: DisplayWindowId::new(42),
        x: 100.0,
        y: 0.0,
        width: 1.0,
        height: 384.0,
        color: Color::WHITE,
    });
    let buf = state.materialize();
    assert_eq!(buf.glyphs.len(), 1);
    match &buf.glyphs[0] {
        FrameGlyph::Border {
            window_id,
            x,
            width,
            color,
            ..
        } => {
            assert_eq!(window_id.get(), 42);
            assert_eq!(*x, 100.0);
            assert_eq!(*width, 1.0);
            assert_eq!(*color, Color::WHITE);
        }
        other => panic!("expected Border, got {:?}", other),
    }
}

#[test]
fn materialize_includes_cursors() {
    let mut state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    state.cursors.push(CursorItem {
        window_id: DisplayWindowId::new(7),
        slot_id: DisplaySlotId::from_pixels(DisplayWindowId::new(7), 40.0, 0.0, 8.0, 16.0),
        x: 40.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        style: CursorStyle::FilledBox,
        color: Color::GREEN,
    });
    let buf = state.materialize();
    assert!(buf.glyphs.is_empty());
    assert_eq!(buf.window_cursors.len(), 1);
    assert!(buf.active_cursor().is_none());
    let cursor = &buf.window_cursors[0];
    assert_eq!(cursor.window_id.get(), 7);
    assert_eq!(cursor.x, 40.0);
    assert_eq!(cursor.style, CursorStyle::FilledBox);
    assert_eq!(cursor.color, Color::GREEN);
}

#[test]
fn materialize_preserves_phys_cursor() {
    let mut state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    state.phys_cursor = Some(PhysCursor {
        window_id: DisplayWindowId::new(11),
        charpos: 42,
        row: 3,
        col: 5,
        x: 80.0,
        y: 48.0,
        width: 9.0,
        height: 18.0,
        ascent: 13.0,
        style: CursorStyle::Hollow,
        color: Color::BLUE,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(11),
            row: 3,
            col: 5,
        },
        cursor_fg: Color::WHITE,
    });

    let buf = state.materialize();
    let phys = buf.active_cursor().expect("preserved active cursor");
    assert_eq!(phys.window_id.get(), 11);
    assert_eq!(phys.slot_id.row, 3);
    assert_eq!(phys.slot_id.col, 5);
    assert!(phys.active);
    assert_eq!(phys.style, CursorStyle::Hollow);
}

#[test]
fn materialize_includes_scroll_bars() {
    let mut state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    state.scroll_bars.push(ScrollBarItem {
        window_id: DisplayWindowId::new(42),
        row_role: GlyphRowRole::Text,
        clip_rect: Some(Rect::new(0.0, 0.0, 640.0, 384.0)),
        horizontal: false,
        x: 632.0,
        y: 0.0,
        width: 8.0,
        height: 384.0,
        position: 10,
        portion: 50,
        whole: 200,
        thumb_start: 10.0,
        thumb_size: 50.0,
        track_color: Color::BLACK,
        thumb_color: Color::WHITE,
    });
    let buf = state.materialize();
    assert_eq!(buf.glyphs.len(), 1);
    assert!(matches!(&buf.glyphs[0], FrameGlyph::ScrollBar { .. }));
}

#[test]
fn for_each_glyph_matches_materialize_glyphs() {
    // Build a state exercising several glyph kinds at once: a background, a
    // window row with both a Char and a Stretch slot, and a scroll bar. This
    // pins down that `for_each_glyph` walks the matrix in the exact same order
    // and with the exact same constructions as `materialize()` builds
    // `buf.glyphs`.
    let char_w = 8.0f32;
    let char_h = 16.0f32;
    let cols = 4;
    let mut state = FrameDisplayState::new(cols, 1, char_w, char_h);
    state.faces.insert(0, Face::new(0));

    // One background (emits FrameGlyph::Background).
    state.backgrounds.push(BackgroundItem {
        bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, char_h),
        color: Color::RED,
    });

    // One window matrix row: two chars then a 2-col stretch
    // (emits FrameGlyph::Char x2 and FrameGlyph::Stretch).
    let mut matrix = GlyphMatrix::new(1, cols);
    matrix.rows[0].enabled = true;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('A', 0, 0));
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('B', 0, 1));
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::stretch(2, 0));
    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, char_h),
        text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, char_h),
        selected: true,
    });

    // One scroll bar (emits FrameGlyph::ScrollBar).
    state.scroll_bars.push(ScrollBarItem {
        window_id: DisplayWindowId::new(1),
        row_role: GlyphRowRole::Text,
        clip_rect: Some(Rect::new(0.0, 0.0, cols as f32 * char_w, char_h)),
        horizontal: false,
        x: 24.0,
        y: 0.0,
        width: 8.0,
        height: char_h,
        position: 10,
        portion: 50,
        whole: 200,
        thumb_start: 10.0,
        thumb_size: 50.0,
        track_color: Color::BLACK,
        thumb_color: Color::WHITE,
    });

    let buf = state.materialize();
    let mut walked = Vec::new();
    state.for_each_glyph(|g| walked.push(g));

    // Sanity: all four kinds actually appeared, so the comparison is meaningful.
    assert!(
        buf.glyphs
            .iter()
            .any(|g| matches!(g, FrameGlyph::Background { .. }))
    );
    assert!(
        buf.glyphs
            .iter()
            .any(|g| matches!(g, FrameGlyph::Char { .. }))
    );
    assert!(
        buf.glyphs
            .iter()
            .any(|g| matches!(g, FrameGlyph::Stretch { .. }))
    );
    assert!(
        buf.glyphs
            .iter()
            .any(|g| matches!(g, FrameGlyph::ScrollBar { .. }))
    );

    // FrameGlyph has no PartialEq, so compare via Debug strings.
    assert_eq!(format!("{:?}", buf.glyphs), format!("{:?}", walked));
}

#[test]
fn materialize_pixel_positions_from_grid() {
    let char_w = 10.0f32;
    let char_h = 20.0f32;
    let cols = 3;
    let rows = 2;
    let mut state = FrameDisplayState::new(cols, rows, char_w, char_h);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(2, cols);
    matrix.rows[0].enabled = true;
    matrix.rows[1].enabled = true;
    // Row 0: "AB"
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('A', 0, 0));
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('B', 0, 1));
    // Row 1: "C"
    matrix.rows[1].glyphs[GlyphArea::Text as usize].push(Glyph::char('C', 0, 2));

    let win_x = 5.0f32;
    let win_y = 3.0f32;
    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(win_x, win_y, cols as f32 * char_w, rows as f32 * char_h),
        text_pixel_bounds: Rect::new(win_x, win_y, cols as f32 * char_w, rows as f32 * char_h),
        selected: true,
    });

    let buf = state.materialize();
    assert_eq!(buf.glyphs.len(), 3);

    // Glyph 'A' at (win_x + 0*char_w, win_y + 0*char_h)
    match &buf.glyphs[0] {
        FrameGlyph::Char {
            char: ch,
            x,
            y,
            width,
            height,
            ..
        } => {
            assert_eq!(*ch, 'A');
            assert_eq!(*x, win_x);
            assert_eq!(*y, win_y);
            assert_eq!(*width, char_w);
            assert_eq!(*height, char_h);
        }
        other => panic!("expected Char, got {:?}", other),
    }

    // Glyph 'B' at (win_x + 1*char_w, win_y + 0*char_h)
    match &buf.glyphs[1] {
        FrameGlyph::Char { char: ch, x, y, .. } => {
            assert_eq!(*ch, 'B');
            assert_eq!(*x, win_x + char_w);
            assert_eq!(*y, win_y);
        }
        other => panic!("expected Char, got {:?}", other),
    }

    // Glyph 'C' at (win_x + 0*char_w, win_y + 1*char_h)
    match &buf.glyphs[2] {
        FrameGlyph::Char { char: ch, x, y, .. } => {
            assert_eq!(*ch, 'C');
            assert_eq!(*x, win_x);
            assert_eq!(*y, win_y + char_h);
        }
        other => panic!("expected Char, got {:?}", other),
    }
}

#[test]
fn materialize_preserves_char_bidi_level() {
    let mut state = FrameDisplayState::new(1, 1, 8.0, 16.0);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(1, 1);
    matrix.rows[0].enabled = true;
    let mut glyph = Glyph::char('א', 0, 1);
    glyph.bidi_level = 1;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(glyph);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 8.0, 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 8.0, 16.0),
        selected: true,
    });

    let buf = state.materialize();
    match &buf.glyphs[0] {
        FrameGlyph::Char {
            char: ch,
            bidi_level,
            ..
        } => {
            assert_eq!(*ch, 'א');
            assert_eq!(*bidi_level, 1);
            assert_eq!(buf.glyphs[0].bidi_level(), Some(1));
        }
        other => panic!("expected Char, got {:?}", other),
    }
}

#[test]
fn materialize_preserves_stretch_bidi_level() {
    let mut state = FrameDisplayState::new(4, 1, 8.0, 16.0);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(1, 4);
    matrix.rows[0].enabled = true;
    let mut glyph = Glyph::stretch(3, 0);
    glyph.bidi_level = 1;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(glyph);

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 32.0, 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 32.0, 16.0),
        selected: true,
    });

    let buf = state.materialize();
    match &buf.glyphs[0] {
        FrameGlyph::Stretch {
            bidi_level, width, ..
        } => {
            assert_eq!(*bidi_level, 1);
            assert_eq!(*width, 24.0);
            assert_eq!(buf.glyphs[0].bidi_level(), Some(1));
        }
        other => panic!("expected Stretch, got {:?}", other),
    }
}

#[test]
fn materialize_uses_explicit_row_metrics() {
    let mut state = FrameDisplayState::new(2, 1, 10.0, 20.0);
    let mut face = Face::new(0);
    face.font_ascent = 14;
    state.faces.insert(0, face);

    let mut matrix = GlyphMatrix::new(1, 2);
    matrix.rows[0].enabled = true;
    matrix.rows[0].pixel_y = 7.0;
    matrix.rows[0].height_px = 18.0;
    matrix.rows[0].ascent_px = 13.0;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('A', 0, 0));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(5.0, 3.0, 20.0, 18.0),
        text_pixel_bounds: Rect::new(5.0, 3.0, 20.0, 18.0),
        selected: true,
    });

    let buf = state.materialize();
    assert_eq!(buf.glyphs.len(), 1);
    match &buf.glyphs[0] {
        FrameGlyph::Char {
            char,
            x,
            y,
            baseline,
            height,
            ascent,
            ..
        } => {
            assert_eq!(*char, 'A');
            assert_eq!(*x, 5.0);
            assert_eq!(*y, 10.0);
            assert_eq!(*baseline, 23.0);
            assert_eq!(*height, 18.0);
            assert_eq!(*ascent, 14.0);
        }
        other => panic!("expected Char, got {:?}", other),
    }
}

#[test]
fn materialize_applies_glyph_vertical_offset_to_char_baseline() {
    let mut state = FrameDisplayState::new(2, 1, 10.0, 20.0);
    let mut matrix = GlyphMatrix::new(1, 2);
    matrix.rows[0].enabled = true;
    matrix.rows[0].height_px = 20.0;
    matrix.rows[0].ascent_px = 15.0;
    matrix.rows[0].glyphs[GlyphArea::Text as usize]
        .push(Glyph::char('A', 0, 0).with_vertical_offset(-4.0));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 20.0, 20.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 20.0, 20.0),
        selected: true,
    });

    let buf = state.materialize();
    assert_eq!(buf.glyphs.len(), 1);
    match &buf.glyphs[0] {
        FrameGlyph::Char { baseline, .. } => {
            assert_eq!(*baseline, 11.0);
        }
        other => panic!("expected Char, got {:?}", other),
    }
}

#[test]
fn materialize_copies_metadata() {
    let mut state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    state.frame_id = DisplayFrameId::new(123);
    state.parent_id = DisplayFrameId::new(456);
    state.parent_x = 10.0;
    state.parent_y = 20.0;
    state.z_order = 5;
    state.background = Color::BLUE;

    let mut face = Face::new(1);
    face.foreground = Color::RED;
    state.faces.insert(1, face);

    let buf = state.materialize();
    assert_eq!(buf.frame_id.get(), 123);
    assert_eq!(buf.parent_id.get(), 456);
    assert_eq!(buf.parent_x, 10.0);
    assert_eq!(buf.parent_y, 20.0);
    assert_eq!(buf.z_order, 5);
    assert_eq!(buf.background, Color::BLUE);
    assert!(buf.faces.contains_key(&1));
    assert_eq!(buf.faces[&1].foreground, Color::RED);
}

#[test]
fn materialize_disabled_rows_are_skipped() {
    let mut state = FrameDisplayState::new(3, 2, 8.0, 16.0);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(2, 3);
    matrix.rows[0].enabled = true;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('A', 0, 0));
    // Row 1 stays disabled (default), so its glyph is filtered out.
    matrix.rows[1].glyphs[GlyphArea::Text as usize].push(Glyph::char('B', 0, 1));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 24.0, 32.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 24.0, 32.0),
        selected: true,
    });

    let buf = state.materialize();
    // Only row 0's glyph should be materialized
    assert_eq!(buf.glyphs.len(), 1);
    match &buf.glyphs[0] {
        FrameGlyph::Char { char: ch, .. } => assert_eq!(*ch, 'A'),
        other => panic!("expected Char, got {:?}", other),
    }
}

#[test]
fn materialize_padding_glyphs_are_skipped() {
    let mut state = FrameDisplayState::new(4, 1, 8.0, 16.0);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(1, 4);
    matrix.rows[0].enabled = true;
    // Wide char 'W' followed by padding
    let mut wide_glyph = Glyph::char('W', 0, 0);
    wide_glyph.wide = true;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(wide_glyph);
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::padding_for(0, 0));
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('x', 0, 1));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 32.0, 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 32.0, 16.0),
        selected: true,
    });

    let buf = state.materialize();
    // Should have 2 visible glyphs: wide 'W' and 'x'; padding is skipped
    assert_eq!(buf.glyphs.len(), 2);
    match &buf.glyphs[0] {
        FrameGlyph::Char {
            char: ch, width, ..
        } => {
            assert_eq!(*ch, 'W');
            assert_eq!(*width, 16.0); // 2 * char_w for wide
        }
        other => panic!("expected wide Char, got {:?}", other),
    }
    match &buf.glyphs[1] {
        FrameGlyph::Char { char: ch, x, .. } => {
            assert_eq!(*ch, 'x');
            // col = 2 (wide took 2 cols), so x = 2 * 8.0 = 16.0
            assert_eq!(*x, 16.0);
        }
        other => panic!("expected Char, got {:?}", other),
    }
}

#[test]
fn materialize_uses_realized_pixel_width_for_text_positions() {
    let mut state = FrameDisplayState::new(10, 1, 8.0, 16.0);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(1, 10);
    matrix.rows[0].enabled = true;
    matrix.rows[0].glyphs[GlyphArea::Text as usize]
        .push(Glyph::char('N', 0, 0).with_pixel_width(13.0));
    matrix.rows[0].glyphs[GlyphArea::Text as usize]
        .push(Glyph::char('E', 0, 1).with_pixel_width(12.0));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 16.0),
        selected: true,
    });

    let buf = state.materialize();
    assert_eq!(buf.glyphs.len(), 2);
    match (&buf.glyphs[0], &buf.glyphs[1]) {
        (
            FrameGlyph::Char {
                char: first,
                x: first_x,
                width: first_width,
                ..
            },
            FrameGlyph::Char {
                char: second,
                x: second_x,
                width: second_width,
                ..
            },
        ) => {
            assert_eq!((*first, *second), ('N', 'E'));
            assert_eq!(*first_x, 0.0);
            assert_eq!(*first_width, 13.0);
            assert_eq!(*second_x, 13.0);
            assert_eq!(*second_width, 12.0);
        }
        other => panic!("expected two chars, got {:?}", other),
    }
}

#[test]
fn materialize_clips_overlong_window_rows_to_pixel_bounds() {
    let mut state = FrameDisplayState::new(6, 1, 8.0, 16.0);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(1, 3);
    matrix.rows[0].enabled = true;
    matrix.rows[0].role = GlyphRowRole::ModeLine;
    matrix.rows[0].mode_line = true;
    for (idx, ch) in "abcdef".chars().enumerate() {
        matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, 0, idx));
    }

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 24.0, 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 24.0, 16.0),
        selected: true,
    });

    let buf = state.materialize();
    let chars: Vec<(char, f32, f32)> = buf
        .glyphs
        .iter()
        .filter_map(|glyph| match glyph {
            FrameGlyph::Char {
                char: ch, x, width, ..
            } => Some((*ch, *x, *width)),
            _ => None,
        })
        .collect();

    assert_eq!(
        chars,
        vec![('a', 0.0, 8.0), ('b', 8.0, 8.0), ('c', 16.0, 8.0)]
    );
    assert!(
        buf.glyphs.iter().all(|glyph| match glyph {
            FrameGlyph::Char { x, width, .. } | FrameGlyph::Stretch { x, width, .. } =>
                *x + *width <= 24.0,
            _ => true,
        }),
        "materialized row glyphs must stay inside their owning window"
    );
}

#[test]
fn materialize_text_rows_from_text_area_but_chrome_from_window_area() {
    let mut state = FrameDisplayState::new(10, 2, 8.0, 16.0);
    let mut matrix = GlyphMatrix::new(2, 4);

    matrix.rows[0].enabled = true;
    matrix.rows[0].role = GlyphRowRole::Text;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::char('t', 0, 0));

    matrix.rows[1].enabled = true;
    matrix.rows[1].role = GlyphRowRole::ModeLine;
    matrix.rows[1].glyphs[GlyphArea::Text as usize].push(Glyph::char('m', 0, 1));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 32.0),
        text_pixel_bounds: Rect::new(8.0, 0.0, 64.0, 16.0),
        selected: true,
    });

    let buf = state.materialize();
    let text = buf
        .glyphs
        .iter()
        .find(|glyph| matches!(glyph, FrameGlyph::Char { char: 't', .. }))
        .expect("text glyph");
    let chrome = buf
        .glyphs
        .iter()
        .find(|glyph| matches!(glyph, FrameGlyph::Char { char: 'm', .. }))
        .expect("mode-line glyph");

    assert!(matches!(
        text,
        FrameGlyph::Char {
            x: 8.0,
            width: 16.0,
            ..
        }
    ));
    assert!(matches!(
        chrome,
        FrameGlyph::Char {
            x: 0.0,
            width: 20.0,
            ..
        }
    ));
}

#[test]
fn materialize_stretch_glyph() {
    let mut state = FrameDisplayState::new(10, 1, 8.0, 16.0);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(1, 10);
    matrix.rows[0].enabled = true;
    matrix.rows[0].glyphs[GlyphArea::Text as usize].push(Glyph::stretch(4, 0));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, 80.0, 16.0),
        text_pixel_bounds: Rect::new(0.0, 0.0, 80.0, 16.0),
        selected: true,
    });

    let buf = state.materialize();
    assert_eq!(buf.glyphs.len(), 1);
    match &buf.glyphs[0] {
        FrameGlyph::Stretch { width, height, .. } => {
            assert_eq!(*width, 4.0 * 8.0); // 4 cols * 8px
            assert_eq!(*height, 16.0);
        }
        other => panic!("expected Stretch, got {:?}", other),
    }
}

#[test]
fn materialize_uses_explicit_stretch_geometry() {
    let mut state = FrameDisplayState::new(10, 1, 8.0, 16.0);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(1, 10);
    matrix.rows[0].enabled = true;
    matrix.rows[0].height_px = 30.0;
    matrix.rows[0].ascent_px = 20.0;
    matrix.rows[0].glyphs[GlyphArea::Text as usize]
        .push(Glyph::stretch(4, 0).with_pixel_geometry(24.0, 12.0, 5.0));

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 10.0, 80.0, 40.0),
        text_pixel_bounds: Rect::new(0.0, 10.0, 80.0, 40.0),
        selected: true,
    });

    let buf = state.materialize();
    match &buf.glyphs[0] {
        FrameGlyph::Stretch {
            y, width, height, ..
        } => {
            assert_eq!(*y, 25.0);
            assert_eq!(*width, 24.0);
            assert_eq!(*height, 12.0);
        }
        other => panic!("expected Stretch, got {:?}", other),
    }
}

#[test]
fn materialize_new_fields_default_to_empty() {
    let state = FrameDisplayState::new(80, 24, 8.0, 16.0);
    assert!(state.backgrounds.is_empty());
    assert!(state.borders.is_empty());
    assert!(state.cursors.is_empty());
    assert!(state.images.is_empty());
    assert!(state.videos.is_empty());
    assert!(state.xwidgets.is_empty());
    assert!(state.scroll_bars.is_empty());
    assert!(state.stipple_patterns.is_empty());
    assert!(state.effect_hints.is_empty());
}

#[test]
fn materialize_mixed_grid_and_nongrid_items() {
    let mut state = state_with_text("Hi");

    // Add one background and one cursor
    state.backgrounds.push(BackgroundItem {
        bounds: Rect::new(0.0, 0.0, 16.0, 16.0),
        color: Color::BLACK,
    });
    state.cursors.push(CursorItem {
        window_id: DisplayWindowId::new(1),
        slot_id: DisplaySlotId::from_pixels(DisplayWindowId::new(1), 0.0, 0.0, 8.0, 16.0),
        x: 0.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        style: CursorStyle::FilledBox,
        color: Color::WHITE,
    });

    let buf = state.materialize();
    // 1 background + 2 chars = 3 glyphs, plus 1 decorative window cursor
    assert_eq!(buf.glyphs.len(), 3);
    assert_eq!(buf.window_cursors.len(), 1);

    // Backgrounds come first
    assert!(matches!(&buf.glyphs[0], FrameGlyph::Background { .. }));
    // Then grid chars
    assert!(matches!(&buf.glyphs[1], FrameGlyph::Char { .. }));
    assert!(matches!(&buf.glyphs[2], FrameGlyph::Char { .. }));
    assert_eq!(buf.window_cursors[0].style, CursorStyle::FilledBox);
}

#[test]
fn materialize_emits_left_fringe_bitmap_glyph_from_row() {
    // A buffer-text row that carries a left-fringe bitmap (magit section-heading
    // fold arrow) must materialize one FrameGlyph::FringeBitmap positioned in the
    // window's left fringe column (between the window edge and the text area),
    // with the row's bitmap index and resolved face id.
    use crate::frame_glyphs::{FringeBitmapData, FringeSide};
    use crate::glyph_matrix::FringeBitmapInfo;

    let char_w = 8.0f32;
    let char_h = 16.0f32;
    let cols = 4;
    let left_fringe = 8.0f32;
    // Window spans from x=10; the text area starts after an 8px left fringe.
    let win = Rect::new(10.0, 20.0, left_fringe + cols as f32 * char_w, char_h);
    let text_area = Rect::new(10.0 + left_fringe, 20.0, cols as f32 * char_w, char_h);

    let mut state = FrameDisplayState::new(cols, 1, char_w, char_h);
    state.faces.insert(0, Face::new(0));
    state.faces.insert(7, Face::new(7));

    // Register the bitmap bits once per frame.
    state.fringe_bitmaps.insert(
        25,
        FringeBitmapData {
            bits: vec![0x6000, 0x3000, 0x1800, 0x0C00],
            width: 8,
            height: 4,
            period: 0,
            align: 0,
        },
    );

    let mut matrix = GlyphMatrix::new(1, cols);
    matrix.rows[0].enabled = true;
    matrix.rows[0].height_px = char_h;
    matrix.rows[0].pixel_y = 0.0;
    matrix.rows[0].left_fringe_bitmap = Some(FringeBitmapInfo {
        bitmap_index: 25,
        face_id: 7,
    });

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: win,
        text_pixel_bounds: text_area,
        selected: true,
    });

    let buf = state.materialize();
    let fringes: Vec<_> = buf
        .glyphs
        .iter()
        .filter(|g| matches!(g, FrameGlyph::FringeBitmap { .. }))
        .collect();
    assert_eq!(fringes.len(), 1, "exactly one fringe bitmap glyph");
    match fringes[0] {
        FrameGlyph::FringeBitmap {
            x,
            y,
            width,
            height,
            bitmap_index,
            face_id,
            side,
            ..
        } => {
            assert_eq!(*bitmap_index, 25);
            assert_eq!(*face_id, 7);
            assert_eq!(*side, FringeSide::Left);
            // Fringe column: from window left edge to text area left edge.
            assert_eq!(*x, 10.0);
            assert_eq!(*width, left_fringe);
            assert_eq!(*y, 20.0);
            assert_eq!(*height, char_h);
        }
        other => panic!("expected FringeBitmap, got {other:?}"),
    }

    // The bits round-trip into the materialized buffer.
    assert!(buf.fringe_bitmaps.contains_key(&25));
}
