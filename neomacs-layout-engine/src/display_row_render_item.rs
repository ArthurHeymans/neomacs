use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayMediaReplacement, DisplaySourceMappedText,
    DisplaySourcePosition, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_render_state::PendingDisplayRowMedia;
use neovm_core::buffer::{CharPos0, EmacsBytePos};

impl DisplayMediaReplacement {
    fn pending_media(self, start: DisplayRowPosition) -> PendingDisplayRowMedia {
        let (horizontal_margin, vertical_margin) = match self.kind {
            crate::display_item::DisplayMediaReplacementKind::Image {
                horizontal_margin,
                vertical_margin,
                ..
            } => (horizontal_margin, vertical_margin),
            _ => (0.0, 0.0),
        };
        PendingDisplayRowMedia {
            kind: self.kind.into(),
            x: start.x_px() + horizontal_margin,
            col: start.col().min(usize::from(u16::MAX)) as u16,
            width: self.width - 2.0 * horizontal_margin,
            height: self.height - 2.0 * vertical_margin,
            ascent: self.ascent - vertical_margin,
        }
    }
}

pub(crate) struct DisplayRowRenderItem {
    source_item: DisplayItem,
    row_item: DisplayItem,
    media_descriptor: Option<DisplayMediaReplacement>,
}

impl DisplayRowRenderItem {
    pub(crate) fn from_source_item(source_item: DisplayItem) -> Self {
        let media_descriptor = match &source_item.kind {
            DisplayItemKind::MediaReplacement(media) => Some(*media),
            _ => None,
        };
        let row_item = media_descriptor
            .map(|descriptor| descriptor.replacement_item(source_item.clone()))
            .unwrap_or_else(|| source_item.clone());
        Self {
            source_item,
            row_item,
            media_descriptor,
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

    pub(crate) fn pending_media_for_progress(
        &self,
        progress: &DisplayRowAppendProgress,
    ) -> Option<PendingDisplayRowMedia> {
        let descriptor = self.media_descriptor?;
        progress
            .is_complete_with_positive_width()
            .then(|| descriptor.pending_media(progress.start()))
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
    use crate::display_item::{DisplayImageItem, DisplayMediaReplacementKind};

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

        let medium = replacement
            .pending_media(DisplayRowPosition::new(11.0, 4))
            .place_on_baseline(30.0);
        assert_eq!(
            (medium.x, medium.y, medium.width, medium.height),
            (14.0, 22.0, 20.0, 10.0)
        );
        assert_eq!(
            medium.kind,
            crate::display_row_render_state::RenderedDisplayRowMediaKind::Image {
                image_id: 7,
                opaque_background: Some(0x12_34_56),
            }
        );
        assert!(matches!(
            replacement.kind,
            DisplayMediaReplacementKind::Image {
                horizontal_margin: 3.0,
                vertical_margin: 2.0,
                ..
            }
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
