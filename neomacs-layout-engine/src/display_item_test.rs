use super::*;
use crate::display_source::{DisplayItemSource, DisplaySourceContext};
use neomacs_display_protocol::types::FaceId;
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos};

fn buffer_span(buffer_id: BufferId, start_char: usize, end_char: usize) -> SourceSpan {
    SourceSpan::new(
        DisplaySourcePosition::buffer(
            buffer_id,
            CharPos0::new(start_char),
            EmacsBytePos::new(start_char),
        ),
        DisplaySourcePosition::buffer(
            buffer_id,
            CharPos0::new(end_char),
            EmacsBytePos::new(end_char),
        ),
    )
}

#[test]
fn display_item_text_run_keeps_source_span_and_face_ref() {
    let span = buffer_span(BufferId(7), 3, 6);
    let item = DisplayItem::new(
        span.clone(),
        RenderFaceRef::FaceId(FaceId::new(12)),
        DisplayItemKind::TextRun(DisplayTextRun::new("abc")),
    );

    assert_eq!(item.span, span);
    assert_eq!(item.face, RenderFaceRef::FaceId(FaceId::new(12)));
    assert_eq!(
        item.kind,
        DisplayItemKind::TextRun(DisplayTextRun::new("abc"))
    );
}

#[test]
fn display_item_stretch_uses_typed_lengths() {
    let item = DisplayItem::new(
        SourceSpan::synthetic(1, 0, 1),
        RenderFaceRef::Inherit,
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(32.0)),
            height: Some(DisplayLength::Pixels(14.0)),
            ascent: Some(DisplayLength::Pixels(10.0)),
        }),
    );

    assert_eq!(
        item.kind,
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(32.0)),
            height: Some(DisplayLength::Pixels(14.0)),
            ascent: Some(DisplayLength::Pixels(10.0)),
        })
    );
}

#[test]
fn display_item_inline_media_slots_are_source_neutral() {
    let span = SourceSpan::lisp_string(2, 0, 1, 0, 1);

    let image = DisplayItem::new(
        span.clone(),
        RenderFaceRef::Inherit,
        DisplayItemKind::MediaReplacement(DisplayMediaReplacement::image(DisplayImageItem {
            image_id: 42,
            width: 64.0,
            height: 32.0,
            ascent: 32.0,
            horizontal_margin: 0.0,
            vertical_margin: 0.0,
            opaque_background: None,
        })),
    );
    let video = DisplayItem::new(
        span.clone(),
        RenderFaceRef::Inherit,
        DisplayItemKind::MediaReplacement(DisplayMediaReplacement::video(DisplayVideoItem {
            video_id: 43,
            width: 80.0,
            height: 45.0,
            loop_count: -1,
            autoplay: true,
        })),
    );
    let xwidget = DisplayItem::new(
        span,
        RenderFaceRef::Inherit,
        DisplayItemKind::MediaReplacement(DisplayMediaReplacement::xwidget(DisplayXwidgetItem {
            xwidget_id: 44,
            width: 96.0,
            height: 54.0,
        })),
    );

    assert_eq!(
        image.kind,
        DisplayItemKind::MediaReplacement(DisplayMediaReplacement::image(DisplayImageItem {
            image_id: 42,
            width: 64.0,
            height: 32.0,
            ascent: 32.0,
            horizontal_margin: 0.0,
            vertical_margin: 0.0,
            opaque_background: None,
        }))
    );
    assert_eq!(
        video.kind,
        DisplayItemKind::MediaReplacement(DisplayMediaReplacement::video(DisplayVideoItem {
            video_id: 43,
            width: 80.0,
            height: 45.0,
            loop_count: -1,
            autoplay: true,
        }))
    );
    assert_eq!(
        xwidget.kind,
        DisplayItemKind::MediaReplacement(DisplayMediaReplacement::xwidget(DisplayXwidgetItem {
            xwidget_id: 44,
            width: 96.0,
            height: 54.0,
        }))
    );
}

#[test]
fn display_item_row_break_is_a_typed_item() {
    let source_pos = DisplaySourcePosition::lisp_string(5, 4, 4);
    let span = SourceSpan::new(source_pos.clone(), source_pos);

    let row_break = DisplayItem::new(
        span,
        RenderFaceRef::Inherit,
        DisplayItemKind::RowBreak(DisplayRowBreak::explicit_newline()),
    );

    assert!(matches!(
        row_break.kind,
        DisplayItemKind::RowBreak(DisplayRowBreak {
            reason: DisplayRowBreakReason::ExplicitNewline,
            ..
        })
    ));
}

struct StaticItemSource {
    items: std::vec::IntoIter<DisplayItem>,
}

impl DisplayItemSource for StaticItemSource {
    fn next_item(&mut self, _context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.items.next()
    }
}

#[test]
fn display_item_source_trait_exposes_items() {
    let expected = DisplayItem::new(
        SourceSpan::synthetic(9, 0, 1),
        RenderFaceRef::FaceId(FaceId::new(1)),
        DisplayItemKind::TextRun(DisplayTextRun::new("x")),
    );
    let mut source = StaticItemSource {
        items: vec![expected.clone()].into_iter(),
    };
    let mut context = DisplaySourceContext::empty();

    assert_eq!(source.next_item(&mut context), Some(expected));
    assert_eq!(source.next_item(&mut context), None);
}

#[test]
fn glyphless_method_routes_cf_format_control_to_zero_width() {
    let m = |c: char| glyphless_method_for_char(c, GlyphlessJoinerPolicy::ClassifyAsGlyphless);
    // GNU `format-control` group (Cf, except SHY): the interlinear annotation
    // marks U+FFF9..U+FFFB were drawn as a `.notdef` box; they must render
    // invisible like ZWSP/ZWJ (glyphless-char-display-control default).
    assert_eq!(m('\u{fff9}'), Some(GlyphlessMethod::ZeroWidth)); // IAA
    assert_eq!(m('\u{fffa}'), Some(GlyphlessMethod::ZeroWidth)); // IAS
    assert_eq!(m('\u{fffb}'), Some(GlyphlessMethod::ZeroWidth)); // IAT
    assert_eq!(m('\u{2060}'), Some(GlyphlessMethod::ZeroWidth)); // WORD JOINER (Cf)
    assert_eq!(m('\u{200d}'), Some(GlyphlessMethod::ZeroWidth)); // ZWJ (fast-path)
    // SHY (U+00AD) is Cf but GNU excludes it (it has a visible glyph).
    assert_eq!(m('\u{00ad}'), None);
    // Hot-path printables stay non-glyphless.
    assert_eq!(m('a'), None);
    assert_eq!(m('中'), None);
    // Object Replacement keeps its EmptyBox handling; the non-printable
    // noncharacters (U+FDD0.., U+FFFE/U+FFFF) are octal-escaped upstream of this
    // fn, so they are intentionally not routed here.
    assert_eq!(m('\u{fffc}'), Some(GlyphlessMethod::EmptyBox));
}

#[test]
fn glyphless_method_keeps_non_ignorable_format_controls_visible() {
    let m = |c: char| glyphless_method_for_char(c, GlyphlessJoinerPolicy::ClassifyAsGlyphless);
    // `Cf` chars that are NOT `Default_Ignorable_Code_Point` must render, not
    // hide -- GNU emits them (font glyph or composed cluster). Regression: a
    // blanket "all `Cf` -> ZeroWidth" dropped U+180E from etc/HELLO's Mongolian
    // line, diverging from GNU on the TTY.
    assert_eq!(m('\u{180e}'), None); // MONGOLIAN VOWEL SEPARATOR (removed from DI in 6.3)
    assert_eq!(m('\u{0600}'), None); // ARABIC NUMBER SIGN (prepended concatenation mark)
    assert_eq!(m('\u{06dd}'), None); // ARABIC END OF AYAH
    assert_eq!(m('\u{070f}'), None); // SYRIAC ABBREVIATION MARK
    assert_eq!(m('\u{08e2}'), None); // ARABIC DISPUTED END OF AYAH
    assert_eq!(m('\u{110bd}'), None); // KAITHI NUMBER SIGN
    assert_eq!(m('\u{13430}'), None); // EGYPTIAN HIEROGLYPH VERTICAL JOINER
    // Other `Cf` chars that ARE Default_Ignorable still hide: e.g. U+061C
    // ARABIC LETTER MARK (a bidi control), so the narrowing is exact.
    assert_eq!(m('\u{061c}'), Some(GlyphlessMethod::ZeroWidth));
}
