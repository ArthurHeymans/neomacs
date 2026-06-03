mod gap;
mod piece;

use std::fmt;

use super::BufferTextBackendKind;
use crate::buffer::buffer_text::TextBackendDebugLayout;
use crate::buffer::position::{CharPos0, EmacsBytePos, EmacsByteRange, TextPositionAnchor};
use crate::buffer::text::{TextEditRange, TextExtent};
use gap::GapTextBackend;
use piece::PieceTreeTextBackend;

#[derive(Clone)]
pub(in crate::buffer) enum TextBackend {
    Gap(GapTextBackend),
    PieceTree(PieceTreeTextBackend),
}

impl TextBackend {
    pub(in crate::buffer) fn new_gap() -> Self {
        Self::Gap(GapTextBackend::new())
    }

    pub(in crate::buffer) fn from_str_gap(text: &str) -> Self {
        Self::Gap(GapTextBackend::from_str(text))
    }

    pub(in crate::buffer) fn from_emacs_bytes_gap(bytes: &[u8], multibyte: bool) -> Self {
        Self::Gap(GapTextBackend::from_emacs_bytes(bytes, multibyte))
    }

    pub(in crate::buffer) fn from_dump_gap(text: Vec<u8>, multibyte: bool) -> Self {
        Self::Gap(GapTextBackend::from_dump(text, multibyte))
    }

    pub(in crate::buffer) fn new_piece_tree() -> Self {
        Self::PieceTree(PieceTreeTextBackend::new())
    }

    pub(in crate::buffer) fn from_str_piece_tree(text: &str) -> Self {
        Self::PieceTree(PieceTreeTextBackend::from_str(text))
    }

    pub(in crate::buffer) fn from_emacs_bytes_piece_tree(bytes: &[u8], multibyte: bool) -> Self {
        Self::PieceTree(PieceTreeTextBackend::from_emacs_bytes(bytes, multibyte))
    }

    pub(in crate::buffer) fn from_dump_piece_tree(text: Vec<u8>, multibyte: bool) -> Self {
        Self::PieceTree(PieceTreeTextBackend::from_dump(text, multibyte))
    }

    pub(in crate::buffer) fn kind(&self) -> BufferTextBackendKind {
        match self {
            Self::Gap(_) => BufferTextBackendKind::GapBuffer,
            Self::PieceTree(piece_tree) => piece_tree.kind(),
        }
    }

    pub(in crate::buffer) fn debug_layout(&self) -> TextBackendDebugLayout {
        match self {
            Self::Gap(gap) => TextBackendDebugLayout::Gap(gap.debug_layout()),
            Self::PieceTree(piece_tree) => piece_tree.debug_layout(),
        }
    }

    pub(in crate::buffer) fn conversion_anchor(&self) -> Option<TextPositionAnchor> {
        match self {
            Self::Gap(gap) => Some(TextPositionAnchor::new(gap.gpt(), gap.gpt_byte())),
            Self::PieceTree(_) => None,
        }
    }

    pub(in crate::buffer) fn len(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.len(),
            Self::PieceTree(piece_tree) => piece_tree.len(),
        }
    }

    pub(in crate::buffer) fn is_empty(&self) -> bool {
        match self {
            Self::Gap(gap) => gap.is_empty(),
            Self::PieceTree(piece_tree) => piece_tree.is_empty(),
        }
    }

    pub(in crate::buffer) fn is_multibyte(&self) -> bool {
        match self {
            Self::Gap(gap) => gap.is_multibyte(),
            Self::PieceTree(piece_tree) => piece_tree.is_multibyte(),
        }
    }

    pub(in crate::buffer) fn set_multibyte(&mut self, multibyte: bool) {
        match self {
            Self::Gap(gap) => gap.set_multibyte(multibyte),
            Self::PieceTree(piece_tree) => piece_tree.set_multibyte(multibyte),
        }
    }

    pub(in crate::buffer) fn byte_at(&self, pos: usize) -> u8 {
        match self {
            Self::Gap(gap) => gap.byte_at(pos),
            Self::PieceTree(piece_tree) => piece_tree.byte_at(pos),
        }
    }

    pub(in crate::buffer) fn emacs_byte_at(&self, pos: usize) -> Option<u8> {
        match self {
            Self::Gap(gap) => gap.emacs_byte_at(pos),
            Self::PieceTree(piece_tree) => piece_tree.emacs_byte_at(pos),
        }
    }

    pub(in crate::buffer) fn char_at(&self, pos: usize) -> Option<char> {
        match self {
            Self::Gap(gap) => gap.char_at(pos),
            Self::PieceTree(piece_tree) => piece_tree.char_at(pos),
        }
    }

    pub(in crate::buffer) fn char_code_at(&self, pos: usize) -> Option<u32> {
        match self {
            Self::Gap(gap) => gap.char_code_at(pos),
            Self::PieceTree(piece_tree) => piece_tree.char_code_at(pos),
        }
    }

    pub(in crate::buffer) fn emacs_byte_pos_to_char_pos(&self, byte_pos: EmacsBytePos) -> CharPos0 {
        match self {
            Self::Gap(gap) => gap.emacs_byte_pos_to_char_pos(byte_pos),
            Self::PieceTree(piece_tree) => piece_tree.emacs_byte_pos_to_char_pos(byte_pos),
        }
    }

    pub(in crate::buffer) fn char_pos_to_emacs_byte_pos(&self, char_pos: CharPos0) -> EmacsBytePos {
        match self {
            Self::Gap(gap) => gap.char_pos_to_emacs_byte_pos(char_pos),
            Self::PieceTree(piece_tree) => piece_tree.char_pos_to_emacs_byte_pos(char_pos),
        }
    }

    pub(in crate::buffer) fn storage_byte_to_emacs_byte(&self, byte_pos: usize) -> usize {
        match self {
            Self::Gap(gap) => gap.storage_byte_to_emacs_byte(byte_pos),
            Self::PieceTree(piece_tree) => piece_tree.storage_byte_to_emacs_byte(byte_pos),
        }
    }

    pub(in crate::buffer) fn emacs_byte_to_storage_byte(&self, byte_pos: usize) -> usize {
        match self {
            Self::Gap(gap) => gap.emacs_byte_to_storage_byte(byte_pos),
            Self::PieceTree(piece_tree) => piece_tree.emacs_byte_to_storage_byte(byte_pos),
        }
    }

    pub(in crate::buffer) fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String {
        match self {
            Self::Gap(gap) => gap.text_emacs_byte_range(range),
            Self::PieceTree(piece_tree) => piece_tree.text_emacs_byte_range(range),
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
        }
    }

    pub(in crate::buffer) fn has_contiguous_emacs_byte_range(&self, range: EmacsByteRange) -> bool {
        match self {
            Self::Gap(gap) => gap.has_contiguous_emacs_byte_range(range),
            Self::PieceTree(piece_tree) => piece_tree.has_contiguous_emacs_byte_range(range),
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
        }
    }

    pub(in crate::buffer) fn insert_str(&mut self, pos: usize, text: &str) {
        match self {
            Self::Gap(gap) => gap.insert_str(pos, text),
            Self::PieceTree(piece_tree) => piece_tree.insert_str(pos, text),
        }
    }

    pub(in crate::buffer) fn insert_emacs_bytes(&mut self, pos: usize, bytes: &[u8]) {
        match self {
            Self::Gap(gap) => gap.insert_emacs_bytes(pos, bytes),
            Self::PieceTree(piece_tree) => piece_tree.insert_emacs_bytes(pos, bytes),
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
        }
    }

    pub(in crate::buffer) fn delete_range(&mut self, start: usize, end: usize) {
        match self {
            Self::Gap(gap) => gap.delete_range(start, end),
            Self::PieceTree(piece_tree) => piece_tree.delete_range(start, end),
        }
    }

    pub(in crate::buffer) fn delete_measured_range(&mut self, range: TextEditRange) {
        match self {
            Self::Gap(gap) => gap.delete_measured_range(range),
            Self::PieceTree(piece_tree) => piece_tree.delete_measured_range(range),
        }
    }

    pub(in crate::buffer) fn replace_same_len_emacs_bytes(
        &mut self,
        range: EmacsByteRange,
        replacement: &[u8],
    ) {
        match self {
            Self::Gap(gap) => gap.replace_same_len_emacs_byte_range(range, replacement),
            Self::PieceTree(piece_tree) => {
                piece_tree.replace_same_len_emacs_byte_range(range, replacement)
            }
        }
    }

    pub(in crate::buffer) fn dump_text(&self) -> Vec<u8> {
        match self {
            Self::Gap(gap) => gap.dump_text(),
            Self::PieceTree(piece_tree) => piece_tree.dump_text(),
        }
    }
}

impl fmt::Display for TextBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gap(gap) => gap.fmt(f),
            Self::PieceTree(piece_tree) => piece_tree.fmt(f),
        }
    }
}
