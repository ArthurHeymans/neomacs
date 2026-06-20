//! Buffer text typed source item consumption.

use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_source::{
    BufferTextDisplayReplacementMode, BufferTextSourceCursor, BufferTextSourceCursorItem,
    BufferTextSourcePosition,
};
use crate::display_buffer_text_source_render_item::{
    BufferTextDirectDisplayItemRequest, BufferTextSourceRenderItem, BufferTextSplitTextRunState,
};
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayRowBreakReason, DisplaySourcePosition,
};
use crate::display_source::DisplaySourceContext;
use crate::neovm_bridge::LayoutBufferView;
use neovm_core::buffer::CharPos0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferTextSourceAlignmentRequest {
    text_start_byte: usize,
    position: BufferTextSourcePosition,
    source_char: Option<char>,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferTextSourceCursorReadRequest {
    text_start_byte: usize,
    position: BufferTextSourcePosition,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceConsumptionItem {
    DisplayItem(BufferTextSourceRenderItem),
    Replacement(BufferTextReplacementItem),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceConsumptionState {
    text_start_byte: usize,
    split_text_run: BufferTextSplitTextRunState,
}

impl BufferTextSourceAlignmentRequest {
    fn new(
        text_start_byte: usize,
        position: BufferTextSourcePosition,
        source_char: Option<char>,
    ) -> Self {
        Self {
            text_start_byte,
            position,
            source_char,
        }
    }

    fn for_position(text_start_byte: usize, position: BufferTextSourcePosition) -> Self {
        Self::new(text_start_byte, position, None)
    }

    fn align_display_item(self, item: DisplayItem) -> Option<BufferTextSourceItem> {
        let DisplaySourcePosition::Buffer { byte_pos, .. } = item.span.start else {
            tracing::error!(
                "BufferTextSourceConsumptionState: source cursor yielded a non-buffer-span item; \
                 a display property escaped the render_next_step checkpoints"
            );
            return None;
        };
        let start_byte_idx = byte_pos.get().checked_sub(self.text_start_byte)?;
        if start_byte_idx != self.position.byte_idx() {
            tracing::error!(
                "BufferTextSourceConsumptionState: source cursor byte position {} did not match \
                 buffer walk byte index {}",
                start_byte_idx,
                self.position.byte_idx()
            );
            return None;
        }
        let DisplaySourcePosition::Buffer { char_pos, .. } = item.span.start else {
            unreachable!("buffer byte position match implies buffer source position");
        };
        let start_charpos = char_pos.get() as i64;
        if start_charpos != self.position.charpos() {
            tracing::error!(
                "BufferTextSourceConsumptionState: source cursor char position {} did not match \
                 buffer walk char position {}",
                start_charpos,
                self.position.charpos()
            );
            return None;
        }
        Some(BufferTextSourceItem::new(
            item,
            start_byte_idx,
            start_charpos,
            self.source_char,
        ))
    }

    fn replacement_matches(self, item: &BufferTextReplacementItem) -> Option<bool> {
        let anchor = item.source_anchor(self.text_start_byte)?;
        Some(anchor.matches(self.position.byte_idx(), self.position.charpos()))
    }
}

impl BufferTextSourceCursorReadRequest {
    fn new(text_start_byte: usize, position: BufferTextSourcePosition) -> Self {
        Self {
            text_start_byte,
            position,
        }
    }

    fn expected_source_pos(self) -> CharPos0 {
        CharPos0::new(self.position.charpos().max(0) as usize)
    }

    fn alignment(self, source_char: Option<char>) -> BufferTextSourceAlignmentRequest {
        BufferTextSourceAlignmentRequest::new(self.text_start_byte, self.position, source_char)
    }

    fn replacement_alignment(self) -> BufferTextSourceAlignmentRequest {
        BufferTextSourceAlignmentRequest::for_position(self.text_start_byte, self.position)
    }

    fn read<B: LayoutBufferView + ?Sized>(
        self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        replacement_mode: BufferTextDisplayReplacementMode,
    ) -> Option<BufferTextAlignedSourceCursorItem> {
        let expected_source_pos = self.expected_source_pos();
        if source.current_char_pos() != expected_source_pos {
            source.reset_to(expected_source_pos);
        }

        let source_char = source.char_at(expected_source_pos);
        match source.next_cursor_item(context, replacement_mode)? {
            BufferTextSourceCursorItem::Item(item) => self
                .alignment(source_char)
                .align_display_item(item)
                .map(BufferTextAlignedSourceCursorItem::Item),
            BufferTextSourceCursorItem::Replacement(item) => {
                if !self.replacement_alignment().replacement_matches(&item)? {
                    tracing::error!(
                        "BufferTextSourceConsumptionState: display replacement did not match \
                         buffer walk byte {} charpos {}",
                        self.position.byte_idx(),
                        self.position.charpos()
                    );
                    return None;
                }
                Some(BufferTextAlignedSourceCursorItem::Replacement(item))
            }
        }
    }
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
            DisplayItemKind::TextRun(run) => {
                let mut chars = run.text.chars();
                let ch = chars.next()?;
                chars.next().is_none().then_some(ch)
            }
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
        Self {
            text_start_byte,
            split_text_run: BufferTextSplitTextRunState::new(text_start_byte),
        }
    }

    #[cfg(test)]
    pub(crate) fn next_display_item_from_source<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceRenderItem> {
        if let Some(step) = self.split_text_run.next_pending_display_item(position) {
            return Some(step);
        }

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
        if self.split_text_run.has_pending_text_run() {
            tracing::debug!(
                "BufferTextSourceConsumptionState: requested typed item while a text run is pending"
            );
            return None;
        }

        match BufferTextSourceCursorReadRequest::new(self.text_start_byte, *position).read(
            source,
            context,
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
        if let Some(step) = self.split_text_run.next_pending_display_item(position) {
            return Some(BufferTextSourceConsumptionItem::DisplayItem(step));
        }

        if self.split_text_run.has_pending_text_run() {
            tracing::debug!(
                "BufferTextSourceConsumptionState: requested typed item while a text run is pending"
            );
            return None;
        }

        match BufferTextSourceCursorReadRequest::new(self.text_start_byte, *position).read(
            source,
            context,
            BufferTextDisplayReplacementMode::LoweredSourceItem,
        )? {
            BufferTextAlignedSourceCursorItem::Item(item) => self
                .consume_aligned_display_item(item, position)
                .map(BufferTextSourceConsumptionItem::DisplayItem),
            BufferTextAlignedSourceCursorItem::Replacement(item) => {
                Some(BufferTextSourceConsumptionItem::Replacement(item))
            }
        }
    }

    pub(crate) fn consume_fallback_source_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceRenderItem> {
        self.consume_aligned_display_item(item, position)
    }

    #[cfg(test)]
    pub(crate) fn lowered_display_item_from_item(
        &mut self,
        item: DisplayItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceRenderItem> {
        let item = BufferTextSourceAlignmentRequest::for_position(self.text_start_byte, *position)
            .align_display_item(item)?;
        self.consume_aligned_display_item(item, position)
    }

    #[cfg(test)]
    pub(crate) fn lowered_display_item_from_source_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceRenderItem> {
        self.consume_aligned_display_item(item, position)
    }

    fn consume_aligned_display_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceRenderItem> {
        match BufferTextDirectDisplayItemRequest::new(item).consume(position) {
            Ok(item) => Some(item),
            Err(item) => self.split_text_run.consume_text_run_item(item, position),
        }
    }
}
