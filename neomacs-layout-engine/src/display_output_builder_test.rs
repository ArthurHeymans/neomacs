use super::*;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor,
};
use neomacs_display_protocol::types::Rect;

fn write_char_to_current_row(
    builder: &mut DisplayOutputBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
) {
    write_char_to_current_row_with_width(builder, ch, face_id, charpos, 0.0);
}

fn write_char_to_current_row_with_width(
    builder: &mut DisplayOutputBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
    pixel_width: f32,
) {
    builder
        .edit_current_row_for_test(|row| {
            crate::glyph_row_writer::push_char_to_row(row, ch, face_id, charpos, pixel_width);
        })
        .expect("current row");
}

fn write_wide_char_to_current_row(
    builder: &mut DisplayOutputBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
) {
    builder
        .edit_current_row_for_test(|row| {
            crate::glyph_row_writer::push_wide_char_to_row(row, ch, face_id, charpos, 0.0);
        })
        .expect("current row");
}

fn write_cluster_continuation_to_current_row(
    builder: &mut DisplayOutputBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
) {
    builder
        .edit_current_row_for_test(|row| {
            crate::glyph_row_writer::push_cluster_continuation_to_row(row, ch, face_id, charpos);
        })
        .expect("current row");
}

fn write_run_member_to_current_row(
    builder: &mut DisplayOutputBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
    pixel_width: f32,
) {
    builder
        .edit_current_row_for_test(|row| {
            crate::glyph_row_writer::push_run_member_to_row(row, ch, face_id, charpos, pixel_width);
        })
        .expect("current row");
}

fn write_stretch_to_current_row(builder: &mut DisplayOutputBuilder, width_cols: u16, face_id: u32) {
    builder
        .edit_current_row_for_test(|row| {
            crate::glyph_row_writer::push_stretch_to_row(row, width_cols, face_id, 0.0, 0.0, 0.0);
        })
        .expect("current row");
}

fn write_left_margin_char_to_current_row(
    builder: &mut DisplayOutputBuilder,
    ch: char,
    face_id: u32,
) {
    builder
        .edit_current_row_for_test(|row| {
            row.glyphs[GlyphArea::LeftMargin.index()].push(Glyph::char(ch, face_id, 0));
        })
        .expect("current row");
}

fn write_left_margin_stretch_to_current_row(
    builder: &mut DisplayOutputBuilder,
    width_cols: u16,
    face_id: u32,
) {
    builder
        .edit_current_row_for_test(|row| {
            row.glyphs[GlyphArea::LeftMargin.index()].push(Glyph::stretch(width_cols, face_id));
        })
        .expect("current row");
}

fn external_text_row(role: GlyphRowRole, glyphs: Vec<Glyph>) -> GlyphRow {
    let mut row = GlyphRow::new(role);
    row.mode_line = matches!(
        role,
        GlyphRowRole::ModeLine | GlyphRowRole::HeaderLine | GlyphRowRole::TabLine
    );
    row.displays_text = !glyphs.is_empty();
    row.glyphs[GlyphArea::Text.index()] = glyphs;
    crate::glyph_row_writer::normalize_external_row(&mut row);
    row
}

#[test]
fn builder_starts_empty() {
    let builder = DisplayOutputBuilder::new();
    let state = builder.finish(80, 24, 8.0, 16.0);
    assert!(state.window_matrices.is_empty());
}

#[test]
fn builder_applies_row_begin_lifecycle_to_matrix_row() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 20.0), true);
    builder.begin_output_row(0, GlyphRowRole::ModeLine, true);
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];
    assert!(row.enabled);
    assert_eq!(row.role, GlyphRowRole::ModeLine);
    assert!(row.mode_line);
}

#[test]
fn builder_can_preserve_exact_frame_pixel_size() {
    let builder = DisplayOutputBuilder::new();
    let state = builder.finish_with_pixel_size(79, 36, 16.25, 33.0, 1300.0, 1188.0);

    assert_eq!(state.frame_cols, 79);
    assert_eq!(state.frame_rows, 36);
    assert_eq!(state.frame_pixel_width, 1300.0);
    assert_eq!(state.frame_pixel_height, 1188.0);
}

#[test]
fn builder_tracks_single_window_single_row() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 24, 80, Rect::new(0.0, 0.0, 640.0, 384.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'H', 0, 0);
    write_char_to_current_row(&mut builder, 'i', 0, 1);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.window_matrices.len(), 1);
    let matrix = &state.window_matrices[0].matrix;
    assert_eq!(matrix.nrows, 24);
    assert_eq!(matrix.ncols, 80);
    assert_eq!(matrix.rows[0].used(GlyphArea::Text), 2);

    let g0 = &matrix.rows[0].glyphs[GlyphArea::Text as usize][0];
    assert_eq!(g0.glyph_type, GlyphType::Char { ch: 'H' });
    assert_eq!(g0.face_id, 0);
    assert_eq!(g0.charpos, 0);

    let g1 = &matrix.rows[0].glyphs[GlyphArea::Text as usize][1];
    assert_eq!(g1.glyph_type, GlyphType::Char { ch: 'i' });
    assert_eq!(g1.charpos, 1);
}

#[test]
fn output_builder_installs_complete_row_metrics_and_cursor() {
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    crate::glyph_row_writer::push_char_to_row(&mut row, 'x', 3, 11, 8.0);

    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(0.0, 4.0, 80.0, 32.0), true);
    builder.install_complete_output_row(0, GlyphRowRole::Text, false, row);
    builder.set_output_row_metrics(0, 12.0, 18.0, 13.0);
    builder.set_output_row_cursor(0, 1, CursorStyle::Bar(2.0));
    builder.mark_current_output_row_truncated_left();
    builder.finalize_output_row_index(0);
    builder.end_window();

    let state = builder.finish(10, 2, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];
    assert_eq!(row.used(GlyphArea::Text), 1);
    assert_eq!(row.pixel_y, 12.0);
    assert_eq!(row.height_px, 18.0);
    assert_eq!(row.ascent_px, 13.0);
    assert_eq!(row.cursor_col, Some(1));
    assert_eq!(row.cursor_type, Some(CursorStyle::Bar(2.0)));
    assert!(row.truncated_left);
    assert_ne!(row.hash, 0);
}

#[test]
fn builder_tracks_multiple_rows() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 10, Rect::new(0.0, 0.0, 80.0, 48.0), true);

    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'a', 0, 0);
    builder.end_row();

    builder.begin_row(1, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'b', 0, 5);
    write_char_to_current_row(&mut builder, 'c', 0, 6);
    builder.end_row();

    builder.end_window();

    let state = builder.finish(10, 3, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;
    assert_eq!(matrix.rows[0].used(GlyphArea::Text), 1);
    assert_eq!(matrix.rows[1].used(GlyphArea::Text), 2);
    assert_eq!(matrix.rows[2].used(GlyphArea::Text), 0);
}

#[test]
fn builder_stores_row_metrics_as_provided() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(5.0, 20.0, 80.0, 40.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    builder.set_current_row_metrics(6.0, 18.0, 13.0);
    write_char_to_current_row(&mut builder, 'x', 0, 0);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(10, 2, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];
    assert_eq!(row.pixel_y, 6.0);
    assert_eq!(row.height_px, 18.0);
    assert_eq!(row.ascent_px, 13.0);
}

#[test]
fn builder_normalizes_row_metrics_during_install() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 20.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    builder.set_current_row_metrics(5.0, -4.0, 9.0);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];
    assert_eq!(row.pixel_y, 5.0);
    assert_eq!(row.height_px, 0.0);
    assert_eq!(row.ascent_px, 0.0);
}

#[test]
fn builder_tracks_wide_chars() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 5, 20, Rect::new(0.0, 0.0, 160.0, 80.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_wide_char_to_current_row(&mut builder, '\u{4e16}', 0, 0);
    write_char_to_current_row(&mut builder, 'x', 0, 3);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(20, 5, 8.0, 16.0);
    let glyphs = &state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::Text as usize];
    assert_eq!(glyphs.len(), 3);
    assert!(glyphs[0].wide);
    assert!(!glyphs[0].padding);
    assert!(glyphs[1].padding);
    assert!(!glyphs[2].wide);
    assert!(!glyphs[2].padding);
}

#[test]
fn builder_handles_stretch_glyphs() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 5, 20, Rect::new(0.0, 0.0, 160.0, 80.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'a', 0, 0);
    write_stretch_to_current_row(&mut builder, 4, 0);
    write_char_to_current_row(&mut builder, 'b', 0, 5);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(20, 5, 8.0, 16.0);
    let glyphs = &state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::Text as usize];
    assert_eq!(glyphs.len(), 3);
    assert_eq!(glyphs[1].glyph_type, GlyphType::Stretch { width_cols: 4 });
}

#[test]
fn builder_computes_row_hashes_on_finish() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(0.0, 0.0, 80.0, 32.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'x', 0, 0);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(10, 2, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];
    assert_ne!(row.hash, 0, "hash should be computed on finish");
}

#[test]
fn builder_resets_on_new_frame() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(0.0, 0.0, 80.0, 32.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'x', 0, 0);
    builder.end_row();
    builder.end_window();

    builder.reset();

    let state = builder.finish(10, 2, 8.0, 16.0);
    assert!(state.window_matrices.is_empty());
}

#[test]
fn builder_installs_status_line_row_glyphs_wholesale() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 4, 80, Rect::new(0.0, 0.0, 640.0, 64.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'a', 0, 0);
    builder.end_row();

    let glyphs = vec![
        Glyph::char('-', 5, 0),
        Glyph::char('U', 5, 0),
        Glyph::char(':', 5, 0),
    ];
    let row = external_text_row(GlyphRowRole::ModeLine, glyphs);
    builder.install_display_row(3, &row);
    builder.end_window();

    let state = builder.finish(80, 4, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;

    assert_eq!(matrix.nrows, 4);
    assert_eq!(matrix.rows.len(), 4);

    let ml_row = &matrix.rows[3];
    assert_eq!(ml_row.role, GlyphRowRole::ModeLine);
    assert!(ml_row.enabled);
    assert!(ml_row.mode_line);

    let ml_glyphs = &ml_row.glyphs[GlyphArea::Text as usize];
    assert_eq!(ml_glyphs.len(), 3);
    assert_eq!(ml_glyphs[0].glyph_type, GlyphType::Char { ch: '-' });
    assert_eq!(ml_glyphs[1].glyph_type, GlyphType::Char { ch: 'U' });
    assert_eq!(ml_glyphs[2].glyph_type, GlyphType::Char { ch: ':' });
    assert_eq!(ml_glyphs[0].face_id, 5);
}

#[test]
fn builder_install_display_row_preserves_row_and_relative_metrics() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 20.0, 640.0, 60.0), true);

    let mut row = external_text_row(GlyphRowRole::Text, vec![Glyph::char('z', 7, 42)]);
    row.pixel_y = 44.0;
    row.height_px = 18.0;
    row.ascent_px = 13.0;
    row.start_charpos = 42;
    row.end_charpos = 43;

    builder.install_display_row(1, &row);
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let installed = &state.window_matrices[0].matrix.rows[1];
    assert_eq!(installed.role, GlyphRowRole::Text);
    assert!(installed.enabled);
    assert_eq!(installed.pixel_y, 24.0);
    assert_eq!(installed.height_px, 18.0);
    assert_eq!(installed.ascent_px, 13.0);
    assert_eq!(installed.start_charpos, 42);
    assert_eq!(installed.end_charpos, 43);
    let glyphs = &installed.glyphs[GlyphArea::Text as usize];
    assert_eq!(glyphs.len(), 1);
    assert_eq!(glyphs[0].glyph_type, GlyphType::Char { ch: 'z' });
    assert_eq!(glyphs[0].face_id, 7);
}

#[test]
fn builder_status_line_empty_row_when_no_chars_pushed() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 40, Rect::new(0.0, 0.0, 320.0, 48.0), true);

    let row = external_text_row(GlyphRowRole::ModeLine, Vec::new());
    builder.install_display_row(2, &row);
    builder.end_window();

    let state = builder.finish(40, 3, 8.0, 16.0);
    let ml_row = &state.window_matrices[0].matrix.rows[2];
    assert_eq!(ml_row.glyphs[GlyphArea::Text as usize].len(), 0);
}

#[test]
fn builder_display_row_without_window_is_noop() {
    let mut builder = DisplayOutputBuilder::new();
    let row = external_text_row(GlyphRowRole::ModeLine, vec![Glyph::char('x', 0, 0)]);
    builder.install_display_row(0, &row);
    let state = builder.finish(80, 24, 8.0, 16.0);
    assert!(state.window_matrices.is_empty());
}

#[test]
fn builder_left_margin_chars() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_left_margin_stretch_to_current_row(&mut builder, 2, 1);
    write_left_margin_char_to_current_row(&mut builder, '4', 1);
    write_left_margin_char_to_current_row(&mut builder, '2', 1);
    write_char_to_current_row(&mut builder, 'H', 0, 0);
    write_char_to_current_row(&mut builder, 'i', 0, 1);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];

    // Left margin should have stretch + 2 digit chars
    let lm = &row.glyphs[GlyphArea::LeftMargin as usize];
    assert_eq!(lm.len(), 3);
    assert_eq!(lm[0].glyph_type, GlyphType::Stretch { width_cols: 2 });
    assert_eq!(lm[1].glyph_type, GlyphType::Char { ch: '4' });
    assert_eq!(lm[2].glyph_type, GlyphType::Char { ch: '2' });

    // Text area should have the buffer chars
    let text = &row.glyphs[GlyphArea::Text as usize];
    assert_eq!(text.len(), 2);
    assert_eq!(text[0].glyph_type, GlyphType::Char { ch: 'H' });
}

#[test]
fn builder_set_cursor_at_row() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'a', 0, 0);
    builder.end_row();
    builder.begin_row(1, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'b', 0, 5);
    builder.end_row();

    // Set cursor on row 1, column 0
    builder.set_cursor_at_row(1, 0, CursorStyle::FilledBox);
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;

    assert!(matrix.rows[0].cursor_col.is_none());
    assert_eq!(matrix.rows[1].cursor_col, Some(0));
    assert_eq!(matrix.rows[1].cursor_type, Some(CursorStyle::FilledBox));
}

#[test]
fn builder_preserves_phys_cursor() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'a', 0, 0);
    builder.end_row();
    builder.set_phys_cursor(PhysCursor {
        window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
        charpos: 0,
        row: 0,
        col: 0,
        slot_id: DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
            row: 0,
            col: 0,
        },
        x: 0.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: neomacs_display_protocol::types::Color::WHITE,
        cursor_fg: neomacs_display_protocol::types::Color::BLACK,
    });
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let cursor = state.phys_cursor.as_ref().expect("phys cursor");
    assert_eq!(cursor.window_id.get(), 1);
    assert_eq!(cursor.charpos, 0);
    assert_eq!(cursor.col, 0);
}

#[test]
fn builder_preserves_high_window_id_phys_cursor() {
    let high_window_id = 1_u64 << 48;
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(
        high_window_id,
        3,
        80,
        Rect::new(0.0, 0.0, 640.0, 48.0),
        true,
    );
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'M', 0, 0);
    builder.end_row();
    builder.set_phys_cursor(PhysCursor {
        window_id: neomacs_display_protocol::types::DisplayWindowId::new(high_window_id as i64),
        charpos: 0,
        row: 0,
        col: 0,
        slot_id: DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(high_window_id as i64),
            row: 0,
            col: 0,
        },
        x: 0.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: neomacs_display_protocol::types::Color::WHITE,
        cursor_fg: neomacs_display_protocol::types::Color::BLACK,
    });
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let cursor = state.phys_cursor.as_ref().expect("phys cursor");
    assert_eq!(cursor.window_id.get(), high_window_id as i64);
    assert_eq!(cursor.slot_id.window_id.get(), high_window_id as i64);
    assert_eq!(state.window_matrices[0].matrix.rows[0].cursor_col, Some(0));
}

#[test]
fn builder_reorders_simple_rtl_row() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'א', 0, 0);
    write_char_to_current_row(&mut builder, 'ב', 0, 1);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let glyphs = &state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::Text as usize];
    assert_eq!(glyphs.len(), 2);
    assert_eq!(glyphs[0].glyph_type, GlyphType::Char { ch: 'ב' });
    assert_eq!(glyphs[1].glyph_type, GlyphType::Char { ch: 'א' });
    assert_eq!(glyphs[0].bidi_level, 1);
    assert_eq!(glyphs[1].bidi_level, 1);
}

#[test]
fn builder_keeps_stretch_fixed_while_reordering_rtl_chars() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'א', 0, 0);
    write_stretch_to_current_row(&mut builder, 3, 0);
    write_char_to_current_row(&mut builder, 'ב', 0, 1);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let glyphs = &state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::Text as usize];
    assert_eq!(glyphs.len(), 3);
    assert_eq!(glyphs[0].glyph_type, GlyphType::Char { ch: 'ב' });
    assert_eq!(glyphs[1].glyph_type, GlyphType::Stretch { width_cols: 3 });
    assert_eq!(glyphs[2].glyph_type, GlyphType::Char { ch: 'א' });
    assert_eq!(glyphs[0].bidi_level, 1);
    assert_eq!(glyphs[1].bidi_level, 1);
    assert_eq!(glyphs[2].bidi_level, 1);
}

#[test]
fn builder_reorders_wide_rtl_row() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_wide_char_to_current_row(&mut builder, 'א', 0, 0);
    write_char_to_current_row(&mut builder, 'ב', 0, 1);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let glyphs = &state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::Text as usize];
    assert_eq!(glyphs.len(), 3);
    assert_eq!(glyphs[0].glyph_type, GlyphType::Char { ch: 'ב' });
    assert_eq!(glyphs[1].glyph_type, GlyphType::Char { ch: 'א' });
    assert!(glyphs[1].wide);
    assert!(glyphs[2].padding);
    assert_eq!(glyphs[0].bidi_level, 1);
    assert_eq!(glyphs[1].bidi_level, 1);
    assert_eq!(glyphs[2].bidi_level, 1);
}

#[test]
fn builder_reorders_wide_rtl_row_across_stretch() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_wide_char_to_current_row(&mut builder, 'א', 0, 0);
    write_stretch_to_current_row(&mut builder, 2, 0);
    write_char_to_current_row(&mut builder, 'ב', 0, 1);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let glyphs = &state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::Text as usize];
    assert_eq!(glyphs.len(), 4);
    assert_eq!(glyphs[0].glyph_type, GlyphType::Char { ch: 'ב' });
    assert_eq!(glyphs[1].glyph_type, GlyphType::Stretch { width_cols: 2 });
    assert_eq!(glyphs[2].glyph_type, GlyphType::Char { ch: 'א' });
    assert!(glyphs[2].wide);
    assert!(glyphs[3].padding);
    assert_eq!(glyphs[0].bidi_level, 1);
    assert_eq!(glyphs[1].bidi_level, 1);
    assert_eq!(glyphs[2].bidi_level, 1);
    assert_eq!(glyphs[3].bidi_level, 1);
}

#[test]
fn builder_remaps_phys_cursor_to_visual_bidi_column() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'א', 0, 0);
    write_char_to_current_row(&mut builder, 'ב', 0, 1);
    builder.end_row();
    builder.set_phys_cursor(PhysCursor {
        window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
        charpos: 0,
        row: 0,
        col: 0,
        slot_id: DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
            row: 0,
            col: 0,
        },
        x: 0.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: neomacs_display_protocol::types::Color::WHITE,
        cursor_fg: neomacs_display_protocol::types::Color::BLACK,
    });
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let cursor = state.phys_cursor.as_ref().expect("phys cursor");
    assert_eq!(cursor.col, 1);
    assert_eq!(cursor.slot_id.col, 1);
    assert_eq!(cursor.x, 8.0);

    let row = &state.window_matrices[0].matrix.rows[0];
    assert_eq!(row.cursor_col, Some(1));
    assert_eq!(row.cursor_type, Some(CursorStyle::FilledBox));
}

#[test]
fn phys_cursor_slot_col_accounts_for_line_number_gutter() {
    // Regression for the Doom `SPC h d h` "two cursors / wrong column" bug.
    //
    // With `display-line-numbers`, the line-number glyphs are emitted into the
    // LeftMargin area and `FrameDisplayState::materialize` assigns slot columns
    // from a single running counter over LeftMargin then Text. The cursor's
    // `slot_id.col` must land on that same counter, otherwise the renderer's
    // `cursor_glyph_slot_rect` resolves the wrong glyph (one `lnum_cols` cells
    // to the left, i.e. inside the gutter), drawing a stray second cursor.
    //
    // GNU does the equivalent in `set_cursor_from_row`: the line-number glyphs
    // sit at the start of TEXT_AREA and the cursor walks past their pixel width
    // before landing on the buffer glyph (src/xdisp.c).
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    // Line-number gutter "12 ": two digits + a one-cell trailing stretch ->
    // three materialize columns (cols 0, 1, 2).
    write_left_margin_char_to_current_row(&mut builder, '1', 1);
    write_left_margin_char_to_current_row(&mut builder, '2', 1);
    write_left_margin_stretch_to_current_row(&mut builder, 1, 1);
    // Buffer text "Hello" at char positions 100..=104 (materialize cols 3..=7).
    write_char_to_current_row(&mut builder, 'H', 0, 100);
    write_char_to_current_row(&mut builder, 'e', 0, 101);
    write_char_to_current_row(&mut builder, 'l', 0, 102);
    write_char_to_current_row(&mut builder, 'l', 0, 103);
    write_char_to_current_row(&mut builder, 'o', 0, 104);
    builder.end_row();

    // Point sits on the first 'l' (charpos 102). The engine's capture passes the
    // Text-area index (2) for both `col` and `slot_id.col`, never counting the
    // gutter -- exactly the input that triggered the bug.
    builder.set_phys_cursor(PhysCursor {
        window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
        charpos: 102,
        row: 0,
        col: 2,
        slot_id: DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
            row: 0,
            col: 2,
        },
        x: 0.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: neomacs_display_protocol::types::Color::WHITE,
        cursor_fg: neomacs_display_protocol::types::Color::BLACK,
    });
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let cursor = state.phys_cursor.as_ref().expect("phys cursor");

    // 3 gutter columns + Text index 2 = materialize column 5.
    assert_eq!(
        cursor.slot_id.col, 5,
        "cursor slot column must include the 3-column line-number gutter"
    );
    assert_eq!(cursor.col, 5);
    // char_w = 640 / 80 = 8.0; grid fallback x = 5 * 8.0.
    assert_eq!(cursor.x, 40.0);

    let row = &state.window_matrices[0].matrix.rows[0];
    assert_eq!(row.cursor_col, Some(5));

    // The glyph the renderer will snap the cursor to (via `slot_glyph`) must be
    // the buffer 'l' at point -- not a gutter stretch and not the wrong letter.
    let buf = state.materialize();
    let slot = buf
        .slot_glyph(cursor.slot_id)
        .expect("cursor slot glyph must be present");
    match slot {
        neomacs_display_protocol::frame_glyphs::FrameGlyph::Char { char, .. } => {
            assert_eq!(
                *char, 'l',
                "cursor must resolve to the buffer glyph at point"
            );
        }
        other => panic!("expected a Char glyph at the cursor slot, got {other:?}"),
    }
}

#[test]
fn glyph_row_resolved_phys_cursor_preserves_display_string_cursor_slot() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);

    for (charpos, ch) in "1/1070 M-x f ".chars().enumerate() {
        write_char_to_current_row(&mut builder, ch, 0, charpos);
    }
    builder.end_row();

    builder.set_glyph_row_resolved_phys_cursor(PhysCursor {
        window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
        charpos: 5,
        row: 0,
        col: 12,
        slot_id: DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
            row: 0,
            col: 12,
        },
        x: 96.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: neomacs_display_protocol::types::Color::WHITE,
        cursor_fg: neomacs_display_protocol::types::Color::BLACK,
    });
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let cursor = state.phys_cursor.as_ref().expect("phys cursor");
    assert_eq!(cursor.col, 12);
    assert_eq!(cursor.slot_id.col, 12);
    assert_eq!(cursor.x, 96.0);
    assert_eq!(state.window_matrices[0].matrix.rows[0].cursor_col, Some(12));
}

#[test]
fn phys_cursor_on_hidden_prefix_resolves_to_first_visible_glyph() {
    // Regression for the Doom `gg`-to-title "two cursors on the heading" bug.
    //
    // An org heading/title collapses its leading markup (`#+title: ` or the
    // leading stars) with the `invisible` property, so point at the line start
    // (charpos 0) has NO Text glyph -- the first visible glyph ('D') carries a
    // later charpos. The cursor must resolve to that first visible glyph (GNU's
    // set_cursor_from_row places the cursor on the glyph that follows the hidden
    // run), NOT fall back to the captured column 0, which would snap it onto the
    // line-number gutter and draw a stray second cursor.
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    // Three-column line-number gutter (cols 0, 1, 2).
    write_left_margin_char_to_current_row(&mut builder, '1', 1);
    write_left_margin_char_to_current_row(&mut builder, ' ', 1);
    write_left_margin_stretch_to_current_row(&mut builder, 1, 1);
    // Visible title text "Doom" begins at charpos 9: charpos 0..=8 is the hidden
    // "#+title: " prefix that produced no glyphs. 'D' is at materialize col 3.
    write_char_to_current_row(&mut builder, 'D', 0, 9);
    write_char_to_current_row(&mut builder, 'o', 0, 10);
    write_char_to_current_row(&mut builder, 'o', 0, 11);
    write_char_to_current_row(&mut builder, 'm', 0, 12);
    builder.end_row();

    // Point at the hidden line start (charpos 0); the capture passes column 0.
    builder.set_phys_cursor(PhysCursor {
        window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
        charpos: 0,
        row: 0,
        col: 0,
        slot_id: DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
            row: 0,
            col: 0,
        },
        x: 0.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: neomacs_display_protocol::types::Color::WHITE,
        cursor_fg: neomacs_display_protocol::types::Color::BLACK,
    });
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let cursor = state.phys_cursor.as_ref().expect("phys cursor");

    // The first visible glyph 'D' sits at materialize col 3 (after the 3-col
    // gutter); the cursor must land there, never on the gutter (col 0).
    assert_eq!(
        cursor.slot_id.col, 3,
        "cursor on hidden-prefix point must resolve to the first visible glyph, not the gutter"
    );

    let buf = state.materialize();
    let slot = buf
        .slot_glyph(cursor.slot_id)
        .expect("cursor slot glyph must be present");
    match slot {
        neomacs_display_protocol::frame_glyphs::FrameGlyph::Char { char, .. } => {
            assert_eq!(
                *char, 'D',
                "cursor must resolve to the first visible title glyph"
            );
        }
        other => panic!("expected a Char glyph at the cursor slot, got {other:?}"),
    }
}

#[test]
fn set_phys_cursor_leaves_window_cursors_untouched() {
    // The "two cursors on a line-numbered / heading line" bug is now prevented
    // at the source: the engine no longer pushes a redundant per-window cursor
    // for the selected window (push_cursor is guarded on !params.selected), so
    // set_phys_cursor has nothing to sync and must not reach into the
    // window-cursor list at all -- it only resolves and stores the phys cursor.
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_left_margin_char_to_current_row(&mut builder, '1', 1);
    write_left_margin_char_to_current_row(&mut builder, '2', 1);
    write_left_margin_stretch_to_current_row(&mut builder, 1, 1); // three-column line-number gutter
    write_char_to_current_row(&mut builder, 'H', 0, 100);
    write_char_to_current_row(&mut builder, 'e', 0, 101);
    write_char_to_current_row(&mut builder, 'l', 0, 102);
    builder.end_row();

    // The captured (pre-resolution) slot is the Text-area index, column 2.
    let captured_slot = DisplaySlotId {
        window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
        row: 0,
        col: 2,
    };
    builder.add_output_cursor(
        1,
        captured_slot,
        0.0,
        0.0,
        8.0,
        16.0,
        CursorStyle::FilledBox,
        neomacs_display_protocol::types::Color::WHITE,
    );
    builder.set_phys_cursor(PhysCursor {
        window_id: neomacs_display_protocol::types::DisplayWindowId::new(1),
        charpos: 102,
        row: 0,
        col: 2,
        slot_id: captured_slot,
        x: 0.0,
        y: 0.0,
        width: 8.0,
        height: 16.0,
        ascent: 12.0,
        style: CursorStyle::FilledBox,
        color: neomacs_display_protocol::types::Color::WHITE,
        cursor_fg: neomacs_display_protocol::types::Color::BLACK,
    });
    builder.end_window();

    let state = builder.finish(80, 3, 8.0, 16.0);
    let phys = state.phys_cursor.as_ref().expect("phys cursor");
    // 3 gutter columns + Text index 2 = materialize column 5.
    assert_eq!(phys.slot_id.col, 5, "phys cursor resolves past the gutter");

    let wc = state
        .cursors
        .iter()
        .find(|c| c.window_id.get() == 1)
        .expect("the manually pushed window cursor");
    assert_eq!(
        wc.slot_id, captured_slot,
        "set_phys_cursor must leave a separately-pushed window cursor exactly as \
         pushed -- it no longer syncs a redundant duplicate (there is none)"
    );
}

#[test]
fn resolve_cursor_visual_col_is_the_single_resolution_authority() {
    // Direct contract for the request that phys cursor installation resolves
    // through, so display column policy stays outside the builder setter.
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_left_margin_char_to_current_row(&mut builder, '1', 1);
    write_left_margin_char_to_current_row(&mut builder, '2', 1);
    write_left_margin_stretch_to_current_row(&mut builder, 1, 1); // three-column line-number gutter
    write_char_to_current_row(&mut builder, 'H', 0, 100);
    write_char_to_current_row(&mut builder, 'e', 0, 101);
    write_char_to_current_row(&mut builder, 'l', 0, 102);
    builder.end_row();

    // Matching window/row, point on 'l': 3 gutter columns + Text index 2 = 5.
    assert_eq!(
        CursorVisualColumnResolutionRequest::new(1, 0, 102)
            .resolve(builder.cursor_visual_column_context()),
        Some(5)
    );
    // Point on the first buffer glyph lands just past the gutter at column 3.
    assert_eq!(
        CursorVisualColumnResolutionRequest::new(1, 0, 100)
            .resolve(builder.cursor_visual_column_context()),
        Some(3)
    );
    // Point on hidden text (charpos 101 has a glyph here, but charpos 50 does
    // not) resolves to the first following visible glyph, never the captured
    // column: smallest charpos > 50 is 'H' at gutter column 3.
    assert_eq!(
        CursorVisualColumnResolutionRequest::new(1, 0, 50)
            .resolve(builder.cursor_visual_column_context()),
        Some(3)
    );
    // A cursor reported against another window has no slot here -> decline.
    assert_eq!(
        CursorVisualColumnResolutionRequest::new(2, 0, 102)
            .resolve(builder.cursor_visual_column_context()),
        None
    );
    // An out-of-range row also declines rather than inventing a column.
    assert_eq!(
        CursorVisualColumnResolutionRequest::new(1, 99, 102)
            .resolve(builder.cursor_visual_column_context()),
        None
    );
    builder.end_window();
}

#[test]
fn resolve_cursor_on_blank_gutter_line_lands_past_the_gutter() {
    // Regression for the "cursor in the line-number column on the blank line
    // after an org #+title" bug. That row carries only line-number gutter
    // glyphs and an empty Text area, so point on it matches no Text glyph. The
    // resolver must still place the cursor at the first column past the gutter,
    // never the captured Text-index 0 (which materialize maps into the gutter),
    // matching GNU set_cursor_from_row placing the cursor in the empty area
    // after a row's text.
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 80, Rect::new(0.0, 0.0, 640.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_left_margin_stretch_to_current_row(&mut builder, 2, 1); // leading "  " before the digit
    write_left_margin_char_to_current_row(&mut builder, '2', 1);
    write_left_margin_stretch_to_current_row(&mut builder, 1, 1); // trailing one-column pad
    // No push_char: the Text area is empty, as on a blank buffer line.
    builder.end_row();

    // The 4-column gutter (2 + 1 + 1) is fully walked; with no Text glyph the
    // cursor lands at column 4, the first cell of the empty text area.
    assert_eq!(
        CursorVisualColumnResolutionRequest::new(1, 0, 34)
            .resolve(builder.cursor_visual_column_context()),
        Some(4)
    );

    // A non-empty row whose point is past the last glyph (end of line) lands in
    // the same end-of-row cell rather than reverting to None.
    builder.begin_row(1, GlyphRowRole::Text);
    write_left_margin_stretch_to_current_row(&mut builder, 2, 1);
    write_left_margin_char_to_current_row(&mut builder, '3', 1);
    write_left_margin_stretch_to_current_row(&mut builder, 1, 1);
    write_char_to_current_row(&mut builder, 'H', 0, 40);
    write_char_to_current_row(&mut builder, 'i', 0, 41);
    builder.end_row();
    // Point at charpos 99 is past 'H'(40) and 'i'(41): 4 gutter + 2 text = col 6.
    assert_eq!(
        CursorVisualColumnResolutionRequest::new(1, 1, 99)
            .resolve(builder.cursor_visual_column_context()),
        Some(6)
    );
    builder.end_window();
}

#[test]
fn builder_reorders_status_line_rtl_row() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(0.0, 0.0, 80.0, 32.0), true);

    let row = external_text_row(
        GlyphRowRole::ModeLine,
        vec![Glyph::char('א', 5, 0), Glyph::char('ב', 5, 1)],
    );
    builder.install_display_row(1, &row);
    builder.end_window();

    let state = builder.finish(10, 2, 8.0, 16.0);
    let glyphs = &state.window_matrices[0].matrix.rows[1].glyphs[GlyphArea::Text as usize];
    assert_eq!(glyphs.len(), 2);
    assert_eq!(glyphs[0].glyph_type, GlyphType::Char { ch: 'ב' });
    assert_eq!(glyphs[1].glyph_type, GlyphType::Char { ch: 'א' });
    assert_eq!(glyphs[0].bidi_level, 1);
    assert_eq!(glyphs[1].bidi_level, 1);
}

/// Cross-row-kind RTL bidi parity (Slice 4 characterization; guards Slice 5).
///
/// A buffer `Text` row (incremental path — reorders at `end_current_row`) and a
/// `ModeLine` chrome row (copied row path — reorders at `end_current_row`)
/// built from the SAME Hebrew string must reorder to the SAME visual glyph
/// order, per-glyph `bidi_level`, and `reversed_p`. Today the two paths run the
/// same `reorder_row_bidi` finalizer, so these axes are identical.
///
/// The cursor column is the ONE axis that legitimately differs (the Text path
/// threads `phys_cursor`; the copied chrome path passes `None`), so it is
/// deliberately NOT asserted here.
#[test]
fn rtl_text_and_chrome_rows_reorder_identically() {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(0.0, 0.0, 80.0, 32.0), true);

    // Row 0 — buffer Text row via the incremental path (`end_current_row` reorder).
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'א', 0, 0);
    write_char_to_current_row(&mut builder, 'ב', 0, 1);
    builder.end_row();

    // Row 1 — ModeLine chrome row copied into the matrix row.
    let chrome = external_text_row(
        GlyphRowRole::ModeLine,
        vec![Glyph::char('א', 5, 0), Glyph::char('ב', 5, 1)],
    );
    builder.install_display_row(1, &chrome);
    builder.end_window();

    let state = builder.finish(10, 2, 8.0, 16.0);
    let rows = &state.window_matrices[0].matrix.rows;
    let text_glyphs = &rows[0].glyphs[GlyphArea::Text as usize];
    let chrome_glyphs = &rows[1].glyphs[GlyphArea::Text as usize];

    // The same Hebrew "אב" reorders to visual "בא" on BOTH row kinds.
    assert_eq!(text_glyphs.len(), 2);
    assert_eq!(chrome_glyphs.len(), 2);
    for glyphs in [text_glyphs, chrome_glyphs] {
        assert_eq!(glyphs[0].glyph_type, GlyphType::Char { ch: 'ב' });
        assert_eq!(glyphs[1].glyph_type, GlyphType::Char { ch: 'א' });
        assert_eq!(glyphs[0].bidi_level, 1);
        assert_eq!(glyphs[1].bidi_level, 1);
    }
    // Element-for-element parity between the two row kinds.
    assert_eq!(text_glyphs[0].glyph_type, chrome_glyphs[0].glyph_type);
    assert_eq!(text_glyphs[1].glyph_type, chrome_glyphs[1].glyph_type);
    assert_eq!(text_glyphs[0].bidi_level, chrome_glyphs[0].bidi_level);
    assert_eq!(text_glyphs[1].bidi_level, chrome_glyphs[1].bidi_level);
    // Both rows are flagged reversed (RTL paragraph) — same row-level flag.
    assert!(rows[0].reversed_p);
    assert!(rows[1].reversed_p);
}

/// Regression test for the face-id-collision bug that caused
/// both mode lines to render with mode-line-inactive colors
/// after `C-x 2`.
///
/// Reproduction: insert TWO faces into the builder's shared
/// `faces` HashMap at the SAME face_id. The first insertion (the
/// active mode-line) is overwritten by the second (the inactive
/// mode-line), and both window matrices that reference the id
/// then read the inactive colors.
///
/// Fix: `LayoutEngine::frame_face_id_counter` is frame-scoped
/// and NEVER resets per window, so sibling windows always get
/// distinct face ids and their entries never overwrite each
/// other. Mirrors GNU's single `face_cache->used` counter per
/// frame at `src/xfaces.c::init_frame_faces` / `realize_face`.
///
/// This test verifies the invariant at the builder level: when
/// the caller inserts two DIFFERENT faces at DIFFERENT ids, both
/// faces remain readable in the finished frame state. That is
/// the contract the face-id counter fix guarantees; without the
/// fix, the caller accidentally uses the SAME id and the second
/// insert wipes out the first.
#[test]
fn builder_preserves_distinct_mode_line_faces_across_sibling_windows() {
    use neomacs_display_protocol::face::Face;
    use neomacs_display_protocol::types::Color;

    let mut builder = DisplayOutputBuilder::new();

    // Emulate the post-C-x-2 redisplay order: active mode-line
    // for the TOP (selected) window, then inactive mode-line for
    // the BOTTOM (non-selected) window. The `LayoutEngine`'s
    // `frame_face_id_counter` guarantees these receive DIFFERENT
    // ids; the builder must keep both in the `faces` HashMap.
    let mut active = Face::new(10);
    active.foreground = Color::rgb(0.0, 0.0, 0.0);
    active.background = Color::rgb(0.75, 0.75, 0.75);
    builder.install_output_face(10, active.clone());

    let mut inactive = Face::new(11);
    inactive.foreground = Color::rgb(0.8, 0.8, 0.8);
    inactive.background = Color::rgb(0.30, 0.30, 0.30);
    builder.install_output_face(11, inactive.clone());

    // Window 1 (top, selected): references the active face on
    // its mode-line row.
    builder.begin_window(1, 12, 80, Rect::new(0.0, 0.0, 640.0, 192.0), true);
    builder.begin_row(11, GlyphRowRole::ModeLine);
    write_char_to_current_row(&mut builder, '-', 10, 0);
    builder.end_row();
    builder.end_window();

    // Window 3 (bottom, not selected): references the inactive
    // face on its mode-line row. Before the fix, the engine
    // re-used face_id 10 here because `current_face_id` was a
    // per-window `let` binding that reset to 1 for every window.
    builder.begin_window(3, 12, 80, Rect::new(0.0, 192.0, 640.0, 192.0), false);
    builder.begin_row(11, GlyphRowRole::ModeLine);
    write_char_to_current_row(&mut builder, '-', 11, 0);
    builder.end_row();
    builder.end_window();

    let state = builder.finish(80, 25, 8.0, 16.0);

    // Both faces must survive into the finished frame state. If
    // they were inserted at the same id, one would clobber the
    // other and this assertion would fail.
    let stored_active = state
        .faces
        .get(&10)
        .expect("face id 10 (active mode-line) must remain in the faces map");
    let stored_inactive = state
        .faces
        .get(&11)
        .expect("face id 11 (inactive mode-line) must remain in the faces map");

    assert_eq!(
        stored_active.background, active.background,
        "active mode-line background must not be overwritten by sibling window's face insertion"
    );
    assert_eq!(
        stored_inactive.background, inactive.background,
        "inactive mode-line background must remain distinct"
    );
    assert_ne!(
        stored_active.background, stored_inactive.background,
        "sibling mode lines must have different background colors"
    );
}

// --- Grapheme-cluster composition (Phase 1: emoji ZWJ + flag pairs) ---

fn cluster_builder() -> DisplayOutputBuilder {
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(1, 3, 40, Rect::new(0.0, 0.0, 320.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    builder
}

fn finish_text_area(builder: DisplayOutputBuilder) -> Vec<Glyph> {
    let mut builder = builder;
    builder.end_row();
    builder.end_window();
    let state = builder.finish(40, 3, 8.0, 16.0);
    state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::Text as usize].clone()
}

fn current_cluster_tail(builder: &DisplayOutputBuilder) -> Option<(char, bool)> {
    builder
        .current_row_for_test()
        .and_then(crate::composition::last_text_cluster_tail_in_row)
}

#[test]
fn cluster_tail_is_none_at_row_start() {
    let builder = cluster_builder();
    assert_eq!(current_cluster_tail(&builder), None);
}

#[test]
fn cluster_continuation_merges_combining_mark() {
    let mut builder = cluster_builder();
    write_char_to_current_row(&mut builder, 'e', 0, 0);
    // Combining acute accent (U+0301) is a cluster extender.
    write_cluster_continuation_to_current_row(&mut builder, '\u{0301}', 0, 1);
    let area = finish_text_area(builder);
    assert_eq!(
        area[0].glyph_type,
        GlyphType::Composite {
            text: "e\u{0301}".into()
        }
    );
    assert_eq!(area.iter().filter(|g| !g.padding).count(), 1);
}

#[test]
fn cluster_continuation_merges_zwj_emoji_sequence() {
    let mut builder = cluster_builder();
    // 👨 base, then ZWJ 👩 ZWJ 👧 as continuations (family emoji).
    write_wide_char_to_current_row(&mut builder, '\u{1F468}', 0, 0);
    for (i, ch) in "\u{200D}\u{1F469}\u{200D}\u{1F467}".chars().enumerate() {
        write_cluster_continuation_to_current_row(&mut builder, ch, 0, i + 1);
    }
    let area = finish_text_area(builder);
    assert_eq!(
        area[0].glyph_type,
        GlyphType::Composite {
            text: "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}".into()
        }
    );
    // One cluster cell + its wide padding; nothing else.
    assert_eq!(area.iter().filter(|g| !g.padding).count(), 1);
    assert!(area[1].padding);
}

#[test]
fn cluster_tail_detects_lone_regional_indicator_then_pairs_flag() {
    let mut builder = cluster_builder();
    // Regional indicators J (U+1F1EF) + P (U+1F1F5) => 🇯🇵 flag.
    write_wide_char_to_current_row(&mut builder, '\u{1F1EF}', 0, 0);
    assert_eq!(current_cluster_tail(&builder), Some(('\u{1F1EF}', true)));
    write_cluster_continuation_to_current_row(&mut builder, '\u{1F1F5}', 0, 1);
    // After pairing, the tail is a Composite — no longer a lone RI, so a
    // third regional indicator would start a fresh flag.
    assert_eq!(current_cluster_tail(&builder), Some(('\u{1F1F5}', false)));
    let area = finish_text_area(builder);
    assert_eq!(
        area[0].glyph_type,
        GlyphType::Composite {
            text: "\u{1F1EF}\u{1F1F5}".into()
        }
    );
    assert_eq!(area.iter().filter(|g| !g.padding).count(), 1);
}

#[test]
fn cluster_continuation_without_base_falls_back_to_standalone() {
    let mut builder = cluster_builder();
    // Stray ZWJ at row start: no base to merge into.
    write_cluster_continuation_to_current_row(&mut builder, '\u{200D}', 0, 0);
    let area = finish_text_area(builder);
    assert_eq!(area[0].glyph_type, GlyphType::Char { ch: '\u{200D}' });
}

// --- complex-script run grouping (Phase 3) ---

#[test]
fn engine_style_loop_clusters_zwj_family_emoji() {
    use neomacs_display_protocol::glyph_matrix::GlyphType;
    // Replicate the engine's per-char decision loop (engine.rs ~5565): for each
    // char, derive is_cluster_continuation from continues_cluster(ch, tail) and
    // dispatch to push_cluster_continuation vs push_wide_char/push_char exactly
    // as the buffer-text walk does. The family is 👨 ZWJ 👩 ZWJ 👧.
    let mut builder = cluster_builder();
    let seq = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    for (i, ch) in seq.chars().enumerate() {
        let tail = current_cluster_tail(&builder);
        let is_cont = crate::composition::continues_cluster(ch, tail);
        if is_cont {
            write_cluster_continuation_to_current_row(&mut builder, ch, 0, i);
        } else if crate::composition::base_width_cols(ch) == 2 {
            write_wide_char_to_current_row(&mut builder, ch, 0, i);
        } else {
            write_char_to_current_row(&mut builder, ch, 0, i);
        }
    }
    let area = finish_text_area(builder);
    let composites: Vec<&str> = area
        .iter()
        .filter_map(|g| match &g.glyph_type {
            GlyphType::Composite { text } => Some(text.as_ref()),
            _ => None,
        })
        .collect();
    // The whole sequence must collapse into ONE composed cell so the renderer
    // shapes it as a unit (HarfBuzz forms the family ligature).
    assert_eq!(
        composites,
        vec![seq],
        "ZWJ family did not cluster; row = {:?}",
        area.iter()
            .map(|g| (g.glyph_type.clone(), g.padding))
            .collect::<Vec<_>>()
    );
}

#[test]
fn complex_run_grows_into_one_composite_with_per_char_padding() {
    let mut builder = cluster_builder();
    // Arabic run: ا (U+0627) ل (U+0644) م (U+0645) => "الم".
    write_char_to_current_row(&mut builder, '\u{0627}', 0, 10);
    write_run_member_to_current_row(&mut builder, '\u{0644}', 0, 11, 8.0);
    write_run_member_to_current_row(&mut builder, '\u{0645}', 0, 12, 8.0);
    let area = finish_text_area(builder);
    // One composed cell holding the whole run + 2 padding cells = 3 columns,
    // so the renderer shapes (joins) the run as a unit.
    assert_eq!(area.len(), 3);
    assert_eq!(
        area[0].glyph_type,
        GlyphType::Composite {
            text: "\u{0627}\u{0644}\u{0645}".into()
        }
    );
    assert!(!area[0].padding);
    assert!(area[1].padding && area[2].padding);
    // Per-letter cursor: each column carries its own buffer position.
    assert_eq!(area[0].charpos, 10);
    assert_eq!(area[1].charpos, 11);
    assert_eq!(area[2].charpos, 12);
}

#[test]
fn lone_rtl_run_stays_in_place_in_ltr_paragraph() {
    use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphType};
    // Build the Text row exactly as the engine does for
    // "Mixed: hello العربية world": ASCII as Char glyphs, the Arabic word as
    // ONE Composite{full run text} followed by one padding cell per extra char.
    // The paragraph's first strong char is L, so the lone Arabic run is a single
    // odd-level bidi unit among even-level units: UAX#9 L2 must leave it where it
    // is (mid-line), not push it to the paragraph edge.
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    let mut cp = 0usize;
    {
        let text = &mut row.glyphs[GlyphArea::Text.index()];
        for ch in "Mixed: hello ".chars() {
            text.push(Glyph::char(ch, 0, cp));
            cp += 1;
        }
        let word = "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064a}\u{0629}";
        text.push(Glyph {
            glyph_type: GlyphType::Composite { text: word.into() },
            face_id: 0,
            charpos: cp,
            bidi_level: 0,
            wide: false,
            pixel_width: 0.0,
            pixel_height: 0.0,
            pixel_ascent: 0.0,
            vertical_offset_px: 0.0,
            padding: false,
        });
        cp += 1;
        for _ in word.chars().skip(1) {
            text.push(Glyph::padding_for(0, cp));
            cp += 1;
        }
        for ch in " world".chars() {
            text.push(Glyph::char(ch, 0, cp));
            cp += 1;
        }
    }
    let before = render_row_text(&row);
    crate::glyph_row_writer::reorder_row_bidi(&mut row, None);
    let after = render_row_text(&row);
    // First strong char is L, so this is not a reversed (right-aligned) row.
    assert!(!row.reversed_p);
    // Reorder is a no-op here: the Arabic word keeps its mid-line slot, between
    // "hello " and " world". (When this rendered wrong it was a draw-side bug,
    // not reordering.)
    assert_eq!(before, "Mixed: hello {العربية}...... world");
    assert_eq!(after, before);
}

#[test]
fn rtl_paragraph_row_is_marked_reversed() {
    use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphType};
    // A pure right-to-left line: one Arabic word as a composed cell. The
    // paragraph's first strong char is R, so the row must be flagged reversed
    // (GNU reversed_p) so materialization aligns it flush-right.
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    {
        let text = &mut row.glyphs[GlyphArea::Text.index()];
        text.push(Glyph {
            glyph_type: GlyphType::Composite {
                text: "\u{0627}\u{0644}\u{0645}".into(),
            },
            face_id: 0,
            charpos: 0,
            bidi_level: 0,
            wide: false,
            pixel_width: 40.0,
            pixel_height: 0.0,
            pixel_ascent: 0.0,
            vertical_offset_px: 0.0,
            padding: false,
        });
        text.push(Glyph::padding_for(0, 1));
        text.push(Glyph::padding_for(0, 2));
    }
    crate::glyph_row_writer::reorder_row_bidi(&mut row, None);
    assert!(row.reversed_p);
    // The reorder itself does not insert filler glyphs; the right-edge offset
    // happens at materialization time.
    assert_eq!(row.glyphs[GlyphArea::Text.index()].len(), 3);
}

#[test]
fn ltr_paragraph_row_is_not_marked_reversed() {
    use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea};
    // First strong char is L: not a reversed row.
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    {
        let text = &mut row.glyphs[GlyphArea::Text.index()];
        for ch in "hello".chars() {
            text.push(Glyph::char(ch, 0, 0).with_pixel_width(16.0));
        }
    }
    crate::glyph_row_writer::reorder_row_bidi(&mut row, None);
    assert!(!row.reversed_p);
}

#[cfg(test)]
fn render_row_text(row: &GlyphRow) -> String {
    use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphType};
    let mut s = String::new();
    for g in &row.glyphs[GlyphArea::Text.index()] {
        if g.padding {
            s.push('.');
            continue;
        }
        match &g.glyph_type {
            GlyphType::Char { ch } => s.push(*ch),
            GlyphType::Composite { text } => {
                s.push('{');
                s.push_str(text);
                s.push('}');
            }
            _ => s.push('?'),
        }
    }
    s
}
