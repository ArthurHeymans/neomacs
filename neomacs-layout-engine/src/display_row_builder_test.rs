use super::*;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayLength, DisplaySourceMappedText,
    DisplaySourcePosition, DisplayStretch, DisplayStretchWidth, DisplayTextRun, GlyphlessMethod,
    RenderFaceRef, SourceSpan,
};
use crate::display_source::{DisplayItemSource, DisplaySourceContext, LispStringSourceCursor};
use crate::matrix_builder::GlyphMatrixBuilder;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphType};
use neomacs_display_protocol::types::Rect;
use neovm_core::emacs_core::{Context, Value};

fn layout() -> DisplayRowLayout {
    DisplayRowLayout {
        role: GlyphRowRole::Text,
        y_px: 0.0,
        width_px: 240.0,
        height_px: 16.0,
        ascent_px: 12.0,
        char_width_px: 8.0,
        tab_policy: DisplayTabPolicy::every(4),
        base_face: RenderFaceRef::FaceId(1),
        symbol_values: std::collections::HashMap::new(),
    }
}

fn text_item(text: &str) -> DisplayItem {
    DisplayItem::new(
        SourceSpan::new(
            DisplaySourcePosition::lisp_string(1, 0, 0),
            DisplaySourcePosition::lisp_string(1, text.chars().count(), text.len()),
        ),
        RenderFaceRef::FaceId(2),
        DisplayItemKind::TextRun(DisplayTextRun::new(text)),
    )
}

fn glyphless_item(ch: char, method: GlyphlessMethod) -> DisplayItem {
    DisplayItem::new(
        SourceSpan::new(
            DisplaySourcePosition::lisp_string(1, 0, 0),
            DisplaySourcePosition::lisp_string(1, 1, ch.len_utf8()),
        ),
        RenderFaceRef::FaceId(2),
        DisplayItemKind::Glyphless(DisplayGlyphless { ch, method }),
    )
}

fn control_item(ch: char) -> DisplayItem {
    DisplayItem::new(
        SourceSpan::new(
            DisplaySourcePosition::lisp_string(1, 0, 0),
            DisplaySourcePosition::lisp_string(1, 1, ch.len_utf8()),
        ),
        RenderFaceRef::FaceId(2),
        DisplayItemKind::ControlChar { ch },
    )
}

fn mapped_text_item(text: &str) -> DisplayItem {
    DisplayItem::new(
        SourceSpan::new(
            DisplaySourcePosition::lisp_string(1, 0, 0),
            DisplaySourcePosition::lisp_string(1, 1, text.len()),
        ),
        RenderFaceRef::FaceId(2),
        DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(text)),
    )
}

fn stretch_item(width: DisplayLength) -> DisplayItem {
    DisplayItem::new(
        SourceSpan::synthetic(1, 0, 1),
        RenderFaceRef::FaceId(2),
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(width),
            height: None,
            ascent: None,
        }),
    )
}

fn row_text(row: &neomacs_display_protocol::glyph_matrix::GlyphRow) -> String {
    let mut text = String::new();
    for glyph in row.glyphs[GlyphArea::Text.index()]
        .iter()
        .filter(|glyph| !glyph.padding)
    {
        match &glyph.glyph_type {
            GlyphType::Char { ch } | GlyphType::Glyphless { ch } => text.push(*ch),
            GlyphType::Composite { text: cluster } => text.push_str(cluster),
            GlyphType::Stretch { width_cols } => {
                text.push_str(&" ".repeat(usize::from(*width_cols)))
            }
            GlyphType::Image { .. } => {}
        }
    }
    text
}

#[test]
fn display_row_progress_writer_skips_zero_width_glyphless_item() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 16.0, col: 2 },
        80.0,
    );

    let progress = writer.push_item(glyphless_item('\u{200b}', GlyphlessMethod::ZeroWidth));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert!(progress.slots.is_empty());
    assert!(row.glyphs[GlyphArea::Text.index()].is_empty());
}

#[test]
fn display_row_progress_writer_uses_empty_box_glyphless_width() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        80.0,
    );

    let progress = writer.push_item(glyphless_item('\u{fffc}', GlyphlessMethod::EmptyBox));

    assert_eq!(progress.end, DisplayRowPosition { x_px: 8.0, col: 1 });
    let glyph = &row.glyphs[GlyphArea::Text.index()][0];
    assert_eq!(glyph.glyph_type, GlyphType::Glyphless { ch: '\u{fffc}' });
    assert_eq!(glyph.pixel_width, 8.0);
}

#[test]
fn display_row_progress_writer_uses_hex_code_glyphless_width() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        80.0,
    );

    let progress = writer.push_item(glyphless_item('\u{fff0}', GlyphlessMethod::HexCode));

    assert_eq!(progress.end, DisplayRowPosition { x_px: 48.0, col: 6 });
    assert_eq!(progress.slots[0].width_px, 48.0);
    assert_eq!(progress.slots[0].width_cols, 6);
    let glyph = &row.glyphs[GlyphArea::Text.index()][0];
    assert_eq!(glyph.glyph_type, GlyphType::Glyphless { ch: '\u{fff0}' });
    assert_eq!(glyph.pixel_width, 48.0);
}

#[test]
fn display_row_progress_writer_uses_thin_space_glyphless_width() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        80.0,
    );

    let progress = writer.push_item(glyphless_item('\u{2009}', GlyphlessMethod::ThinSpace));

    assert_eq!(progress.end, DisplayRowPosition { x_px: 2.0, col: 1 });
    let glyph = &row.glyphs[GlyphArea::Text.index()][0];
    assert_eq!(glyph.glyph_type, GlyphType::Glyphless { ch: '\u{2009}' });
    assert_eq!(glyph.pixel_width, 2.0);
}

#[test]
fn display_row_progress_writer_clips_glyphless_before_row_mutation() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 40.0, col: 5 },
        80.0,
    );

    let progress = writer.push_item(glyphless_item('\u{fff0}', GlyphlessMethod::HexCode));

    assert_eq!(progress.status, DisplayRowAppendStatus::Clipped);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 40.0, col: 5 });
    assert!(progress.slots.is_empty());
    assert!(row.glyphs[GlyphArea::Text.index()].is_empty());
    assert!(!row.displays_text);
}

#[test]
fn display_row_progress_writer_clips_stretch_before_row_mutation() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 64.0, col: 8 },
        80.0,
    );

    let progress = writer.push_item(stretch_item(DisplayLength::Pixels(24.0)));

    assert_eq!(progress.status, DisplayRowAppendStatus::Clipped);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 64.0, col: 8 });
    assert!(progress.slots.is_empty());
    assert!(row.glyphs[GlyphArea::Text.index()].is_empty());
    assert!(!row.displays_text);
}

#[test]
fn display_row_builder_renders_control_char_as_caret_notation() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(control_item('\u{0001}'));

    let row = builder.finish();
    let glyphs = &row.glyphs[GlyphArea::Text.index()];

    assert_eq!(row_text(&row), "^A");
    assert_eq!(glyphs.len(), 2);
    assert!(glyphs.iter().all(|glyph| glyph.charpos == 0));
}

#[test]
fn display_row_builder_renders_delete_control_char_as_caret_question() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(control_item('\u{007f}'));

    let row = builder.finish();

    assert_eq!(row_text(&row), "^?");
}

#[test]
fn display_row_progress_writer_reports_control_char_as_single_source_slot() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 8.0, col: 1 },
        80.0,
    );

    let progress = writer.push_item(control_item('\u{0001}'));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 24.0, col: 3 });
    assert_eq!(progress.slots.len(), 1);
    assert_eq!(progress.slots[0].width_px, 16.0);
    assert_eq!(progress.slots[0].width_cols, 2);
    assert_eq!(row_text(&row), "^A");
}

#[test]
fn append_display_item_to_current_matrix_row_returns_progress_and_updates_row() {
    let row_layout = layout();
    let mut matrix = GlyphMatrixBuilder::new();
    matrix.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    matrix.begin_row(0, GlyphRowRole::Text);

    let progress = append_display_item_to_current_matrix_row(
        &mut matrix,
        &row_layout,
        text_item("ab"),
        DisplayRowPosition { x_px: 8.0, col: 1 },
        80.0,
    )
    .expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(progress.end, DisplayRowPosition { x_px: 24.0, col: 3 });
    assert_eq!(progress.slots.len(), 2);
    matrix
        .with_current_row_mut(|row| {
            assert_eq!(row_text(row), "ab");
        })
        .expect("current row");
}

#[test]
fn append_measured_display_item_to_current_matrix_row_uses_glyph_measurer() {
    struct TestMeasurer;

    impl DisplayGlyphMeasurer for TestMeasurer {
        fn glyph_advance_px(
            &mut self,
            ch: char,
            _face_id: u32,
            _columns: u8,
            _fallback_advance_px: f32,
        ) -> Option<f32> {
            match ch {
                'm' => Some(12.0),
                'i' => Some(4.0),
                _ => None,
            }
        }
    }

    let row_layout = layout();
    let mut matrix = GlyphMatrixBuilder::new();
    let mut measurer = TestMeasurer;
    matrix.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    matrix.begin_row(0, GlyphRowRole::Text);

    let progress = append_measured_display_item_to_current_matrix_row(
        &mut matrix,
        &row_layout,
        text_item("mi"),
        &mut measurer,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        80.0,
    )
    .expect("append progress");

    assert_eq!(progress.end, DisplayRowPosition { x_px: 16.0, col: 2 });
    matrix
        .with_current_row_mut(|row| {
            let glyphs = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(glyphs[0].pixel_width, 12.0);
            assert_eq!(glyphs[1].pixel_width, 4.0);
        })
        .expect("current row");
}

#[test]
fn display_row_append_cursor_updates_position_after_append() {
    let row_layout = layout();
    let mut matrix = GlyphMatrixBuilder::new();
    matrix.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    matrix.begin_row(0, GlyphRowRole::Text);

    let mut cursor = DisplayRowAppendCursor::new(DisplayRowPosition { x_px: 8.0, col: 1 }, 80.0);
    let progress = cursor
        .append_item_to_current_matrix_row(&mut matrix, &row_layout, text_item("ab"))
        .expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(progress.end, DisplayRowPosition { x_px: 24.0, col: 3 });
    assert_eq!(cursor.position(), DisplayRowPosition { x_px: 24.0, col: 3 });
    matrix
        .with_current_row_mut(|row| {
            assert_eq!(row_text(row), "ab");
        })
        .expect("current row");
}

#[test]
fn display_row_append_cursor_updates_position_to_clipped_end() {
    let row_layout = layout();
    let mut matrix = GlyphMatrixBuilder::new();
    matrix.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    matrix.begin_row(0, GlyphRowRole::Text);

    let mut cursor = DisplayRowAppendCursor::new(DisplayRowPosition { x_px: 8.0, col: 1 }, 16.0);
    let progress = cursor
        .append_item_to_current_matrix_row(&mut matrix, &row_layout, text_item("ab"))
        .expect("append progress");

    assert_eq!(progress.status, DisplayRowAppendStatus::Clipped);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(cursor.position(), DisplayRowPosition { x_px: 16.0, col: 2 });
    matrix
        .with_current_row_mut(|row| {
            assert_eq!(row_text(row), "a");
        })
        .expect("current row");
}

#[test]
fn display_row_append_cursor_uses_glyph_measurer() {
    let row_layout = layout();
    let mut matrix = GlyphMatrixBuilder::new();
    let mut measurer = FixedGlyphAdvances::new();
    measurer.insert('m', 2, 12.0);
    measurer.insert('i', 2, 4.0);
    matrix.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    matrix.begin_row(0, GlyphRowRole::Text);

    let mut cursor = DisplayRowAppendCursor::new(DisplayRowPosition { x_px: 0.0, col: 0 }, 80.0);
    let progress = cursor
        .append_measured_item_to_current_matrix_row(
            &mut matrix,
            &row_layout,
            text_item("mi"),
            &mut measurer,
        )
        .expect("append progress");

    assert_eq!(progress.end, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(cursor.position(), DisplayRowPosition { x_px: 16.0, col: 2 });
    matrix
        .with_current_row_mut(|row| {
            let glyphs = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(glyphs[0].pixel_width, 12.0);
            assert_eq!(glyphs[1].pixel_width, 4.0);
        })
        .expect("current row");
}

#[test]
fn display_row_append_cursor_appends_explicit_source_item() {
    let _eval = Context::new();
    let row_layout = layout();
    let mut matrix = GlyphMatrixBuilder::new();
    matrix.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    matrix.begin_row(0, GlyphRowRole::Text);
    let mut source = LispStringSourceCursor::new(1, Value::string("abc"), RenderFaceRef::FaceId(2))
        .expect("source");
    let mut context = DisplaySourceContext::empty();

    let mut cursor = DisplayRowAppendCursor::new(DisplayRowPosition { x_px: 8.0, col: 1 }, 80.0);
    let item = source.next_item(&mut context).expect("source item");
    let progress = cursor
        .append_item_to_current_matrix_row(&mut matrix, &row_layout, item)
        .expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(progress.end, DisplayRowPosition { x_px: 32.0, col: 4 });
    assert_eq!(cursor.position(), DisplayRowPosition { x_px: 32.0, col: 4 });
    matrix
        .with_current_row_mut(|row| {
            assert_eq!(row_text(row), "abc");
        })
        .expect("current row");
}

#[test]
fn display_row_append_cursor_appends_explicit_source_item_with_glyph_measurer() {
    let _eval = Context::new();
    let row_layout = layout();
    let mut matrix = GlyphMatrixBuilder::new();
    let mut measurer = FixedGlyphAdvances::new();
    measurer.insert('m', 2, 12.0);
    measurer.insert('i', 2, 4.0);
    matrix.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    matrix.begin_row(0, GlyphRowRole::Text);
    let mut source = LispStringSourceCursor::new(1, Value::string("mi"), RenderFaceRef::FaceId(2))
        .expect("source");
    let mut context = DisplaySourceContext::empty();

    let mut cursor = DisplayRowAppendCursor::new(DisplayRowPosition { x_px: 0.0, col: 0 }, 80.0);
    let item = source.next_item(&mut context).expect("source item");
    let progress = cursor
        .append_measured_item_to_current_matrix_row(&mut matrix, &row_layout, item, &mut measurer)
        .expect("append progress");

    assert_eq!(progress.end, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(cursor.position(), DisplayRowPosition { x_px: 16.0, col: 2 });
    matrix
        .with_current_row_mut(|row| {
            let glyphs = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(glyphs[0].pixel_width, 12.0);
            assert_eq!(glyphs[1].pixel_width, 4.0);
        })
        .expect("current row");
}

#[test]
fn fixed_glyph_advance_matches_only_configured_glyph() {
    let mut measurer = FixedGlyphAdvance::new('m', 7, 13.0);

    assert_eq!(measurer.glyph_advance_px('m', 7, 1, 8.0), Some(13.0));
    assert_eq!(measurer.glyph_advance_px('m', 8, 1, 8.0), None);
    assert_eq!(measurer.glyph_advance_px('i', 7, 1, 8.0), None);
}

#[test]
fn fixed_glyph_advances_return_inserted_widths() {
    let mut measurer = FixedGlyphAdvances::new();
    measurer.insert('m', 7, 13.0);
    measurer.insert('i', 7, 4.0);

    assert_eq!(measurer.glyph_advance_px('m', 7, 1, 8.0), Some(13.0));
    assert_eq!(measurer.glyph_advance_px('i', 7, 1, 8.0), Some(4.0));
    assert_eq!(measurer.glyph_advance_px('m', 8, 1, 8.0), None);
}

#[test]
fn display_text_run_measurement_exposes_measured_advances() {
    let advances = vec![
        DisplayTextRunAdvance::new(0, 0, 8.0),
        DisplayTextRunAdvance::new(1, 1, 9.0),
    ];
    let measured = DisplayTextRunMeasurement::Measured(advances.clone());

    assert_eq!(measured.measured_advances(), Some(advances.as_slice()));
    assert_eq!(DisplayTextRunMeasurement::PerChar.measured_advances(), None);
}

#[test]
fn display_text_run_measurement_builds_uniform_advances_for_text() {
    let measurement = DisplayTextRunMeasurement::uniform_for_text("aé中", 5.0);

    let advances = measurement
        .measured_advances()
        .expect("uniform measurement should produce advances");
    assert_eq!(
        advances
            .iter()
            .map(|advance| (advance.char_offset, advance.byte_offset, advance.advance_px))
            .collect::<Vec<_>>(),
        vec![(0, 0, 5.0), (1, 1, 5.0), (2, 3, 5.0)]
    );
}

#[test]
fn display_text_run_measurement_maps_base_char_byte_advances() {
    let measurement = DisplayTextRunMeasurement::Measured(vec![
        DisplayTextRunAdvance::new(0, 0, 7.0),
        DisplayTextRunAdvance::new(1, 1, 0.0),
        DisplayTextRunAdvance::new(2, 3, 9.0),
    ]);

    assert_eq!(
        measurement.base_char_byte_advances("a\u{301}中", 100),
        vec![(100, 7.0), (103, 9.0)]
    );
    assert_eq!(
        DisplayTextRunMeasurement::PerChar.base_char_byte_advances("a\u{301}", 100),
        Vec::<(usize, f32)>::new()
    );
}

#[test]
fn display_row_builder_renders_source_mapped_text_with_one_source_charpos() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(mapped_text_item("\\ "));

    let row = builder.finish();
    let glyphs = &row.glyphs[GlyphArea::Text.index()];

    assert_eq!(row_text(&row), "\\ ");
    assert_eq!(glyphs.len(), 2);
    assert!(glyphs.iter().all(|glyph| glyph.charpos == 0));
}

#[test]
fn display_row_progress_writer_reports_source_mapped_text_slots_with_same_source() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 8.0, col: 1 },
        80.0,
    );

    let progress = writer.push_item(mapped_text_item("\\-"));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 24.0, col: 3 });
    assert_eq!(progress.slots.len(), 2);
    assert!(
        progress
            .slots
            .iter()
            .all(|slot| slot.source == DisplaySourcePosition::lisp_string(1, 0, 0))
    );
    assert_eq!(progress.slots[0].width_px, 8.0);
    assert_eq!(progress.slots[0].width_cols, 1);
    assert_eq!(progress.slots[1].width_px, 8.0);
    assert_eq!(progress.slots[1].width_cols, 1);
    assert_eq!(row_text(&row), "\\-");
}

#[test]
fn display_row_builder_emits_ascii_text_items() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(text_item("abc"));

    let row = builder.finish();

    assert_eq!(row.role, GlyphRowRole::Text);
    assert_eq!(row_text(&row), "abc");
    assert_eq!(row.glyphs[GlyphArea::Text.index()].len(), 3);
    assert!(
        row.glyphs[GlyphArea::Text.index()]
            .iter()
            .all(|glyph| glyph.face_id == 2)
    );
}

#[test]
fn display_row_builder_consumes_display_item_source() {
    let _eval = Context::new();
    let mut source = LispStringSourceCursor::new(1, Value::string("abc"), RenderFaceRef::FaceId(2))
        .expect("source");
    let mut context = DisplaySourceContext::empty();
    let mut builder = DisplayRowBuilder::new(layout());

    builder.push_source(&mut source, &mut context);

    let row = builder.finish();
    assert_eq!(row_text(&row), "abc");
}

#[test]
fn display_row_builder_emits_tab_as_stretch_to_next_tab_stop() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(text_item("a\tb"));

    let row = builder.finish();
    let glyphs = &row.glyphs[GlyphArea::Text.index()];

    assert_eq!(row_text(&row), "a   b");
    assert_eq!(glyphs[1].glyph_type, GlyphType::Stretch { width_cols: 3 });
}

#[test]
fn display_row_writer_appends_items_to_existing_row_tab_context() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    row.enabled = true;
    crate::matrix_builder::GlyphMatrixBuilder::push_char_to_row(&mut row, 'x', 2, 0, 8.0);

    let row_layout = layout();
    let mut writer = DisplayRowWriter::new(&row_layout, &mut row);
    writer.push_item(text_item("a\tb"));

    let glyphs = &row.glyphs[GlyphArea::Text.index()];
    assert_eq!(row_text(&row), "xa  b");
    assert_eq!(glyphs[2].glyph_type, GlyphType::Stretch { width_cols: 2 });
}

#[test]
fn display_row_writer_consumes_display_item_source() {
    let _eval = Context::new();
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    row.enabled = true;
    crate::matrix_builder::GlyphMatrixBuilder::push_char_to_row(&mut row, 'x', 2, 0, 8.0);
    let mut source =
        LispStringSourceCursor::new(1, Value::string("a\tb"), RenderFaceRef::FaceId(2))
            .expect("source");
    let mut context = DisplaySourceContext::empty();

    let row_layout = layout();
    let mut writer = DisplayRowWriter::new(&row_layout, &mut row);
    writer.push_source(&mut source, &mut context);

    assert_eq!(row_text(&row), "xa  b");
}

#[test]
fn display_row_writer_reports_appended_metrics() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowWriter::new(&row_layout, &mut row);

    let metrics = writer.push_item(text_item("a\tb"));

    assert_eq!(metrics.width_cols, 5);
    assert_eq!(metrics.width_px, 40.0);
}

#[test]
fn display_row_progress_writer_stops_text_before_right_limit() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        20.0,
    );

    let progress = writer.push_item(text_item("abcd"));

    assert_eq!(progress.status, DisplayRowAppendStatus::Clipped);
    assert_eq!(progress.start, DisplayRowPosition { x_px: 0.0, col: 0 });
    assert_eq!(progress.end, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(writer.position(), DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(row_text(&row), "ab");
}

#[test]
fn display_row_progress_writer_reports_source_slots_for_text_run() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 4.0, col: 2 },
        80.0,
    );

    let progress = writer.push_item(text_item("aé"));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(progress.slots.len(), 2);
    assert_eq!(
        progress.slots[0].source,
        DisplaySourcePosition::lisp_string(1, 0, 0)
    );
    assert_eq!(progress.slots[0].x_px, 4.0);
    assert_eq!(progress.slots[0].col, 2);
    assert_eq!(progress.slots[0].width_px, 8.0);
    assert_eq!(
        progress.slots[1].source,
        DisplaySourcePosition::lisp_string(1, 1, 1)
    );
    assert_eq!(progress.slots[1].x_px, 12.0);
    assert_eq!(progress.slots[1].col, 3);
    assert_eq!(progress.slots[1].width_px, 8.0);
}

#[test]
fn display_row_progress_writer_uses_text_run_measurement_plan() {
    struct RunOnlyMeasurer;

    impl DisplayGlyphMeasurer for RunOnlyMeasurer {
        fn glyph_advance_px(
            &mut self,
            _ch: char,
            _face_id: u32,
            _columns: u8,
            _fallback_advance_px: f32,
        ) -> Option<f32> {
            panic!("text run should use the run measurement plan");
        }

        fn text_run_advances_px(
            &mut self,
            text: &str,
            face_id: u32,
            _fallback_char_width_px: f32,
        ) -> DisplayTextRunMeasurement {
            assert_eq!(text, "abc");
            assert_eq!(face_id, 2);
            DisplayTextRunMeasurement::Measured(vec![
                DisplayTextRunAdvance::new(0, 0, 4.0),
                DisplayTextRunAdvance::new(1, 1, 20.0),
                DisplayTextRunAdvance::new(2, 2, 6.0),
            ])
        }
    }

    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut measurer = RunOnlyMeasurer;
    let mut writer = DisplayRowProgressWriter::with_glyph_measurer(
        &row_layout,
        &mut row,
        &mut measurer,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        80.0,
    );

    let progress = writer.push_item(text_item("abc"));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 30.0, col: 3 });
    assert_eq!(
        progress
            .slots
            .iter()
            .map(|slot| slot.width_px)
            .collect::<Vec<_>>(),
        vec![4.0, 20.0, 6.0]
    );
    assert_eq!(
        row.glyphs[GlyphArea::Text.index()]
            .iter()
            .map(|glyph| glyph.pixel_width)
            .collect::<Vec<_>>(),
        vec![4.0, 20.0, 6.0]
    );
}

#[test]
fn display_row_progress_writer_accepts_direct_text_run_measurement_plan() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let measurement = DisplayTextRunMeasurement::Measured(vec![
        DisplayTextRunAdvance::new(0, 0, 4.0),
        DisplayTextRunAdvance::new(1, 1, 20.0),
        DisplayTextRunAdvance::new(2, 2, 6.0),
    ]);
    let mut writer = DisplayRowProgressWriter::with_text_run_measurement(
        &row_layout,
        &mut row,
        measurement,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        80.0,
    );

    let progress = writer.push_item(text_item("abc"));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 30.0, col: 3 });
    assert_eq!(
        row.glyphs[GlyphArea::Text.index()]
            .iter()
            .map(|glyph| glyph.pixel_width)
            .collect::<Vec<_>>(),
        vec![4.0, 20.0, 6.0]
    );
}

#[test]
fn display_row_progress_writer_uses_position_for_tabs() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let row_layout = layout();
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition { x_px: 16.0, col: 2 },
        80.0,
    );

    let progress = writer.push_item(text_item("\tb"));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(progress.end, DisplayRowPosition { x_px: 40.0, col: 5 });
    assert_eq!(
        row.glyphs[GlyphArea::Text.index()][0].glyph_type,
        GlyphType::Stretch { width_cols: 2 }
    );
}

#[test]
fn display_row_progress_writer_uses_tab_policy_origin_for_pixel_tabs() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let mut row_layout = layout();
    row_layout.tab_policy = DisplayTabPolicy::from_tab_width_and_stops(96.0, 8, &[]);
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition {
            x_px: 96.0 + 24.0,
            col: 3,
        },
        240.0,
    );

    let progress = writer.push_item(text_item("\tb"));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(
        row.glyphs[GlyphArea::Text.index()][0].glyph_type,
        GlyphType::Stretch { width_cols: 5 }
    );
    assert_eq!(progress.slots[0].x_px, 120.0);
    assert_eq!(progress.slots[0].width_px, 40.0);
    assert_eq!(
        progress.end,
        DisplayRowPosition {
            x_px: 168.0,
            col: 9
        }
    );
}

#[test]
fn display_row_progress_writer_uses_tab_policy_explicit_stops() {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Text);
    let mut row_layout = layout();
    row_layout.tab_policy = DisplayTabPolicy::from_tab_width_and_stops(100.0, 8, &[4, 10]);
    let mut writer = DisplayRowProgressWriter::new(
        &row_layout,
        &mut row,
        DisplayRowPosition {
            x_px: 100.0 + 24.0,
            col: 3,
        },
        240.0,
    );

    let progress = writer.push_item(text_item("\tb"));

    assert_eq!(progress.status, DisplayRowAppendStatus::Complete);
    assert_eq!(
        row.glyphs[GlyphArea::Text.index()][0].glyph_type,
        GlyphType::Stretch { width_cols: 1 }
    );
    assert_eq!(progress.slots[0].x_px, 124.0);
    assert_eq!(progress.slots[0].width_px, 8.0);
    assert_eq!(
        progress.end,
        DisplayRowPosition {
            x_px: 140.0,
            col: 5
        }
    );
}

#[test]
fn display_row_builder_uses_glyph_measurer_for_text_pixel_widths() {
    struct TestMeasurer;

    impl DisplayGlyphMeasurer for TestMeasurer {
        fn glyph_advance_px(
            &mut self,
            ch: char,
            _face_id: u32,
            _columns: u8,
            _fallback_advance_px: f32,
        ) -> Option<f32> {
            match ch {
                'm' => Some(12.0),
                'i' => Some(4.0),
                _ => None,
            }
        }
    }

    let mut measurer = TestMeasurer;
    let mut builder = DisplayRowBuilder::with_glyph_measurer(layout(), &mut measurer);
    builder.push_item(text_item("mi"));

    let row = builder.finish();
    let glyphs = &row.glyphs[GlyphArea::Text.index()];

    assert_eq!(glyphs[0].pixel_width, 12.0);
    assert_eq!(glyphs[1].pixel_width, 4.0);
}

#[test]
fn display_row_builder_push_measured_item_accepts_per_call_measurer() {
    struct TestMeasurer;

    impl DisplayGlyphMeasurer for TestMeasurer {
        fn glyph_advance_px(
            &mut self,
            ch: char,
            _face_id: u32,
            _columns: u8,
            _fallback_advance_px: f32,
        ) -> Option<f32> {
            (ch == '中').then_some(24.0)
        }
    }

    let mut builder = DisplayRowBuilder::new(layout());
    let mut measurer = TestMeasurer;

    builder.push_measured_item(text_item("A中"), &mut measurer);
    let row = builder.finish();
    let cjk = row.glyphs[GlyphArea::Text.index()]
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("CJK glyph");

    assert_eq!(row_text(&row), "A中");
    assert_eq!(cjk.pixel_width, 24.0);
}

#[test]
fn display_row_builder_emits_cjk_wide_char_with_padding() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(text_item("A中B"));

    let row = builder.finish();
    let glyphs = &row.glyphs[GlyphArea::Text.index()];

    assert_eq!(row_text(&row), "A中B");
    assert!(
        glyphs.iter().any(|glyph| {
            matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide
        })
    );
    assert!(glyphs.iter().any(|glyph| glyph.padding));
}

#[test]
fn display_row_builder_composes_emoji_zwj_cluster() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(text_item("👨‍👩"));

    let row = builder.finish();
    let glyphs = &row.glyphs[GlyphArea::Text.index()];

    assert!(glyphs
        .iter()
        .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))));
}

#[test]
fn display_row_builder_composes_combining_mark_cluster() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(text_item("e\u{301}"));

    let row = builder.finish();

    assert!(row.glyphs[GlyphArea::Text.index()]
        .iter()
        .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.as_ref() == "e\u{301}")));
}

#[test]
fn display_row_builder_groups_arabic_complex_run() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(text_item("سلام"));

    let row = builder.finish();
    let glyphs = &row.glyphs[GlyphArea::Text.index()];

    assert!(glyphs.iter().any(
        |glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.as_ref() == "سلام")
    ));
    assert!(glyphs.iter().filter(|glyph| glyph.padding).count() >= 3);
}

#[test]
fn display_row_builder_reorders_rtl_rows() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(text_item("אב"));

    let row = builder.finish();

    assert!(row.reversed_p);
    assert_eq!(row_text(&row), "בא");
}

#[test]
fn display_row_builder_emits_stretch_with_pixel_width() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(DisplayItem::new(
        SourceSpan::synthetic(1, 0, 1),
        RenderFaceRef::FaceId(4),
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(24.0)),
            height: Some(DisplayLength::Pixels(16.0)),
            ascent: Some(DisplayLength::Pixels(12.0)),
        }),
    ));

    let row = builder.finish();
    let glyph = &row.glyphs[GlyphArea::Text.index()][0];

    assert_eq!(glyph.glyph_type, GlyphType::Stretch { width_cols: 3 });
    assert_eq!(glyph.face_id, 4);
    assert_eq!(glyph.pixel_width, 24.0);
    assert_eq!(glyph.pixel_height, 16.0);
    assert_eq!(glyph.pixel_ascent, 12.0);
}

#[test]
fn display_row_builder_promotes_explicit_stretch_height_to_row_metrics() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(DisplayItem::new(
        SourceSpan::synthetic(1, 0, 1),
        RenderFaceRef::FaceId(4),
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(24.0)),
            height: Some(DisplayLength::Pixels(24.0)),
            ascent: Some(DisplayLength::Pixels(24.0)),
        }),
    ));

    let row = builder.finish();

    assert_eq!(row.height_px, 24.0);
    assert_eq!(row.ascent_px, 24.0);
}

#[test]
fn display_row_builder_ceil_pixel_stretch_columns() {
    let mut builder = DisplayRowBuilder::new(layout());
    builder.push_item(stretch_item(DisplayLength::Pixels(9.0)));

    let row = builder.finish();
    let glyph = &row.glyphs[GlyphArea::Text.index()][0];

    assert_eq!(glyph.glyph_type, GlyphType::Stretch { width_cols: 2 });
    assert_eq!(glyph.pixel_width, 9.0);
}
