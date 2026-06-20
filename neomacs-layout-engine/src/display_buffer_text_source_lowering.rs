//! Buffer text compatibility lowering from typed source items to row-walk items.

use crate::display_buffer_text_source::BufferTextSourcePosition;
use crate::display_buffer_text_source_consumption::BufferTextSourceItem;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayRowBreakReason, DisplaySourcePosition,
    DisplayTextRunItemCursor,
};
use crate::unicode::decode_utf8;
use neovm_core::buffer::CharPos0;

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
pub(crate) struct BufferTextLoweredDisplayItem {
    source_char: BufferTextSourceStepChar,
    item: DisplayItem,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextDirectDisplayItem {
    source_char: BufferTextSourceStepChar,
    item: DisplayItem,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceRenderItem {
    Direct(BufferTextDirectDisplayItem),
    Lowered(BufferTextLoweredDisplayItem),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceRenderItemKind {
    Direct,
    Lowered,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceLoweringState {
    text_start_byte: usize,
    pending_text_run: Option<DisplayTextRunItemCursor>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextDirectDisplayItemRequest {
    item: BufferTextSourceItem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferTextSplitItemAlignment {
    text_start_byte: usize,
    position: BufferTextSourcePosition,
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

impl BufferTextLoweredDisplayItem {
    pub(crate) fn new(source_char: BufferTextSourceStepChar, item: DisplayItem) -> Self {
        Self { source_char, item }
    }
}

impl BufferTextDirectDisplayItem {
    pub(crate) fn new(source_char: BufferTextSourceStepChar, item: DisplayItem) -> Self {
        Self { source_char, item }
    }
}

impl BufferTextSourceRenderItem {
    pub(crate) fn kind(&self) -> BufferTextSourceRenderItemKind {
        match self {
            Self::Direct(_) => BufferTextSourceRenderItemKind::Direct,
            Self::Lowered(_) => BufferTextSourceRenderItemKind::Lowered,
        }
    }

    pub(crate) fn source_char(&self) -> BufferTextSourceStepChar {
        match self {
            Self::Direct(item) => item.source_char,
            Self::Lowered(item) => item.source_char,
        }
    }

    pub(crate) fn is_explicit_line_break(&self) -> bool {
        let item = match self {
            Self::Direct(item) => &item.item,
            Self::Lowered(item) => &item.item,
        };
        matches!(
            item.kind,
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline
        )
    }

    pub(crate) fn end_charpos(&self) -> i64 {
        let (source_char, item) = match self {
            Self::Direct(item) => (item.source_char, &item.item),
            Self::Lowered(item) => (item.source_char, &item.item),
        };
        display_item_buffer_end_charpos(item)
            .unwrap_or_else(|| source_char.start_charpos().saturating_add(1))
    }

    pub(crate) fn into_parts(self) -> (BufferTextSourceStepChar, DisplayItem) {
        match self {
            Self::Direct(item) => (item.source_char, item.item),
            Self::Lowered(item) => (item.source_char, item.item),
        }
    }
}

impl BufferTextSourceLoweringState {
    pub(crate) fn new(text_start_byte: usize) -> Self {
        Self {
            text_start_byte,
            pending_text_run: None,
        }
    }

    pub(crate) fn has_pending_text_run(&self) -> bool {
        self.pending_text_run.is_some()
    }

    pub(crate) fn next_pending_display_item(
        &mut self,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceRenderItem> {
        let text_start_byte = self.text_start_byte;
        let pending = self.pending_text_run.as_mut()?;
        let item = pending.next_item();
        let finished = pending.is_finished();
        let step = item.and_then(|item| {
            Self::lowered_display_item_from_split_text_item(text_start_byte, item, position)
        });
        if finished || step.is_none() {
            self.pending_text_run = None;
        }
        step.map(BufferTextSourceRenderItem::Lowered)
    }

    pub(crate) fn consume_text_run_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceRenderItem> {
        if !position.matches(item.start_byte_idx(), item.start_charpos()) {
            tracing::error!(
                "BufferTextSourceLoweringState: validated source item at byte {} charpos {} \
                 did not match buffer walk byte {} charpos {}",
                item.start_byte_idx(),
                item.start_charpos(),
                position.byte_idx(),
                position.charpos()
            );
            return None;
        }
        self.split_text_run_item(item, position)
    }
}

impl BufferTextDirectDisplayItemRequest {
    pub(crate) fn new(item: BufferTextSourceItem) -> Self {
        Self { item }
    }

    pub(crate) fn consume(
        self,
        position: &mut BufferTextSourcePosition,
    ) -> Result<BufferTextSourceRenderItem, BufferTextSourceItem> {
        Self::try_into_direct_display_item(self.item, position)
            .map(BufferTextSourceRenderItem::Direct)
    }

    fn try_into_direct_display_item(
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Result<BufferTextDirectDisplayItem, BufferTextSourceItem> {
        if !position.matches(item.start_byte_idx(), item.start_charpos()) {
            tracing::error!(
                "BufferTextDirectDisplayItemRequest: validated source item at byte {} charpos {} \
                 did not match buffer walk byte {} charpos {}",
                item.start_byte_idx(),
                item.start_charpos(),
                position.byte_idx(),
                position.charpos()
            );
            return Err(item);
        }
        let Some(ch) = item.direct_source_char() else {
            return Err(item);
        };
        let start_byte_idx = item.start_byte_idx();
        let start_charpos = item.start_charpos();
        let byte_len = item.buffer_byte_len().unwrap_or_else(|| ch.len_utf8());
        position.advance_byte_idx_to(start_byte_idx.saturating_add(byte_len));
        Ok(BufferTextDirectDisplayItem::new(
            BufferTextSourceStepChar::new(ch, start_byte_idx, start_charpos),
            item.into_item(),
        ))
    }
}

impl BufferTextSourceLoweringState {
    fn split_text_run_item(
        &mut self,
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextSourceRenderItem> {
        match DisplayTextRunItemCursor::from_item(item.into_item()) {
            Ok(cursor) => {
                self.pending_text_run = Some(cursor);
                self.next_pending_display_item(position)
            }
            Err(_) => {
                tracing::error!(
                    "BufferTextSourceLoweringState: source cursor yielded a non-text item kind; \
                     a direct item escaped source-item lowering"
                );
                None
            }
        }
    }

    fn lowered_display_item_from_split_text_item(
        text_start_byte: usize,
        item: DisplayItem,
        position: &mut BufferTextSourcePosition,
    ) -> Option<BufferTextLoweredDisplayItem> {
        let alignment = BufferTextSplitItemAlignment::for_position(text_start_byte, *position);
        let (start_byte_idx, start_charpos) = alignment.split_text_item_start(&item)?;
        let ch = match &item.kind {
            DisplayItemKind::TextRun(run) => {
                let mut chars = run.text.chars();
                let ch = chars.next()?;
                if chars.next().is_some() {
                    tracing::error!(
                        "BufferTextSourceLoweringState: split text run yielded multiple chars"
                    );
                    return None;
                }
                ch
            }
            _ => {
                tracing::error!(
                    "BufferTextSourceLoweringState: split text run yielded non-text item"
                );
                return None;
            }
        };
        let end_byte_idx = alignment.split_text_item_end_byte_idx(&item)?;
        position.advance_byte_idx_to(end_byte_idx);
        Some(BufferTextLoweredDisplayItem::new(
            BufferTextSourceStepChar::new(ch, start_byte_idx, start_charpos),
            item,
        ))
    }
}

impl BufferTextSplitItemAlignment {
    fn for_position(text_start_byte: usize, position: BufferTextSourcePosition) -> Self {
        Self {
            text_start_byte,
            position,
        }
    }

    fn split_text_item_start(self, item: &DisplayItem) -> Option<(usize, i64)> {
        let DisplaySourcePosition::Buffer {
            char_pos, byte_pos, ..
        } = &item.span.start
        else {
            tracing::error!(
                "BufferTextSourceLoweringState: split text run yielded a non-buffer-span item"
            );
            return None;
        };
        let start_byte_idx = byte_pos.get().checked_sub(self.text_start_byte)?;
        let start_charpos = char_pos.get() as i64;
        if !self.position.matches(start_byte_idx, start_charpos) {
            tracing::debug!(
                "BufferTextSourceLoweringState: split text run at byte {} charpos {} did not \
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
                "BufferTextSourceLoweringState: split text run yielded a non-buffer end span"
            );
            return None;
        };
        end_byte_pos.get().checked_sub(self.text_start_byte)
    }
}

fn display_item_buffer_end_charpos(item: &DisplayItem) -> Option<i64> {
    item.span
        .buffer_end_charpos()
        .map(|char_pos| char_pos.get() as i64)
}
