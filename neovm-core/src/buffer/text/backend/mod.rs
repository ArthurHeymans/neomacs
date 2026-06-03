mod gap;

use std::fmt;

use super::BufferTextBackendKind;
use gap::GapTextBackend;

#[derive(Clone)]
pub(in crate::buffer) enum TextBackend {
    Gap(GapTextBackend),
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

    pub(in crate::buffer) fn kind(&self) -> BufferTextBackendKind {
        match self {
            Self::Gap(_) => BufferTextBackendKind::GapBuffer,
        }
    }

    pub(in crate::buffer) fn len(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.len(),
        }
    }

    pub(in crate::buffer) fn is_empty(&self) -> bool {
        match self {
            Self::Gap(gap) => gap.is_empty(),
        }
    }

    pub(in crate::buffer) fn is_multibyte(&self) -> bool {
        match self {
            Self::Gap(gap) => gap.is_multibyte(),
        }
    }

    pub(in crate::buffer) fn set_multibyte(&mut self, multibyte: bool) {
        match self {
            Self::Gap(gap) => gap.set_multibyte(multibyte),
        }
    }

    pub(in crate::buffer) fn char_count(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.char_count(),
        }
    }

    pub(in crate::buffer) fn emacs_byte_len(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.emacs_byte_len(),
        }
    }

    pub(in crate::buffer) fn gpt(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.gpt(),
        }
    }

    pub(in crate::buffer) fn z(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.z(),
        }
    }

    pub(in crate::buffer) fn gpt_byte(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.gpt_byte(),
        }
    }

    pub(in crate::buffer) fn z_byte(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.z_byte(),
        }
    }

    pub(in crate::buffer) fn gap_size(&self) -> usize {
        match self {
            Self::Gap(gap) => gap.gap_size(),
        }
    }

    pub(in crate::buffer) fn byte_at(&self, pos: usize) -> u8 {
        match self {
            Self::Gap(gap) => gap.byte_at(pos),
        }
    }

    pub(in crate::buffer) fn emacs_byte_at(&self, pos: usize) -> Option<u8> {
        match self {
            Self::Gap(gap) => gap.emacs_byte_at(pos),
        }
    }

    pub(in crate::buffer) fn char_at(&self, pos: usize) -> Option<char> {
        match self {
            Self::Gap(gap) => gap.char_at(pos),
        }
    }

    pub(in crate::buffer) fn char_code_at(&self, pos: usize) -> Option<u32> {
        match self {
            Self::Gap(gap) => gap.char_code_at(pos),
        }
    }

    pub(in crate::buffer) fn byte_to_char(&self, byte_pos: usize) -> usize {
        match self {
            Self::Gap(gap) => gap.byte_to_char(byte_pos),
        }
    }

    pub(in crate::buffer) fn char_to_byte(&self, char_pos: usize) -> usize {
        match self {
            Self::Gap(gap) => gap.char_to_byte(char_pos),
        }
    }

    pub(in crate::buffer) fn storage_byte_to_emacs_byte(&self, byte_pos: usize) -> usize {
        match self {
            Self::Gap(gap) => gap.storage_byte_to_emacs_byte(byte_pos),
        }
    }

    pub(in crate::buffer) fn emacs_byte_to_storage_byte(&self, byte_pos: usize) -> usize {
        match self {
            Self::Gap(gap) => gap.emacs_byte_to_storage_byte(byte_pos),
        }
    }

    pub(in crate::buffer) fn text_range(&self, start: usize, end: usize) -> String {
        match self {
            Self::Gap(gap) => gap.text_range(start, end),
        }
    }

    pub(in crate::buffer) fn copy_bytes_to(&self, start: usize, end: usize, out: &mut Vec<u8>) {
        match self {
            Self::Gap(gap) => gap.copy_bytes_to(start, end, out),
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
        }
    }

    pub(in crate::buffer) fn has_contiguous_emacs_bytes(&self, start: usize, end: usize) -> bool {
        match self {
            Self::Gap(gap) => gap.has_contiguous_emacs_bytes(start, end),
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
        }
    }

    pub(in crate::buffer) fn insert_str(&mut self, pos: usize, text: &str) {
        match self {
            Self::Gap(gap) => gap.insert_str(pos, text),
        }
    }

    pub(in crate::buffer) fn insert_emacs_bytes(&mut self, pos: usize, bytes: &[u8]) {
        match self {
            Self::Gap(gap) => gap.insert_emacs_bytes(pos, bytes),
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
        }
    }

    pub(in crate::buffer) fn delete_range(&mut self, start: usize, end: usize) {
        match self {
            Self::Gap(gap) => gap.delete_range(start, end),
        }
    }

    pub(in crate::buffer) fn delete_range_both(&mut self, start: usize, end: usize, nchars: usize) {
        match self {
            Self::Gap(gap) => gap.delete_range_both(start, end, nchars),
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
        }
    }

    pub(in crate::buffer) fn dump_text(&self) -> Vec<u8> {
        match self {
            Self::Gap(gap) => gap.dump_text(),
        }
    }
}

impl fmt::Display for TextBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gap(gap) => gap.fmt(f),
        }
    }
}
