#[cfg(test)]
mod conformance;
mod gap;
mod piece_tree;
mod rope;

use std::fmt;

use super::ImplementedBufferTextBackendKind;
use crate::buffer::position::{CharPos0, EmacsBytePos, EmacsByteRange, TextPositionAnchor};
#[cfg(test)]
use crate::buffer::text::TextBackendDebugLayout;
use crate::buffer::text::{
    GapCompatState, TextEditRange, TextExtent, TextMetrics, TextReplacement,
};
use gap::GapTextBackend;
use piece_tree::PieceTreeTextBackend;
use rope::RopeTextBackend;

/// Physical storage for buffer text.
///
/// This enum is intentionally private to the buffer module.  `BufferText`
/// owns GNU-visible text semantics such as markers, text properties, ticks,
/// narrowing interactions, and cache invalidation.  Concrete backends own only
/// byte storage and backend-local lookup hints.
#[derive(Clone)]
pub(in crate::buffer) enum TextBackend {
    Gap(GapTextBackend),
    PieceTree(PieceTreeTextBackend),
    Rope(RopeTextBackend),
}

impl TextBackend {
    pub(in crate::buffer) fn new(kind: ImplementedBufferTextBackendKind) -> Self {
        match kind {
            ImplementedBufferTextBackendKind::GapBuffer => Self::Gap(GapTextBackend::new()),
            ImplementedBufferTextBackendKind::PieceTree => {
                Self::PieceTree(PieceTreeTextBackend::new())
            }
            ImplementedBufferTextBackendKind::Rope => Self::Rope(RopeTextBackend::new()),
        }
    }

    pub(in crate::buffer) fn from_str(text: &str, kind: ImplementedBufferTextBackendKind) -> Self {
        match kind {
            ImplementedBufferTextBackendKind::GapBuffer => {
                Self::Gap(GapTextBackend::from_str(text))
            }
            ImplementedBufferTextBackendKind::PieceTree => {
                Self::PieceTree(PieceTreeTextBackend::from_str(text))
            }
            ImplementedBufferTextBackendKind::Rope => Self::Rope(RopeTextBackend::from_str(text)),
        }
    }

    pub(in crate::buffer) fn from_emacs_bytes(
        bytes: &[u8],
        multibyte: bool,
        kind: ImplementedBufferTextBackendKind,
    ) -> Self {
        match kind {
            ImplementedBufferTextBackendKind::GapBuffer => {
                Self::Gap(GapTextBackend::from_emacs_bytes(bytes, multibyte))
            }
            ImplementedBufferTextBackendKind::PieceTree => {
                Self::PieceTree(PieceTreeTextBackend::from_emacs_bytes(bytes, multibyte))
            }
            ImplementedBufferTextBackendKind::Rope => {
                Self::Rope(RopeTextBackend::from_emacs_bytes(bytes, multibyte))
            }
        }
    }

    pub(in crate::buffer) fn from_dump(
        text: Vec<u8>,
        multibyte: bool,
        kind: ImplementedBufferTextBackendKind,
    ) -> Self {
        match kind {
            ImplementedBufferTextBackendKind::GapBuffer => {
                Self::Gap(GapTextBackend::from_dump(text, multibyte))
            }
            ImplementedBufferTextBackendKind::PieceTree => {
                Self::PieceTree(PieceTreeTextBackend::from_dump(text, multibyte))
            }
            ImplementedBufferTextBackendKind::Rope => {
                Self::Rope(RopeTextBackend::from_dump(text, multibyte))
            }
        }
    }

    pub(in crate::buffer) fn kind(&self) -> ImplementedBufferTextBackendKind {
        match self {
            Self::Gap(_) => ImplementedBufferTextBackendKind::GapBuffer,
            Self::PieceTree(_) => ImplementedBufferTextBackendKind::PieceTree,
            Self::Rope(_) => ImplementedBufferTextBackendKind::Rope,
        }
    }

    #[cfg(test)]
    pub(in crate::buffer) fn debug_layout(&self) -> TextBackendDebugLayout {
        match self {
            Self::Gap(gap) => TextBackendDebugLayout::Gap(gap.debug_layout()),
            Self::PieceTree(piece_tree) => piece_tree.debug_layout(),
            Self::Rope(rope) => rope.debug_layout(),
        }
    }

    pub(in crate::buffer) fn metrics(&self) -> TextMetrics {
        match self {
            Self::Gap(gap) => gap.metrics(),
            Self::PieceTree(piece_tree) => piece_tree.metrics(),
            Self::Rope(rope) => rope.metrics(),
        }
    }

    pub(in crate::buffer) fn storage_conversion_anchor(&self) -> Option<TextPositionAnchor> {
        match self {
            Self::Gap(gap) => Some(gap.storage_conversion_anchor()),
            Self::PieceTree(_) => None,
            Self::Rope(_) => None,
        }
    }

    pub(in crate::buffer) fn real_gap_compat_state(&self) -> Option<GapCompatState> {
        match self {
            Self::Gap(gap) => Some(gap.real_gap_compat_state()),
            Self::PieceTree(_) | Self::Rope(_) => None,
        }
    }

    pub(in crate::buffer) fn is_multibyte(&self) -> bool {
        match self {
            Self::Gap(gap) => gap.is_multibyte(),
            Self::PieceTree(piece_tree) => piece_tree.is_multibyte(),
            Self::Rope(rope) => rope.is_multibyte(),
        }
    }

    pub(in crate::buffer) fn set_multibyte(&mut self, multibyte: bool) {
        match self {
            Self::Gap(gap) => gap.set_multibyte(multibyte),
            Self::PieceTree(piece_tree) => piece_tree.set_multibyte(multibyte),
            Self::Rope(rope) => rope.set_multibyte(multibyte),
        }
    }

    pub(in crate::buffer) fn byte_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> u8 {
        match self {
            Self::Gap(gap) => gap.byte_at_emacs_byte_pos(pos),
            Self::PieceTree(piece_tree) => piece_tree.byte_at_emacs_byte_pos(pos),
            Self::Rope(rope) => rope.byte_at_emacs_byte_pos(pos),
        }
    }

    pub(in crate::buffer) fn emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8> {
        match self {
            Self::Gap(gap) => gap.emacs_byte_at_pos(pos),
            Self::PieceTree(piece_tree) => piece_tree.emacs_byte_at_pos(pos),
            Self::Rope(rope) => rope.emacs_byte_at_pos(pos),
        }
    }

    pub(in crate::buffer) fn char_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<char> {
        match self {
            Self::Gap(gap) => gap.char_at_emacs_byte_pos(pos),
            Self::PieceTree(piece_tree) => piece_tree.char_at_emacs_byte_pos(pos),
            Self::Rope(rope) => rope.char_at_emacs_byte_pos(pos),
        }
    }

    pub(in crate::buffer) fn char_code_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<u32> {
        match self {
            Self::Gap(gap) => gap.char_code_at_emacs_byte_pos(pos),
            Self::PieceTree(piece_tree) => piece_tree.char_code_at_emacs_byte_pos(pos),
            Self::Rope(rope) => rope.char_code_at_emacs_byte_pos(pos),
        }
    }

    pub(in crate::buffer) fn emacs_byte_pos_to_char_pos(&self, byte_pos: EmacsBytePos) -> CharPos0 {
        match self {
            Self::Gap(gap) => gap.emacs_byte_pos_to_char_pos(byte_pos),
            Self::PieceTree(piece_tree) => piece_tree.emacs_byte_pos_to_char_pos(byte_pos),
            Self::Rope(rope) => rope.emacs_byte_pos_to_char_pos(byte_pos),
        }
    }

    pub(in crate::buffer) fn char_pos_to_emacs_byte_pos(&self, char_pos: CharPos0) -> EmacsBytePos {
        match self {
            Self::Gap(gap) => gap.char_pos_to_emacs_byte_pos(char_pos),
            Self::PieceTree(piece_tree) => piece_tree.char_pos_to_emacs_byte_pos(char_pos),
            Self::Rope(rope) => rope.char_pos_to_emacs_byte_pos(char_pos),
        }
    }

    pub(in crate::buffer) fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String {
        match self {
            Self::Gap(gap) => gap.text_emacs_byte_range(range),
            Self::PieceTree(piece_tree) => piece_tree.text_emacs_byte_range(range),
            Self::Rope(rope) => rope.text_emacs_byte_range(range),
        }
    }

    pub(in crate::buffer) fn copy_emacs_byte_range_to(
        &self,
        range: EmacsByteRange,
        out: &mut Vec<u8>,
    ) {
        match self {
            Self::Gap(gap) => gap.copy_emacs_byte_range_to(range, out),
            Self::PieceTree(piece_tree) => piece_tree.copy_emacs_byte_range_to(range, out),
            Self::Rope(rope) => rope.copy_emacs_byte_range_to(range, out),
        }
    }

    pub(in crate::buffer) fn for_each_emacs_byte_range_chunk<E>(
        &self,
        range: EmacsByteRange,
        f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Gap(gap) => gap.for_each_emacs_byte_range_chunk(range, f),
            Self::PieceTree(piece_tree) => piece_tree.for_each_emacs_byte_range_chunk(range, f),
            Self::Rope(rope) => rope.for_each_emacs_byte_range_chunk(range, f),
        }
    }

    pub(in crate::buffer) fn has_contiguous_emacs_byte_range(&self, range: EmacsByteRange) -> bool {
        match self {
            Self::Gap(gap) => gap.has_contiguous_emacs_byte_range(range),
            Self::PieceTree(piece_tree) => piece_tree.has_contiguous_emacs_byte_range(range),
            Self::Rope(rope) => rope.has_contiguous_emacs_byte_range(range),
        }
    }

    pub(in crate::buffer) fn with_contiguous_emacs_byte_range<R>(
        &self,
        range: EmacsByteRange,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Option<R> {
        match self {
            Self::Gap(gap) => gap.with_contiguous_emacs_byte_range(range, f),
            Self::PieceTree(piece_tree) => piece_tree.with_contiguous_emacs_byte_range(range, f),
            Self::Rope(rope) => rope.with_contiguous_emacs_byte_range(range, f),
        }
    }

    pub(in crate::buffer) fn insert_measured_emacs_bytes(
        &mut self,
        pos: EmacsBytePos,
        bytes: &[u8],
        extent: TextExtent,
    ) {
        match self {
            Self::Gap(gap) => gap.insert_measured_emacs_bytes(pos, bytes, extent),
            Self::PieceTree(piece_tree) => {
                piece_tree.insert_measured_emacs_bytes(pos, bytes, extent)
            }
            Self::Rope(rope) => rope.insert_measured_emacs_bytes(pos, bytes, extent),
        }
    }

    pub(in crate::buffer) fn delete_measured_range(&mut self, range: TextEditRange) {
        match self {
            Self::Gap(gap) => gap.delete_measured_range(range),
            Self::PieceTree(piece_tree) => piece_tree.delete_measured_range(range),
            Self::Rope(rope) => rope.delete_measured_range(range),
        }
    }

    pub(in crate::buffer) fn replace_measured_range(
        &mut self,
        replacement: TextReplacement,
        bytes: &[u8],
    ) {
        match self {
            Self::Gap(gap) => gap.replace_measured_range(replacement, bytes),
            Self::PieceTree(piece_tree) => piece_tree.replace_measured_range(replacement, bytes),
            Self::Rope(rope) => rope.replace_measured_range(replacement, bytes),
        }
    }

    pub(in crate::buffer) fn replace_same_len_emacs_byte_range(
        &mut self,
        range: EmacsByteRange,
        replacement: &[u8],
    ) {
        match self {
            Self::Gap(gap) => gap.replace_same_len_emacs_byte_range(range, replacement),
            Self::PieceTree(piece_tree) => {
                piece_tree.replace_same_len_emacs_byte_range(range, replacement)
            }
            Self::Rope(rope) => rope.replace_same_len_emacs_byte_range(range, replacement),
        }
    }

    pub(in crate::buffer) fn dump_text(&self) -> Vec<u8> {
        match self {
            Self::Gap(gap) => gap.dump_text(),
            Self::PieceTree(piece_tree) => piece_tree.dump_text(),
            Self::Rope(rope) => rope.dump_text(),
        }
    }
}

impl fmt::Display for TextBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gap(gap) => gap.fmt(f),
            Self::PieceTree(piece_tree) => piece_tree.fmt(f),
            Self::Rope(rope) => rope.fmt(f),
        }
    }
}
