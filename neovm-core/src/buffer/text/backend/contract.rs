use std::fmt;

use crate::buffer::position::{CharPos0, EmacsBytePos, EmacsByteRange, TextPositionHint};
#[cfg(test)]
use crate::buffer::text::TextBackendDebugLayout;
use crate::buffer::text::{
    GapCompatState, TextEditRange, TextExtent, TextMetrics, TextReplacement,
};

use super::gap::GapTextBackend;
use super::piece_tree::PieceTreeTextBackend;
use super::rope::RopeTextBackend;

pub(super) trait PhysicalTextBackend: fmt::Display {
    fn metrics(&self) -> TextMetrics;

    #[cfg(test)]
    fn debug_layout(&self) -> TextBackendDebugLayout;

    fn storage_position_hint(&self) -> TextPositionHint {
        TextPositionHint::none()
    }

    fn real_gap_compat_state(&self) -> Option<GapCompatState> {
        None
    }

    fn is_multibyte(&self) -> bool;
    fn set_multibyte(&mut self, multibyte: bool);
    fn byte_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> u8;
    fn emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8>;
    fn char_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<char>;
    fn char_code_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<u32>;
    fn emacs_byte_pos_to_char_pos(&self, byte_pos: EmacsBytePos) -> CharPos0;
    fn char_pos_to_emacs_byte_pos(&self, char_pos: CharPos0) -> EmacsBytePos;
    fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String;
    fn copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>);
    fn for_each_emacs_byte_range_chunk<E, F>(&self, range: EmacsByteRange, f: F) -> Result<(), E>
    where
        F: FnMut(&[u8]) -> Result<(), E>;
    fn has_contiguous_emacs_byte_range(&self, range: EmacsByteRange) -> bool;
    fn with_contiguous_emacs_byte_range<R, F>(&self, range: EmacsByteRange, f: F) -> Option<R>
    where
        F: FnOnce(&[u8]) -> R;
    fn insert_measured_emacs_bytes(&mut self, pos: EmacsBytePos, bytes: &[u8], extent: TextExtent);
    fn delete_measured_range(&mut self, range: TextEditRange);
    fn replace_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]);
    fn replace_same_len_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]);
    fn dump_text(&self) -> Vec<u8>;
}

impl PhysicalTextBackend for GapTextBackend {
    fn metrics(&self) -> TextMetrics {
        GapTextBackend::metrics(self)
    }

    #[cfg(test)]
    fn debug_layout(&self) -> TextBackendDebugLayout {
        TextBackendDebugLayout::Gap(GapTextBackend::debug_layout(self))
    }

    fn storage_position_hint(&self) -> TextPositionHint {
        GapTextBackend::storage_position_hint(self)
    }

    fn real_gap_compat_state(&self) -> Option<GapCompatState> {
        Some(GapTextBackend::real_gap_compat_state(self))
    }

    fn is_multibyte(&self) -> bool {
        GapTextBackend::is_multibyte(self)
    }

    fn set_multibyte(&mut self, multibyte: bool) {
        GapTextBackend::set_multibyte(self, multibyte);
    }

    fn byte_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> u8 {
        GapTextBackend::byte_at_emacs_byte_pos(self, pos)
    }

    fn emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8> {
        GapTextBackend::emacs_byte_at_pos(self, pos)
    }

    fn char_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<char> {
        GapTextBackend::char_at_emacs_byte_pos(self, pos)
    }

    fn char_code_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<u32> {
        GapTextBackend::char_code_at_emacs_byte_pos(self, pos)
    }

    fn emacs_byte_pos_to_char_pos(&self, byte_pos: EmacsBytePos) -> CharPos0 {
        GapTextBackend::emacs_byte_pos_to_char_pos(self, byte_pos)
    }

    fn char_pos_to_emacs_byte_pos(&self, char_pos: CharPos0) -> EmacsBytePos {
        GapTextBackend::char_pos_to_emacs_byte_pos(self, char_pos)
    }

    fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String {
        GapTextBackend::text_emacs_byte_range(self, range)
    }

    fn copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>) {
        GapTextBackend::copy_emacs_byte_range_to(self, range, out);
    }

    fn for_each_emacs_byte_range_chunk<E, F>(&self, range: EmacsByteRange, f: F) -> Result<(), E>
    where
        F: FnMut(&[u8]) -> Result<(), E>,
    {
        GapTextBackend::for_each_emacs_byte_range_chunk(self, range, f)
    }

    fn has_contiguous_emacs_byte_range(&self, range: EmacsByteRange) -> bool {
        GapTextBackend::has_contiguous_emacs_byte_range(self, range)
    }

    fn with_contiguous_emacs_byte_range<R, F>(&self, range: EmacsByteRange, f: F) -> Option<R>
    where
        F: FnOnce(&[u8]) -> R,
    {
        GapTextBackend::with_contiguous_emacs_byte_range(self, range, f)
    }

    fn insert_measured_emacs_bytes(&mut self, pos: EmacsBytePos, bytes: &[u8], extent: TextExtent) {
        GapTextBackend::insert_measured_emacs_bytes(self, pos, bytes, extent);
    }

    fn delete_measured_range(&mut self, range: TextEditRange) {
        GapTextBackend::delete_measured_range(self, range);
    }

    fn replace_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]) {
        GapTextBackend::replace_measured_range(self, replacement, bytes);
    }

    fn replace_same_len_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]) {
        GapTextBackend::replace_same_len_measured_range(self, replacement, bytes);
    }

    fn dump_text(&self) -> Vec<u8> {
        GapTextBackend::dump_text(self)
    }
}

impl PhysicalTextBackend for PieceTreeTextBackend {
    fn metrics(&self) -> TextMetrics {
        PieceTreeTextBackend::metrics(self)
    }

    #[cfg(test)]
    fn debug_layout(&self) -> TextBackendDebugLayout {
        PieceTreeTextBackend::debug_layout(self)
    }

    fn is_multibyte(&self) -> bool {
        PieceTreeTextBackend::is_multibyte(self)
    }

    fn set_multibyte(&mut self, multibyte: bool) {
        PieceTreeTextBackend::set_multibyte(self, multibyte);
    }

    fn byte_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> u8 {
        PieceTreeTextBackend::byte_at_emacs_byte_pos(self, pos)
    }

    fn emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8> {
        PieceTreeTextBackend::emacs_byte_at_pos(self, pos)
    }

    fn char_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<char> {
        PieceTreeTextBackend::char_at_emacs_byte_pos(self, pos)
    }

    fn char_code_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<u32> {
        PieceTreeTextBackend::char_code_at_emacs_byte_pos(self, pos)
    }

    fn emacs_byte_pos_to_char_pos(&self, byte_pos: EmacsBytePos) -> CharPos0 {
        PieceTreeTextBackend::emacs_byte_pos_to_char_pos(self, byte_pos)
    }

    fn char_pos_to_emacs_byte_pos(&self, char_pos: CharPos0) -> EmacsBytePos {
        PieceTreeTextBackend::char_pos_to_emacs_byte_pos(self, char_pos)
    }

    fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String {
        PieceTreeTextBackend::text_emacs_byte_range(self, range)
    }

    fn copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>) {
        PieceTreeTextBackend::copy_emacs_byte_range_to(self, range, out);
    }

    fn for_each_emacs_byte_range_chunk<E, F>(&self, range: EmacsByteRange, f: F) -> Result<(), E>
    where
        F: FnMut(&[u8]) -> Result<(), E>,
    {
        PieceTreeTextBackend::for_each_emacs_byte_range_chunk(self, range, f)
    }

    fn has_contiguous_emacs_byte_range(&self, range: EmacsByteRange) -> bool {
        PieceTreeTextBackend::has_contiguous_emacs_byte_range(self, range)
    }

    fn with_contiguous_emacs_byte_range<R, F>(&self, range: EmacsByteRange, f: F) -> Option<R>
    where
        F: FnOnce(&[u8]) -> R,
    {
        PieceTreeTextBackend::with_contiguous_emacs_byte_range(self, range, f)
    }

    fn insert_measured_emacs_bytes(&mut self, pos: EmacsBytePos, bytes: &[u8], extent: TextExtent) {
        PieceTreeTextBackend::insert_measured_emacs_bytes(self, pos, bytes, extent);
    }

    fn delete_measured_range(&mut self, range: TextEditRange) {
        PieceTreeTextBackend::delete_measured_range(self, range);
    }

    fn replace_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]) {
        PieceTreeTextBackend::replace_measured_range(self, replacement, bytes);
    }

    fn replace_same_len_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]) {
        PieceTreeTextBackend::replace_same_len_measured_range(self, replacement, bytes);
    }

    fn dump_text(&self) -> Vec<u8> {
        PieceTreeTextBackend::dump_text(self)
    }
}

impl PhysicalTextBackend for RopeTextBackend {
    fn metrics(&self) -> TextMetrics {
        RopeTextBackend::metrics(self)
    }

    #[cfg(test)]
    fn debug_layout(&self) -> TextBackendDebugLayout {
        RopeTextBackend::debug_layout(self)
    }

    fn is_multibyte(&self) -> bool {
        RopeTextBackend::is_multibyte(self)
    }

    fn set_multibyte(&mut self, multibyte: bool) {
        RopeTextBackend::set_multibyte(self, multibyte);
    }

    fn byte_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> u8 {
        RopeTextBackend::byte_at_emacs_byte_pos(self, pos)
    }

    fn emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8> {
        RopeTextBackend::emacs_byte_at_pos(self, pos)
    }

    fn char_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<char> {
        RopeTextBackend::char_at_emacs_byte_pos(self, pos)
    }

    fn char_code_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<u32> {
        RopeTextBackend::char_code_at_emacs_byte_pos(self, pos)
    }

    fn emacs_byte_pos_to_char_pos(&self, byte_pos: EmacsBytePos) -> CharPos0 {
        RopeTextBackend::emacs_byte_pos_to_char_pos(self, byte_pos)
    }

    fn char_pos_to_emacs_byte_pos(&self, char_pos: CharPos0) -> EmacsBytePos {
        RopeTextBackend::char_pos_to_emacs_byte_pos(self, char_pos)
    }

    fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String {
        RopeTextBackend::text_emacs_byte_range(self, range)
    }

    fn copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>) {
        RopeTextBackend::copy_emacs_byte_range_to(self, range, out);
    }

    fn for_each_emacs_byte_range_chunk<E, F>(&self, range: EmacsByteRange, f: F) -> Result<(), E>
    where
        F: FnMut(&[u8]) -> Result<(), E>,
    {
        RopeTextBackend::for_each_emacs_byte_range_chunk(self, range, f)
    }

    fn has_contiguous_emacs_byte_range(&self, range: EmacsByteRange) -> bool {
        RopeTextBackend::has_contiguous_emacs_byte_range(self, range)
    }

    fn with_contiguous_emacs_byte_range<R, F>(&self, range: EmacsByteRange, f: F) -> Option<R>
    where
        F: FnOnce(&[u8]) -> R,
    {
        RopeTextBackend::with_contiguous_emacs_byte_range(self, range, f)
    }

    fn insert_measured_emacs_bytes(&mut self, pos: EmacsBytePos, bytes: &[u8], extent: TextExtent) {
        RopeTextBackend::insert_measured_emacs_bytes(self, pos, bytes, extent);
    }

    fn delete_measured_range(&mut self, range: TextEditRange) {
        RopeTextBackend::delete_measured_range(self, range);
    }

    fn replace_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]) {
        RopeTextBackend::replace_measured_range(self, replacement, bytes);
    }

    fn replace_same_len_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]) {
        RopeTextBackend::replace_same_len_measured_range(self, replacement, bytes);
    }

    fn dump_text(&self) -> Vec<u8> {
        RopeTextBackend::dump_text(self)
    }
}
