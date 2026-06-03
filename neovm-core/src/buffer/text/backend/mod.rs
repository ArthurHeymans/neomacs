mod gap;
mod piece;

use std::fmt;

use super::BufferTextBackendKind;
use crate::buffer::buffer_text::TextBackendDebugLayout;
use crate::buffer::position::TextPositionAnchor;
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

    pub(in crate::buffer) fn byte_to_char(&self, byte_pos: usize) -> usize {
        match self {
            Self::Gap(gap) => gap.byte_to_char(byte_pos),
            Self::PieceTree(piece_tree) => piece_tree.byte_to_char(byte_pos),
        }
    }

    pub(in crate::buffer) fn char_to_byte(&self, char_pos: usize) -> usize {
        match self {
            Self::Gap(gap) => gap.char_to_byte(char_pos),
            Self::PieceTree(piece_tree) => piece_tree.char_to_byte(char_pos),
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

    pub(in crate::buffer) fn text_range(&self, start: usize, end: usize) -> String {
        match self {
            Self::Gap(gap) => gap.text_range(start, end),
            Self::PieceTree(piece_tree) => piece_tree.text_range(start, end),
        }
    }

    pub(in crate::buffer) fn copy_bytes_to(&self, start: usize, end: usize, out: &mut Vec<u8>) {
        match self {
            Self::Gap(gap) => gap.copy_bytes_to(start, end, out),
            Self::PieceTree(piece_tree) => piece_tree.copy_bytes_to(start, end, out),
        }
    }

    pub(in crate::buffer) fn copy_emacs_bytes_to(
        &self,
        start: usize,
        end: usize,
        out: &mut Vec<u8>,
    ) {
        match self {
            Self::Gap(gap) => gap.copy_emacs_bytes_to(start, end, out),
            Self::PieceTree(piece_tree) => piece_tree.copy_emacs_bytes_to(start, end, out),
        }
    }

    pub(in crate::buffer) fn for_each_emacs_byte_chunk<E>(
        &self,
        start: usize,
        end: usize,
        f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Gap(gap) => gap.for_each_emacs_byte_chunk(start, end, f),
            Self::PieceTree(piece_tree) => piece_tree.for_each_emacs_byte_chunk(start, end, f),
        }
    }

    pub(in crate::buffer) fn has_contiguous_emacs_bytes(&self, start: usize, end: usize) -> bool {
        match self {
            Self::Gap(gap) => gap.has_contiguous_emacs_bytes(start, end),
            Self::PieceTree(piece_tree) => piece_tree.has_contiguous_emacs_bytes(start, end),
        }
    }

    pub(in crate::buffer) fn with_contiguous_emacs_bytes<R>(
        &self,
        start: usize,
        end: usize,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Option<R> {
        match self {
            Self::Gap(gap) => gap.with_contiguous_emacs_bytes(start, end, f),
            Self::PieceTree(piece_tree) => piece_tree.with_contiguous_emacs_bytes(start, end, f),
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

    pub(in crate::buffer) fn insert_emacs_bytes_both(
        &mut self,
        pos: usize,
        bytes: &[u8],
        nchars: usize,
    ) {
        match self {
            Self::Gap(gap) => gap.insert_emacs_bytes_both(pos, bytes, nchars),
            Self::PieceTree(piece_tree) => piece_tree.insert_emacs_bytes_both(pos, bytes, nchars),
        }
    }

    pub(in crate::buffer) fn delete_range(&mut self, start: usize, end: usize) {
        match self {
            Self::Gap(gap) => gap.delete_range(start, end),
            Self::PieceTree(piece_tree) => piece_tree.delete_range(start, end),
        }
    }

    pub(in crate::buffer) fn delete_range_both(&mut self, start: usize, end: usize, nchars: usize) {
        match self {
            Self::Gap(gap) => gap.delete_range_both(start, end, nchars),
            Self::PieceTree(piece_tree) => piece_tree.delete_range_both(start, end, nchars),
        }
    }

    pub(in crate::buffer) fn replace_same_len_emacs_bytes(
        &mut self,
        start: usize,
        end: usize,
        replacement: &[u8],
    ) {
        match self {
            Self::Gap(gap) => gap.replace_same_len_emacs_bytes(start, end, replacement),
            Self::PieceTree(piece_tree) => {
                piece_tree.replace_same_len_emacs_bytes(start, end, replacement)
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
