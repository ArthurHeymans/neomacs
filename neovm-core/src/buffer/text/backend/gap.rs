use std::fmt;

use crate::buffer::buffer_text::GapDebugLayout;
use crate::buffer::gap_buffer::GapBuffer;
use crate::buffer::position::{CharPos0, EmacsBytePos};

#[derive(Clone)]
pub(in crate::buffer) struct GapTextBackend {
    gap: GapBuffer,
}

impl GapTextBackend {
    pub(in crate::buffer) fn new() -> Self {
        Self {
            gap: GapBuffer::new(),
        }
    }

    pub(in crate::buffer) fn from_str(text: &str) -> Self {
        Self {
            gap: GapBuffer::from_str(text),
        }
    }

    pub(in crate::buffer) fn from_emacs_bytes(bytes: &[u8], multibyte: bool) -> Self {
        Self {
            gap: GapBuffer::from_emacs_bytes(bytes, multibyte),
        }
    }

    pub(in crate::buffer) fn from_dump(text: Vec<u8>, multibyte: bool) -> Self {
        Self {
            gap: GapBuffer::from_dump(text, multibyte),
        }
    }

    pub(in crate::buffer) fn len(&self) -> usize {
        self.gap.len()
    }

    pub(in crate::buffer) fn is_empty(&self) -> bool {
        self.gap.is_empty()
    }

    pub(in crate::buffer) fn is_multibyte(&self) -> bool {
        self.gap.is_multibyte()
    }

    pub(in crate::buffer) fn set_multibyte(&mut self, multibyte: bool) {
        self.gap.set_multibyte(multibyte);
    }

    pub(in crate::buffer) fn char_count(&self) -> usize {
        self.gap.char_count()
    }

    pub(in crate::buffer) fn gpt(&self) -> usize {
        self.gap.gpt()
    }

    pub(in crate::buffer) fn z(&self) -> usize {
        self.gap.z()
    }

    pub(in crate::buffer) fn gpt_byte(&self) -> usize {
        self.gap.gpt_byte()
    }

    pub(in crate::buffer) fn z_byte(&self) -> usize {
        self.gap.z_byte()
    }

    pub(in crate::buffer) fn gap_size(&self) -> usize {
        self.gap.gap_size()
    }

    pub(in crate::buffer) fn debug_layout(&self) -> GapDebugLayout {
        GapDebugLayout {
            gpt: CharPos0::new(self.gpt()),
            z: CharPos0::new(self.z()),
            gpt_byte: EmacsBytePos::new(self.gpt_byte()),
            z_byte: EmacsBytePos::new(self.z_byte()),
            gap_size: self.gap_size(),
        }
    }

    pub(in crate::buffer) fn byte_at(&self, pos: usize) -> u8 {
        self.gap.byte_at(pos)
    }

    pub(in crate::buffer) fn emacs_byte_at(&self, pos: usize) -> Option<u8> {
        self.gap.emacs_byte_at(pos)
    }

    pub(in crate::buffer) fn char_at(&self, pos: usize) -> Option<char> {
        self.gap.char_at(pos)
    }

    pub(in crate::buffer) fn char_code_at(&self, pos: usize) -> Option<u32> {
        self.gap.char_code_at(pos)
    }

    pub(in crate::buffer) fn byte_to_char(&self, byte_pos: usize) -> usize {
        self.gap.byte_to_char(byte_pos)
    }

    pub(in crate::buffer) fn char_to_byte(&self, char_pos: usize) -> usize {
        self.gap.char_to_byte(char_pos)
    }

    pub(in crate::buffer) fn storage_byte_to_emacs_byte(&self, byte_pos: usize) -> usize {
        self.gap.storage_byte_to_emacs_byte(byte_pos)
    }

    pub(in crate::buffer) fn emacs_byte_to_storage_byte(&self, byte_pos: usize) -> usize {
        self.gap.emacs_byte_to_storage_byte(byte_pos)
    }

    pub(in crate::buffer) fn text_range(&self, start: usize, end: usize) -> String {
        self.gap.text_range(start, end)
    }

    pub(in crate::buffer) fn copy_bytes_to(&self, start: usize, end: usize, out: &mut Vec<u8>) {
        self.gap.copy_bytes_to(start, end, out);
    }

    pub(in crate::buffer) fn copy_emacs_bytes_to(
        &self,
        start: usize,
        end: usize,
        out: &mut Vec<u8>,
    ) {
        self.gap.copy_emacs_bytes_to(start, end, out);
    }

    pub(in crate::buffer) fn for_each_emacs_byte_chunk<E>(
        &self,
        start: usize,
        end: usize,
        f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        self.gap.for_each_emacs_byte_chunk(start, end, f)
    }

    pub(in crate::buffer) fn has_contiguous_emacs_bytes(&self, start: usize, end: usize) -> bool {
        self.gap.has_contiguous_emacs_bytes(start, end)
    }

    pub(in crate::buffer) fn with_contiguous_emacs_bytes<R>(
        &self,
        start: usize,
        end: usize,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Option<R> {
        self.gap.with_contiguous_emacs_bytes(start, end, f)
    }

    pub(in crate::buffer) fn insert_str(&mut self, pos: usize, text: &str) {
        self.gap.insert_str(pos, text);
    }

    pub(in crate::buffer) fn insert_emacs_bytes(&mut self, pos: usize, bytes: &[u8]) {
        self.gap.insert_emacs_bytes(pos, bytes);
    }

    pub(in crate::buffer) fn insert_emacs_bytes_both(
        &mut self,
        pos: usize,
        bytes: &[u8],
        nchars: usize,
    ) {
        self.gap.insert_emacs_bytes_both(pos, bytes, nchars);
    }

    pub(in crate::buffer) fn delete_range(&mut self, start: usize, end: usize) {
        self.gap.delete_range(start, end);
    }

    pub(in crate::buffer) fn delete_range_both(&mut self, start: usize, end: usize, nchars: usize) {
        self.gap.delete_range_both(start, end, nchars);
    }

    pub(in crate::buffer) fn replace_same_len_emacs_bytes(
        &mut self,
        start: usize,
        end: usize,
        replacement: &[u8],
    ) {
        self.gap
            .replace_same_len_emacs_bytes(start, end, replacement);
    }

    pub(in crate::buffer) fn dump_text(&self) -> Vec<u8> {
        self.gap.dump_text()
    }
}

impl fmt::Display for GapTextBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.gap.fmt(f)
    }
}

impl fmt::Debug for GapTextBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GapTextBackend")
            .field("len", &self.len())
            .field("chars", &self.char_count())
            .finish()
    }
}
