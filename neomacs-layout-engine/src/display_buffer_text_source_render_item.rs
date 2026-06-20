//! Buffer text render items produced by the typed buffer source.

use crate::display_buffer_text_source::BufferTextSourcePosition;
use crate::display_buffer_text_source_consumption::BufferTextSourceItem;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayRowBreakReason, DisplaySourcePosition, RenderFaceRef,
    SourceSpan,
};
use crate::unicode::decode_utf8;
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos};

/// A single source character aligned with the current buffer byte and char
/// positions for the row walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourceStepChar {
    ch: char,
    start_byte_idx: usize,
    start_charpos: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextDirectDisplayItem {
    source_char: BufferTextSourceStepChar,
    item: DisplayItem,
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

impl BufferTextDirectDisplayItem {
    pub(crate) fn new(source_char: BufferTextSourceStepChar, item: DisplayItem) -> Self {
        Self { source_char, item }
    }

    pub(crate) fn consume_source_item(
        item: BufferTextSourceItem,
        position: &mut BufferTextSourcePosition,
    ) -> Result<Self, BufferTextSourceItem> {
        if !position.matches(item.start_byte_idx(), item.start_charpos()) {
            tracing::error!(
                "BufferTextDirectDisplayItem: validated source item at byte {} charpos {} \
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
        Ok(Self::new(
            BufferTextSourceStepChar::new(ch, start_byte_idx, start_charpos),
            item.into_item(),
        ))
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

    #[cfg(test)]
    pub(crate) fn end_charpos(&self) -> i64 {
        display_item_buffer_end_charpos(&self.item)
            .unwrap_or_else(|| self.source_char.start_charpos().saturating_add(1))
    }

    pub(crate) fn end_byte_idx(&self, text_start_byte: usize) -> Option<usize> {
        display_item_buffer_end_byte_idx(&self.item, text_start_byte)
    }

    pub(crate) fn is_multi_char_text_run(&self) -> bool {
        let DisplayItemKind::TextRun(run) = &self.item.kind else {
            return false;
        };
        let mut chars = run.text.chars();
        chars.next().is_some() && chars.next().is_some()
    }

    pub(crate) fn split_text_run_items(
        self,
        text_start_byte: usize,
    ) -> Option<(Self, Vec<BufferTextDirectDisplayItem>)> {
        if !self.is_multi_char_text_run() {
            return None;
        }
        let DisplayItem {
            span,
            face,
            kind,
            layout,
        } = self.item;
        let DisplayItemKind::TextRun(run) = kind else {
            return None;
        };
        let DisplaySourcePosition::Buffer { buffer_id, .. } = span.start else {
            return None;
        };
        let mut byte_idx = self.source_char.start_byte_idx();
        let mut charpos = self.source_char.start_charpos();
        let mut items = Vec::new();
        for ch in run.text.chars() {
            let ch_len = ch.len_utf8();
            let item = direct_text_run_char_item(
                buffer_id,
                face,
                layout,
                text_start_byte,
                byte_idx,
                charpos,
                ch,
            );
            items.push(BufferTextDirectDisplayItem::new(
                BufferTextSourceStepChar::new(ch, byte_idx, charpos),
                item,
            ));
            byte_idx = byte_idx.saturating_add(ch_len);
            charpos = charpos.saturating_add(1);
        }
        if items.len() <= 1 {
            return None;
        }
        let mut iter = items.into_iter();
        let first = iter.next()?;
        let pending = iter.collect();
        Some((first, pending))
    }

    pub(crate) fn into_parts(self) -> (BufferTextSourceStepChar, DisplayItem) {
        (self.source_char, self.item)
    }
}

fn direct_text_run_char_item(
    buffer_id: BufferId,
    face: RenderFaceRef,
    layout: crate::display_item::DisplayItemLayout,
    text_start_byte: usize,
    start_byte_idx: usize,
    start_charpos: i64,
    ch: char,
) -> DisplayItem {
    let end_byte_idx = start_byte_idx.saturating_add(ch.len_utf8());
    let end_charpos = start_charpos.saturating_add(1);
    DisplayItem::new(
        SourceSpan::new(
            DisplaySourcePosition::buffer(
                buffer_id,
                CharPos0::new(start_charpos.max(0) as usize),
                EmacsBytePos::new(text_start_byte.saturating_add(start_byte_idx)),
            ),
            DisplaySourcePosition::buffer(
                buffer_id,
                CharPos0::new(end_charpos.max(0) as usize),
                EmacsBytePos::new(text_start_byte.saturating_add(end_byte_idx)),
            ),
        ),
        face,
        DisplayItemKind::TextRun(crate::display_item::DisplayTextRun::new(ch.to_string())),
    )
    .with_layout(layout)
}

#[cfg(test)]
fn display_item_buffer_end_charpos(item: &DisplayItem) -> Option<i64> {
    item.span
        .buffer_end_charpos()
        .map(|char_pos| char_pos.get() as i64)
}

fn display_item_buffer_end_byte_idx(item: &DisplayItem, text_start_byte: usize) -> Option<usize> {
    let DisplaySourcePosition::Buffer {
        byte_pos: end_byte_pos,
        ..
    } = item.span.end
    else {
        return None;
    };
    end_byte_pos.get().checked_sub(text_start_byte)
}
