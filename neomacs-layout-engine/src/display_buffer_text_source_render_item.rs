//! Buffer text render items produced by the typed buffer source.

use crate::display_buffer_text_source::BufferTextSourcePosition;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplaySourcePosition, RenderFaceRef, SourceSpan,
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

pub(crate) fn direct_text_run_char_item(
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
