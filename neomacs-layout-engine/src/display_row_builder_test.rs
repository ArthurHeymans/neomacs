use super::*;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLength, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_source::{DisplaySourceContext, LispStringSourceCursor};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphType};
use neovm_core::emacs_core::{Context, Value};

fn layout() -> DisplayRowLayout {
    DisplayRowLayout {
        role: GlyphRowRole::Text,
        y_px: 0.0,
        width_px: 240.0,
        height_px: 16.0,
        ascent_px: 12.0,
        char_width_px: 8.0,
        tab_width_cols: 4,
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
