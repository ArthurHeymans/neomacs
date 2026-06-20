use super::*;
use crate::display_source::{DisplayItemSource, DisplaySourceContext};
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
        RenderFaceRef::FaceId(12),
        DisplayItemKind::TextRun(DisplayTextRun::new("abc")),
    );

    assert_eq!(item.span, span);
    assert_eq!(item.face, RenderFaceRef::FaceId(12));
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
fn display_item_row_break_cursor_and_hit_test_are_typed_items() {
    let source_pos = DisplaySourcePosition::lisp_string(5, 4, 4);
    let span = SourceSpan::new(source_pos.clone(), source_pos.clone());

    let row_break = DisplayItem::new(
        span.clone(),
        RenderFaceRef::Inherit,
        DisplayItemKind::RowBreak(DisplayRowBreak {
            reason: DisplayRowBreakReason::ExplicitNewline,
        }),
    );
    let cursor = DisplayItem::new(
        span.clone(),
        RenderFaceRef::Inherit,
        DisplayItemKind::CursorAnchor(CursorAnchor {
            kind: CursorAnchorKind::Point,
            position: source_pos.clone(),
        }),
    );
    let hit_test = DisplayItem::new(
        span,
        RenderFaceRef::Inherit,
        DisplayItemKind::HitTestAnchor(DisplayHitTestAnchor {
            position: source_pos,
        }),
    );

    assert!(matches!(
        row_break.kind,
        DisplayItemKind::RowBreak(DisplayRowBreak {
            reason: DisplayRowBreakReason::ExplicitNewline
        })
    ));
    assert!(matches!(
        cursor.kind,
        DisplayItemKind::CursorAnchor(CursorAnchor {
            kind: CursorAnchorKind::Point,
            ..
        })
    ));
    assert!(matches!(
        hit_test.kind,
        DisplayItemKind::HitTestAnchor(DisplayHitTestAnchor { .. })
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
        RenderFaceRef::FaceId(1),
        DisplayItemKind::TextRun(DisplayTextRun::new("x")),
    );
    let mut source = StaticItemSource {
        items: vec![expected.clone()].into_iter(),
    };
    let mut context = DisplaySourceContext::empty();

    assert_eq!(source.next_item(&mut context), Some(expected));
    assert_eq!(source.next_item(&mut context), None);
}
