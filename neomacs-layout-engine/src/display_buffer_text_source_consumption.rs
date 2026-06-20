//! Buffer text typed source item consumption.

use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_source::{
    BufferTextDisplayReplacementMode, BufferTextSourceCursor, BufferTextSourceCursorItem,
    BufferTextSourcePosition,
};
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayRowBreakReason, DisplaySourcePosition,
    DisplayTextRunItemCursor,
};
use crate::display_source::DisplaySourceContext;
use crate::neovm_bridge::LayoutBufferView;
use crate::unicode::decode_utf8;
use neovm_core::buffer::CharPos0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferTextSourceAlignmentRequest {
    text_start_byte: usize,
    position: BufferTextSourcePosition,
    source_char: Option<char>,
}

/// A single source character aligned with the current buffer byte and char
/// positions for the row walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourceStepChar {
    ch: char,
    start_byte_idx: usize,
    start_charpos: i64,
}

/// A typed display item consumed by the buffer text row walk after it has been
/// aligned with the current buffer byte/char cursor.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextConsumedDisplayItem {
    source_char: BufferTextSourceStepChar,
    item: DisplayItem,
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
    DisplayItem(BufferTextConsumedDisplayItem),
    Replacement(BufferTextReplacementItem),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceConsumptionState {
    text_start_byte: usize,
    pending_text_run: Option<DisplayTextRunItemCursor>,
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

    fn split_text_item_start(self, item: &DisplayItem) -> Option<(usize, i64)> {
        let DisplaySourcePosition::Buffer {
            char_pos, byte_pos, ..
        } = &item.span.start
        else {
            tracing::error!(
                "BufferTextSourceConsumptionState: split text run yielded a non-buffer-span item"
            );
            return None;
        };
        let start_byte_idx = byte_pos.get().checked_sub(self.text_start_byte)?;
        let start_charpos = char_pos.get() as i64;
        if !self.position.matches(start_byte_idx, start_charpos) {
            tracing::debug!(
                "BufferTextSourceConsumptionState: split text run at byte {} charpos {} did not \
                 match buffer walk byte {} charpos {}",
                start_byte_idx,
                start_charpos,
                self.position.byte_idx(),
                self.position.charpos()
            );
            return None;
        }
        Some((start_byte_idx, start_charpos))
    }

    fn split_text_item_end_byte_idx(self, item: &DisplayItem) -> Option<usize> {
        let DisplaySourcePosition::Buffer {
            byte_pos: end_byte_pos,
            ..
        } = &item.span.end
        else {
            tracing::error!(
                "BufferTextSourceConsumptionState: split text run yielded a non-buffer end span"
            );
            return None;
        };
        end_byte_pos.get().checked_sub(self.text_start_byte)
    }
}

impl BufferTextSourceStepChar {
    pub(crate) const fn new(ch: char, start_byte_idx: usize, start_charpos: i64) -> Self {
        Self {
            ch,
            start_byte_idx,
            start_charpos,
        }
    }

    pub(crate) fn consume_from_position(
        text: &[u8],
        position: &mut BufferTextSourcePosition,
    ) -> Option<Self> {
        if position.byte_idx() >= text.len() {
            return None;
        }
        let start_byte_idx = position.byte_idx();
        let start_charpos = position.charpos();
        let (ch, ch_len) = decode_utf8(&text[start_byte_idx..]);
        if ch_len == 0 {
            return None;
        }
        position.advance_one_char(ch_len);
        Some(Self::new(ch, start_byte_idx, start_charpos))
    }

    pub(crate) fn ch(self) -> char {
        self.ch
    }

    pub(crate) fn start_byte_idx(self) -> usize {
        self.start_byte_idx
    }

    pub(crate) fn start_charpos(self) -> i64 {
        self.start_charpos
    }

    pub(crate) fn source_range(self) -> crate::display_source::BufferTextSourceRange {
        crate::display_source::BufferTextSourceRange::single_char(CharPos0::new(
            self.start_charpos as usize,
        ))
    }

    pub(crate) fn source_char(
        self,
        nobreak_display_policy: i32,
    ) -> crate::display_source::BufferTextSourceChar {
        crate::display_source::BufferTextSourceChar::new(
            self.ch,
            self.source_range().start(),
            nobreak_display_policy,
        )
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

impl BufferTextConsumedDisplayItem {
    pub(crate) fn new(source_char: BufferTextSourceStepChar, item: DisplayItem) -> Self {
        Self { source_char, item }
    }

    pub(crate) fn source_char(&self) -> BufferTextSourceStepChar {
        self.source_char
    }

    pub(crate) fn is_explicit_line_break(&self) -> bool {
        matches!(
            self.item.kind,
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline
        )
    }

    pub(crate) fn end_charpos(&self) -> i64 {
        display_item_buffer_end_charpos(&self.item)
            .unwrap_or_else(|| self.source_char.start_charpos().saturating_add(1))
    }

    pub(crate) fn into_parts(self) -> (BufferTextSourceStepChar, DisplayItem) {
        (self.source_char, self.item)
    }
}

fn display_item_buffer_end_charpos(item: &DisplayItem) -> Option<i64> {
    item.span
        .buffer_end_charpos()
        .map(|char_pos| char_pos.get() as i64)
}

fn display_item_buffer_byte_len(item: &DisplayItem) -> Option<usize> {
    item.span.buffer_byte_len()
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

    fn direct_source_char(&self) -> Option<char> {
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

    pub(crate) fn try_into_direct_consumed_display_item(
        self,
        position: &mut BufferTextSourcePosition,
    ) -> Result<BufferTextConsumedDisplayItem, Self> {
        if !position.matches(self.start_byte_idx, self.start_charpos) {
            tracing::error!(
                "BufferTextSourceItem: validated source item at byte {} charpos {} \
                 did not match buffer walk byte {} charpos {}",
                self.start_byte_idx,
                self.start_charpos,
                position.byte_idx(),
                position.charpos()
            );
            return Err(self);
        }
        let Some(ch) = self.direct_source_char() else {
            return Err(self);
        };
        let start_byte_idx = self.start_byte_idx;
        let start_charpos = self.start_charpos;
        let byte_len = display_item_buffer_byte_len(&self.item).unwrap_or_else(|| ch.len_utf8());
        position.advance_byte_idx_to(start_byte_idx.saturating_add(byte_len));
        Ok(BufferTextConsumedDisplayItem::new(
            BufferTextSourceStepChar::new(ch, start_byte_idx, start_charpos),
            self.item,
        ))
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
}

impl BufferTextSourceConsumptionState {
    pub(crate) fn new(text_start_byte: usize) -> Self {
        Self {
            text_start_byte,
            pending_text_run: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn next_display_item_from_source<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        if let Some(step) = self.next_pending_display_item(position) {
            return Some(step);
        }

        let item = self.next_item_from_source(source, context, position)?;
        self.consume_aligned_display_item(item, position)
    }

    fn next_pending_display_item(
        &mut self,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        self.next_pending_step(position)
    }

    #[cfg(test)]
    pub(crate) fn next_item_from_source<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &BufferTextSourcePosition,
    ) -> Option<BufferTextSourceItem> {
        if self.pending_text_run.is_some() {
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

    #[cfg(test)]
    pub(crate) fn next_source_consumption_result<B, R>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &mut BufferTextSourcePosition,
        display_item: impl FnOnce(BufferTextConsumedDisplayItem) -> R,
        replacement: impl FnOnce(BufferTextReplacementItem) -> R,
    ) -> Option<R>
    where
        B: LayoutBufferView + ?Sized,
    {
        match self.next_source_consumption_item(source, context, position)? {
            BufferTextSourceConsumptionItem::DisplayItem(item) => Some(display_item(item)),
            BufferTextSourceConsumptionItem::Replacement(item) => Some(replacement(item)),
        }
    }

    pub(crate) fn next_source_consumption_item<B: LayoutBufferView + ?Sized>(
        &mut self,
        source: &mut BufferTextSourceCursor<'_, B>,
        context: &mut DisplaySourceContext<'_>,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceConsumptionItem> {
        if let Some(step) = self.next_pending_display_item(position) {
            return Some(BufferTextSourceConsumptionItem::DisplayItem(step));
        }

        if self.pending_text_run.is_some() {
            tracing::debug!(
                "BufferTextSourceConsumptionState: requested typed item while a text run is pending"
            );
            return None;
        }

        match BufferTextSourceCursorReadRequest::new(self.text_start_byte, *position).read(
            source,
            context,
            BufferTextDisplayReplacementMode::ConsumedSourceItem,
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
    ) -> Option<BufferTextConsumedDisplayItem> {
        self.consume_aligned_display_item(item, position)
    }

    #[cfg(test)]
    pub(crate) fn consumed_display_item_from_item(
        &mut self,
        item: DisplayItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        let item = BufferTextSourceAlignmentRequest::for_position(self.text_start_byte, *position)
            .align_display_item(item)?;
        self.consume_aligned_display_item(item, position)
    }

    #[cfg(test)]
    pub(crate) fn consumed_display_item_from_source_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        self.consume_aligned_display_item(item, position)
    }

    fn consume_aligned_display_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        if !position.matches(item.start_byte_idx(), item.start_charpos()) {
            tracing::error!(
                "BufferTextSourceConsumptionState: validated source item at byte {} charpos {} \
                 did not match buffer walk byte {} charpos {}",
                item.start_byte_idx(),
                item.start_charpos(),
                position.byte_idx(),
                position.charpos()
            );
            return None;
        }
        let item = match item.try_into_direct_consumed_display_item(position) {
            Ok(step) => return Some(step),
            Err(item) => item,
        };
        self.split_text_run_item(item, position)
    }

    fn split_text_run_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        match DisplayTextRunItemCursor::from_item(item.into_item()) {
            Ok(cursor) => {
                self.pending_text_run = Some(cursor);
                self.next_pending_step(position)
            }
            Err(_) => {
                tracing::error!(
                    "BufferTextSourceConsumptionState: source cursor yielded a non-text item kind; \
                     a direct item escaped source-item lowering"
                );
                None
            }
        }
    }

    fn next_pending_step(
        &mut self,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        let text_start_byte = self.text_start_byte;
        let pending = self.pending_text_run.as_mut()?;
        let item = pending.next_item();
        let finished = pending.is_finished();
        let step = item.and_then(|item| {
            Self::consumed_display_item_from_split_text_item(text_start_byte, item, position)
        });
        if finished || step.is_none() {
            self.pending_text_run = None;
        }
        step
    }

    fn consumed_display_item_from_split_text_item(
        text_start_byte: usize,
        item: DisplayItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextConsumedDisplayItem> {
        let alignment = BufferTextSourceAlignmentRequest::for_position(text_start_byte, *position);
        let (start_byte_idx, start_charpos) = alignment.split_text_item_start(&item)?;
        let ch = match &item.kind {
            DisplayItemKind::TextRun(run) => {
                let mut chars = run.text.chars();
                let ch = chars.next()?;
                if chars.next().is_some() {
                    tracing::error!(
                        "BufferTextSourceConsumptionState: split text run yielded multiple chars"
                    );
                    return None;
                }
                ch
            }
            _ => {
                tracing::error!(
                    "BufferTextSourceConsumptionState: split text run yielded non-text item"
                );
                return None;
            }
        };
        let end_byte_idx = alignment.split_text_item_end_byte_idx(&item)?;
        position.advance_byte_idx_to(end_byte_idx);
        Some(BufferTextConsumedDisplayItem::new(
            BufferTextSourceStepChar::new(ch, start_byte_idx, start_charpos),
            item,
        ))
    }
}
