//! Buffer text typed source item consumption.

use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_source::{
    BufferTextDisplayReplacementMode, BufferTextSourceCursor, BufferTextSourceCursorItem,
    BufferTextSourcePosition,
};
use crate::display_buffer_text_source_render_item::BufferTextDirectDisplayItem;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayRowBreakReason, DisplaySourcePosition,
};
use crate::display_source::DisplaySourceContext;
use crate::neovm_bridge::LayoutBufferView;
use neovm_core::buffer::CharPos0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceItem {
    item: DisplayItem,
    start_byte_idx: usize,
    start_charpos: i64,
    source_char: Option<char>,
}

#[derive(Clone, Debug, PartialEq)]
enum BufferTextAlignedSourceCursorItem {
    Item(BufferTextSourceItem),
    Replacement(BufferTextReplacementItem),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceConsumptionItem {
    DisplayItem(BufferTextDirectDisplayItem),
    Replacement(BufferTextReplacementItem),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceConsumptionState {
    text_start_byte: usize,
}

impl BufferTextSourceItem {
    pub(crate) fn new(
        item: DisplayItem,
        start_byte_idx: usize,
        start_charpos: i64,
        source_char: Option<char>,
    ) -> Self {
        Self {
            item,
            start_byte_idx,
            start_charpos,
            source_char,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(
        item: DisplayItem,
        start_byte_idx: usize,
        start_charpos: i64,
        source_char: Option<char>,
    ) -> Self {
        Self::new(item, start_byte_idx, start_charpos, source_char)
    }

    pub(crate) fn direct_source_char(&self) -> Option<char> {
        match &self.item.kind {
            DisplayItemKind::TextRun(run) => run.text.chars().next(),
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline =>
            {
                Some('\n')
            }
            DisplayItemKind::ControlChar { ch } => Some(*ch),
            DisplayItemKind::Glyphless(glyphless) => Some(glyphless.ch),
            DisplayItemKind::SourceMappedText(_) => self.source_char,
            _ => None,
        }
    }

    pub(crate) fn start_byte_idx(&self) -> usize {
        self.start_byte_idx
    }

    pub(crate) fn start_charpos(&self) -> i64 {
        self.start_charpos
    }

    #[cfg(test)]
    pub(crate) fn item(&self) -> &DisplayItem {
        &self.item
    }

    pub(crate) fn into_item(self) -> DisplayItem {
        self.item
    }

    pub(crate) fn buffer_byte_len(&self) -> Option<usize> {
        self.item.span.buffer_byte_len()
    }
}

impl BufferTextSourceConsumptionState {
    pub(crate) fn new(text_start_byte: usize) -> Self {
        Self { text_start_byte }
    }

    fn expected_source_pos(position: BufferTextSourcePosition) -> CharPos0 {
        CharPos0::new(position.charpos().max(0) as usize)
    }

    fn align_display_item(
        &self,
        position: BufferTextSourcePosition,
        source_char: Option<char>,
        item: DisplayItem,
    ) -> Option<BufferTextSourceItem> {
        let DisplaySourcePosition::Buffer { byte_pos, .. } = item.span.start else {
            tracing::error!(
                "BufferTextSourceConsumptionState: source cursor yielded a non-buffer-span item; \
                 a display property escaped the render_next_step checkpoints"
            );
            return None;
        };
        let start_byte_idx = byte_pos.get().checked_sub(self.text_start_byte)?;
        if start_byte_idx != position.byte_idx() {
            tracing::error!(
                "BufferTextSourceConsumptionState: source cursor byte position {} did not match \
                 buffer walk byte index {}",
                start_byte_idx,
                position.byte_idx()
            );
            return None;
        }
        let DisplaySourcePosition::Buffer { char_pos, .. } = item.span.start else {
            unreachable!("buffer byte position match implies buffer source position");
        };
        let start_charpos = char_pos.get() as i64;
        if start_charpos != position.charpos() {
            tracing::error!(
                "BufferTextSourceConsumptionState: source cursor char position {} did not match \
                 buffer walk char position {}",
                start_charpos,
                position.charpos()
            );
            return None;
        }
        Some(BufferTextSourceItem::new(
            item,
            start_byte_idx,
            start_charpos,
            source_char,
        ))
    }

    fn replacement_matches(
        &self,
        position: BufferTextSourcePosition,
        item: &BufferTextReplacementItem,
    ) -> Option<bool> {
        let anchor = item.source_anchor(self.text_start_byte)?;
        Some(anchor.matches(position.byte_idx(), position.charpos()))
    }

    fn read_source_cursor<B: LayoutBufferView + ?Sized>(
        &self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: BufferTextSourcePosition,
        replacement_mode: BufferTextDisplayReplacementMode,
    ) -> Option<BufferTextAlignedSourceCursorItem> {
        let expected_source_pos = Self::expected_source_pos(position);
        if source.current_char_pos() != expected_source_pos {
            source.reset_to(expected_source_pos);
        }

        let source_char = source.char_at(expected_source_pos);
        match source.next_cursor_item(context, replacement_mode)? {
            BufferTextSourceCursorItem::Item(item) => self
                .align_display_item(position, source_char, item)
                .map(BufferTextAlignedSourceCursorItem::Item),
            BufferTextSourceCursorItem::Replacement(item) => {
                if !self.replacement_matches(position, &item)? {
                    tracing::error!(
                        "BufferTextSourceConsumptionState: display replacement did not match \
                         buffer walk byte {} charpos {}",
                        position.byte_idx(),
                        position.charpos()
                    );
                    return None;
                }
                Some(BufferTextAlignedSourceCursorItem::Replacement(item))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn next_display_item_from_source<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextDirectDisplayItem> {
        let item = self.next_item_from_source(source, context, position)?;
        self.consume_aligned_display_item(item, position)
    }

    #[cfg(test)]
    pub(crate) fn next_item_from_source<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &BufferTextSourcePosition,
    ) -> Option<BufferTextSourceItem> {
        match self.read_source_cursor(
            source,
            context,
            *position,
            BufferTextDisplayReplacementMode::InlineSourceItems,
        )? {
            BufferTextAlignedSourceCursorItem::Item(item) => Some(item),
            BufferTextAlignedSourceCursorItem::Replacement(_) => {
                debug_assert!(false, "inline source cursor surfaced a buffer replacement");
                None
            }
        }
    }

    pub(crate) fn next_source_consumption_item<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceConsumptionItem> {
        match self.read_source_cursor(
            source,
            context,
            *position,
            BufferTextDisplayReplacementMode::TypedReplacementItem,
        )? {
            BufferTextAlignedSourceCursorItem::Item(item) => self
                .consume_aligned_display_item(item, position)
                .map(BufferTextSourceConsumptionItem::DisplayItem),
            BufferTextAlignedSourceCursorItem::Replacement(item) => {
                Some(BufferTextSourceConsumptionItem::Replacement(item))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn render_item_from_item(
        &mut self,
        item: DisplayItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextDirectDisplayItem> {
        let item = self.align_display_item(*position, None, item)?;
        self.consume_aligned_display_item(item, position)
    }

    #[cfg(test)]
    pub(crate) fn render_item_from_source_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextDirectDisplayItem> {
        self.consume_aligned_display_item(item, position)
    }

    fn consume_aligned_display_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextDirectDisplayItem> {
        BufferTextDirectDisplayItem::consume_source_item(item, position).ok()
    }
}
