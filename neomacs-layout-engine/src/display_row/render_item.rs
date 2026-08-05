use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplaySourceMappedText, DisplaySourcePosition, DisplayTextRun,
    RenderFaceRef, SourceSpan,
};
use crate::display_row::builder::DisplayRowAppendProgress;
use neovm_core::buffer::{CharPos0, EmacsBytePos};

pub(crate) struct DisplayRowRenderItem {
    source_item: DisplayItem,
    row_item: DisplayItem,
}

impl DisplayRowRenderItem {
    pub(crate) fn from_source_item(source_item: DisplayItem) -> Self {
        // Preserve media as one row item.  The row writer now emits a typed
        // media glyph that owns both its layout metrics and drawable identity.
        let row_item = source_item.clone();
        Self {
            source_item,
            row_item,
        }
    }

    pub(crate) fn source_item(&self) -> &DisplayItem {
        &self.source_item
    }

    pub(crate) fn row_face(&self) -> RenderFaceRef {
        self.row_item.face
    }

    pub(crate) fn row_item(&self) -> &DisplayItem {
        &self.row_item
    }

    pub(crate) fn row_item_for_write(&self) -> DisplayItem {
        self.row_item.clone()
    }

    pub(crate) fn clipped_remainder(
        self,
        progress: &DisplayRowAppendProgress,
    ) -> Option<DisplayItem> {
        clipped_display_item_remainder(self.source_item, progress)
    }
}

fn clipped_display_item_remainder(
    item: DisplayItem,
    progress: &DisplayRowAppendProgress,
) -> Option<DisplayItem> {
    let DisplayItem {
        span,
        face,
        kind,
        layout,
        pointer_appearance,
    } = item;
    let emitted_chars = progress.slots().len();
    match kind {
        DisplayItemKind::TextRun(run) => {
            let (split_byte, remaining) = clipped_text_remainder(run.text.as_ref(), emitted_chars)?;
            Some(DisplayItem {
                span: SourceSpan::new(
                    display_source_position_advance(&span.start, emitted_chars, split_byte),
                    span.end,
                ),
                face,
                kind: DisplayItemKind::TextRun(DisplayTextRun::new(remaining)),
                layout,
                pointer_appearance,
            })
        }
        DisplayItemKind::SourceMappedText(text) => {
            let (_, remaining) = clipped_text_remainder(text.text.as_ref(), emitted_chars)?;
            Some(DisplayItem {
                span,
                face,
                kind: DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(remaining)),
                layout,
                pointer_appearance,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display_item::{
        DisplayImageItem, DisplayMediaReplacement, DisplayMediaReplacementKind,
    };

    #[test]
    fn image_margin_expands_the_slot_but_keeps_media_bounds_on_image_content() {
        let replacement = DisplayMediaReplacement::image(DisplayImageItem {
            image_id: 7,
            width: 20.0,
            height: 10.0,
            ascent: 8.0,
            horizontal_margin: 3.0,
            vertical_margin: 2.0,
            opaque_background: Some(0x12_34_56),
        });
        assert_eq!(replacement.width, 26.0);
        assert_eq!(replacement.height, 14.0);
        assert_eq!(replacement.ascent, 10.0);

        assert!(matches!(
            replacement.kind,
            DisplayMediaReplacementKind::Image {
                horizontal_margin: 3.0,
                vertical_margin: 2.0,
                ..
            }
        ));
    }

    #[test]
    fn media_replacement_remains_one_authoritative_row_item() {
        let replacement = DisplayMediaReplacement::image(DisplayImageItem {
            image_id: 7,
            width: 20.0,
            height: 10.0,
            ascent: 8.0,
            horizontal_margin: 3.0,
            vertical_margin: 2.0,
            opaque_background: Some(0x12_34_56),
        });
        let source = DisplayItem {
            span: SourceSpan::synthetic(1, 0, 1),
            face: RenderFaceRef::FaceId(neomacs_display_protocol::types::FaceId::new(1)),
            kind: DisplayItemKind::MediaReplacement(replacement),
            layout: Default::default(),
            pointer_appearance: None,
        };

        let rendered = DisplayRowRenderItem::from_source_item(source);

        assert!(matches!(
            rendered.row_item().kind,
            DisplayItemKind::MediaReplacement(actual) if actual == replacement
        ));
    }
}

fn clipped_text_remainder(text: &str, emitted_chars: usize) -> Option<(usize, String)> {
    if emitted_chars >= text.chars().count() {
        return None;
    }
    let split_byte = text
        .char_indices()
        .nth(emitted_chars)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len());
    Some((split_byte, text[split_byte..].to_string()))
}

fn display_source_position_advance(
    start: &DisplaySourcePosition,
    char_offset: usize,
    byte_offset: usize,
) -> DisplaySourcePosition {
    match start {
        DisplaySourcePosition::Buffer {
            buffer_id,
            char_pos,
            byte_pos,
        } => DisplaySourcePosition::buffer(
            *buffer_id,
            CharPos0::new(char_pos.get() + char_offset),
            EmacsBytePos::new(byte_pos.get() + byte_offset),
        ),
        DisplaySourcePosition::LispString {
            source_id,
            char_index,
            byte_index,
        } => DisplaySourcePosition::lisp_string(
            source_id.get(),
            char_index + char_offset,
            byte_index + byte_offset,
        ),
        DisplaySourcePosition::Synthetic { source_id, offset } => {
            DisplaySourcePosition::synthetic(source_id.get(), offset + char_offset)
        }
    }
}
