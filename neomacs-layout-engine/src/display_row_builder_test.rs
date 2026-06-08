use super::*;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLength, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphType};

fn layout() -> DisplayRowLayout {
    DisplayRowLayout {
        role: GlyphRowRole::Text,
        width_px: 240.0,
        height_px: 16.0,
        ascent_px: 12.0,
        char_width_px: 8.0,
        base_face: RenderFaceRef::FaceId(1),
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
