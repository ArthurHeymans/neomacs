//! Buffer text storage.
//!
//! GNU Emacs separates per-buffer metadata from the underlying text object.
//! `BufferText` is the first local seam toward that design. It owns the
//! Lisp-visible text semantics while the concrete byte storage backend remains
//! hidden behind a private enum.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::emacs_core::value::Value;
use crate::gc_trace::GcTrace;

use super::buffer::{BufferId, InsertionType};
use super::gap_buffer::{GAP_BYTES_DFL, GAP_BYTES_MIN};
use super::position::{
    CharLen, CharPos0, CharRange, EmacsBytePos, EmacsByteRange, TextPositionAnchor,
};
use super::text::backend::TextBackend;
use super::text::{
    BufferTextBackendKind, ImplementedBufferTextBackendKind, TextEditRange, TextExtent,
    TextMetrics, TextReplacement, emacs_char_count_bytes,
};
#[cfg(test)]
use super::text::{GapDebugLayout, TextBackendDebugLayout};
use super::text_props::{PropertyInterval, TextPropertyTable};

/// Last successful char↔byte conversion. Reused on a subsequent query if the
/// buffer text has not changed since the entry was stored. Mirrors GNU
/// `marker.c:202-203` but uses a (total_chars, total_bytes) epoch rather than
/// `chars_modiff` so it works correctly even when called directly on
/// `BufferText` without going through the `insdel.rs` tick-bumping path.
#[derive(Clone, Copy, Default)]
struct PositionCache {
    /// Total text size when this entry was stored. Empty = invalid.
    epoch: TextMetrics,
    anchor: TextPositionAnchor,
}

impl PositionCache {
    fn is_valid_for(self, metrics: TextMetrics) -> bool {
        self.epoch == metrics && !self.epoch.is_empty()
    }
}

struct BufferTextStorage {
    metrics: TextMetrics,
    backend: TextBackend,
    virtual_gap_pos: CharPos0,
    virtual_gap_size: usize,
    modified_tick: i64,
    chars_modified_tick: i64,
    save_modified_tick: i64,
    text_props: TextPropertyTable,
    /// Head of the intrusive per-buffer marker chain (GNU `buffer->own_text.markers`).
    /// Authoritative since T6; the parallel `Vec<MarkerEntry>` was deleted in T7.
    markers_head: *mut crate::tagged::header::MarkerObj,
    /// Interior-mutable last-query cache for char↔byte conversion.
    pos_cache: Cell<PositionCache>,
    /// Internal (non-Lisp-visible) anchor positions populated on long scans.
    /// Invalidated wholesale when `(total_chars, total_bytes)` advances.
    anchor_cache: RefCell<Vec<TextPositionAnchor>>,
    /// Text size at which the anchor_cache is valid.
    /// Mismatch triggers a wholesale clear on next read.
    anchor_cache_key: Cell<TextMetrics>,
}

impl Clone for BufferTextStorage {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics,
            backend: self.backend.clone(),
            virtual_gap_pos: self.virtual_gap_pos,
            virtual_gap_size: self.virtual_gap_size,
            modified_tick: self.modified_tick,
            chars_modified_tick: self.chars_modified_tick,
            save_modified_tick: self.save_modified_tick,
            text_props: self.text_props.clone(),
            // Chain head intentionally not cloned: chain pointers are unique
            // per TaggedHeap; a cloned buffer starts with an empty chain and
            // rebuilds it via register_marker.
            markers_head: std::ptr::null_mut(),
            pos_cache: self.pos_cache.clone(),
            anchor_cache: self.anchor_cache.clone(),
            anchor_cache_key: self.anchor_cache_key.clone(),
        }
    }
}

pub struct BufferText {
    storage: Rc<RefCell<BufferTextStorage>>,
}

impl Clone for BufferText {
    fn clone(&self) -> Self {
        let storage = self.storage.borrow().clone();
        Self {
            storage: Rc::new(RefCell::new(storage)),
        }
    }
}

impl Default for BufferText {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferText {
    fn from_backend(backend: TextBackend) -> Self {
        let metrics = backend.metrics();
        let virtual_gap_pos = backend
            .gap_compat_char_pos()
            .unwrap_or_else(|| CharPos0::new(metrics.chars()));
        let virtual_gap_size = backend.gap_compat_size().unwrap_or(GAP_BYTES_MIN);
        Self {
            storage: Rc::new(RefCell::new(BufferTextStorage {
                metrics,
                backend,
                virtual_gap_pos,
                virtual_gap_size,
                modified_tick: 1,
                chars_modified_tick: 1,
                save_modified_tick: 1,
                text_props: TextPropertyTable::new(),
                markers_head: std::ptr::null_mut(),
                pos_cache: Cell::new(PositionCache::default()),
                anchor_cache: RefCell::new(Vec::new()),
                anchor_cache_key: Cell::new(TextMetrics::ZERO),
            })),
        }
    }

    fn refresh_backend_metrics(storage: &mut BufferTextStorage) {
        storage.metrics = storage.backend.metrics();
    }

    fn virtual_gap_consume_bytes(storage: &mut BufferTextStorage, bytes: usize) {
        if storage.virtual_gap_size < bytes {
            let need = bytes - storage.virtual_gap_size;
            storage.virtual_gap_size += need.saturating_add(GAP_BYTES_DFL);
        }
        storage.virtual_gap_size = storage.virtual_gap_size.saturating_sub(bytes);
    }

    fn note_virtual_gap_insert(
        storage: &mut BufferTextStorage,
        pos: EmacsBytePos,
        extent: TextExtent,
    ) {
        if storage.backend.gap_compat_char_pos().is_some() {
            return;
        }
        let start_char = storage.backend.emacs_byte_pos_to_char_pos(pos);
        storage.virtual_gap_pos = CharPos0::new(start_char.get() + extent.chars().get());
        Self::virtual_gap_consume_bytes(storage, extent.emacs_bytes().get());
    }

    fn note_virtual_gap_delete(storage: &mut BufferTextStorage, range: TextEditRange) {
        if storage.backend.gap_compat_char_pos().is_some() {
            return;
        }
        storage.virtual_gap_pos = range.char_start();
        storage.virtual_gap_size += range.byte_len().get();
    }

    fn note_virtual_gap_replace(storage: &mut BufferTextStorage, replacement: TextReplacement) {
        if storage.backend.gap_compat_char_pos().is_some() {
            return;
        }
        let char_start = replacement.old_range().char_start();
        storage.virtual_gap_pos = char_start;
        storage.virtual_gap_size += replacement.old_range().byte_len().get();
        storage.virtual_gap_pos =
            CharPos0::new(char_start.get() + replacement.new_extent().chars().get());
        Self::virtual_gap_consume_bytes(storage, replacement.new_extent().emacs_bytes().get());
    }

    fn invalidate_position_caches(storage: &mut BufferTextStorage) {
        storage.pos_cache.set(PositionCache::default());
        storage.anchor_cache.borrow_mut().clear();
        storage.anchor_cache_key.set(TextMetrics::ZERO);
    }

    fn byte_range_to_char_range_with_storage(
        storage: &BufferTextStorage,
        range: EmacsByteRange,
    ) -> CharRange {
        CharRange::new(
            storage.backend.emacs_byte_pos_to_char_pos(range.start()),
            storage.backend.emacs_byte_pos_to_char_pos(range.end()),
        )
    }

    fn byte_range_to_char_range(&self, range: EmacsByteRange) -> CharRange {
        let storage = self.storage.borrow();
        Self::byte_range_to_char_range_with_storage(&storage, range)
    }

    pub fn new() -> Self {
        Self::from_backend(TextBackend::new(
            ImplementedBufferTextBackendKind::GapBuffer,
        ))
    }

    pub fn try_new_with_backend_kind(
        kind: BufferTextBackendKind,
    ) -> Result<Self, BufferTextBackendKind> {
        Ok(Self::new_with_backend_kind(kind.try_into()?))
    }

    pub(crate) fn new_with_backend_kind(kind: ImplementedBufferTextBackendKind) -> Self {
        Self::from_backend(TextBackend::new(kind))
    }

    pub fn from_str(text: &str) -> Self {
        Self::from_backend(TextBackend::from_str(
            text,
            ImplementedBufferTextBackendKind::GapBuffer,
        ))
    }

    pub fn try_from_str_with_backend_kind(
        text: &str,
        kind: BufferTextBackendKind,
    ) -> Result<Self, BufferTextBackendKind> {
        Ok(Self::from_str_with_backend_kind(text, kind.try_into()?))
    }

    pub(crate) fn from_str_with_backend_kind(
        text: &str,
        kind: ImplementedBufferTextBackendKind,
    ) -> Self {
        Self::from_backend(TextBackend::from_str(text, kind))
    }

    pub fn from_lisp_string(text: &crate::heap_types::LispString) -> Self {
        Self::from_lisp_string_with_backend_kind(text, ImplementedBufferTextBackendKind::GapBuffer)
    }

    pub fn try_from_lisp_string_with_backend_kind(
        text: &crate::heap_types::LispString,
        kind: BufferTextBackendKind,
    ) -> Result<Self, BufferTextBackendKind> {
        Ok(Self::from_lisp_string_with_backend_kind(
            text,
            kind.try_into()?,
        ))
    }

    pub(crate) fn from_lisp_string_with_backend_kind(
        text: &crate::heap_types::LispString,
        kind: ImplementedBufferTextBackendKind,
    ) -> Self {
        Self::from_backend(TextBackend::from_emacs_bytes(
            text.as_bytes(),
            text.is_multibyte(),
            kind,
        ))
    }

    pub fn backend_kind(&self) -> BufferTextBackendKind {
        self.storage.borrow().backend.kind().public_kind()
    }

    pub(crate) fn implemented_backend_kind(&self) -> ImplementedBufferTextBackendKind {
        self.storage.borrow().backend.kind()
    }

    pub fn try_convert_backend_kind(
        &self,
        kind: BufferTextBackendKind,
    ) -> Result<(), BufferTextBackendKind> {
        self.convert_backend_kind(kind.try_into()?);
        Ok(())
    }

    pub(crate) fn convert_backend_kind(&self, kind: ImplementedBufferTextBackendKind) {
        let mut storage = self.storage.borrow_mut();
        if storage.backend.kind() == kind {
            return;
        }
        let virtual_gap_pos = storage
            .backend
            .gap_compat_char_pos()
            .unwrap_or(storage.virtual_gap_pos);
        let virtual_gap_size = storage
            .backend
            .gap_compat_size()
            .unwrap_or(storage.virtual_gap_size);
        let text = storage.backend.dump_text();
        let multibyte = storage.backend.is_multibyte();
        storage.backend = TextBackend::from_dump(text, multibyte, kind);
        storage.virtual_gap_pos = virtual_gap_pos;
        storage.virtual_gap_size = virtual_gap_size;
        Self::refresh_backend_metrics(&mut storage);
        Self::invalidate_position_caches(&mut storage);
    }

    pub fn len(&self) -> usize {
        self.storage.borrow().metrics.emacs_bytes()
    }

    pub fn is_multibyte(&self) -> bool {
        self.storage.borrow().backend.is_multibyte()
    }

    pub fn set_multibyte(&self, multibyte: bool) {
        let mut storage = self.storage.borrow_mut();
        if storage.backend.is_multibyte() == multibyte {
            return;
        }
        storage.backend.set_multibyte(multibyte);
        Self::refresh_backend_metrics(&mut storage);
        Self::invalidate_position_caches(&mut storage);
    }

    pub fn is_empty(&self) -> bool {
        self.storage.borrow().metrics.is_empty()
    }

    pub fn char_count(&self) -> usize {
        self.storage.borrow().metrics.chars()
    }

    pub fn emacs_byte_len(&self) -> usize {
        self.storage.borrow().metrics.emacs_bytes()
    }

    pub fn metrics(&self) -> TextMetrics {
        self.storage.borrow().metrics
    }

    pub(crate) fn gap_position_lisp(&self) -> i64 {
        let storage = self.storage.borrow();
        storage
            .backend
            .gap_compat_char_pos()
            .unwrap_or(storage.virtual_gap_pos)
            .to_lisp()
            .as_i64()
    }

    pub(crate) fn gap_size_lisp(&self) -> usize {
        let storage = self.storage.borrow();
        storage
            .backend
            .gap_compat_size()
            .unwrap_or(storage.virtual_gap_size)
    }

    #[cfg(test)]
    pub(crate) fn backend_debug_layout(&self) -> TextBackendDebugLayout {
        self.storage.borrow().backend.debug_layout()
    }

    #[cfg(test)]
    pub(crate) fn gap_debug_layout(&self) -> Option<GapDebugLayout> {
        self.storage.borrow().backend.debug_layout().gap()
    }

    pub fn modified_tick(&self) -> i64 {
        self.storage.borrow().modified_tick
    }

    pub fn chars_modified_tick(&self) -> i64 {
        self.storage.borrow().chars_modified_tick
    }

    pub fn save_modified_tick(&self) -> i64 {
        self.storage.borrow().save_modified_tick
    }

    pub fn byte_at(&self, pos: usize) -> u8 {
        self.byte_at_emacs_byte_pos(EmacsBytePos::new(pos))
    }

    pub(crate) fn byte_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> u8 {
        self.storage.borrow().backend.byte_at_emacs_byte_pos(pos)
    }

    pub fn emacs_byte_at(&self, pos: usize) -> Option<u8> {
        self.emacs_byte_at_pos(EmacsBytePos::new(pos))
    }

    pub(crate) fn emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8> {
        self.storage.borrow().backend.emacs_byte_at_pos(pos)
    }

    pub fn char_at(&self, pos: usize) -> Option<char> {
        self.char_at_emacs_byte_pos(EmacsBytePos::new(pos))
    }

    pub(crate) fn char_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<char> {
        self.storage.borrow().backend.char_at_emacs_byte_pos(pos)
    }

    pub fn char_code_at(&self, pos: usize) -> Option<u32> {
        self.char_code_at_emacs_byte_pos(EmacsBytePos::new(pos))
    }

    pub(crate) fn char_code_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<u32> {
        self.storage
            .borrow()
            .backend
            .char_code_at_emacs_byte_pos(pos)
    }

    pub fn text_range(&self, start: usize, end: usize) -> String {
        self.text_emacs_byte_range(EmacsByteRange::from_usize(start, end))
    }

    pub(crate) fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String {
        self.storage.borrow().backend.text_emacs_byte_range(range)
    }

    pub fn copy_bytes_to(&self, start: usize, end: usize, out: &mut Vec<u8>) {
        self.copy_emacs_byte_range_to(EmacsByteRange::from_usize(start, end), out);
    }

    pub(crate) fn copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>) {
        self.storage
            .borrow()
            .backend
            .copy_emacs_byte_range_to(range, out);
    }

    pub(crate) fn for_each_emacs_byte_range_chunk<E>(
        &self,
        range: EmacsByteRange,
        f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        self.storage
            .borrow()
            .backend
            .for_each_emacs_byte_range_chunk(range, f)
    }

    pub(crate) fn has_contiguous_emacs_byte_range(&self, range: EmacsByteRange) -> bool {
        self.storage
            .borrow()
            .backend
            .has_contiguous_emacs_byte_range(range)
    }

    pub(crate) fn with_contiguous_emacs_byte_range<R>(
        &self,
        range: EmacsByteRange,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Option<R> {
        self.storage
            .borrow()
            .backend
            .with_contiguous_emacs_byte_range(range, f)
    }

    #[cfg(test)]
    pub fn insert_str(&mut self, pos: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let multibyte = self.is_multibyte();
        let bytes =
            crate::emacs_core::string_escape::storage_string_to_buffer_bytes(text, multibyte);
        let extent = TextExtent::from_usize(emacs_char_count_bytes(&bytes, multibyte), bytes.len());
        self.insert_measured_emacs_bytes(EmacsBytePos::new(pos), &bytes, extent);
    }

    #[cfg(test)]
    pub fn insert_emacs_bytes(&mut self, pos: usize, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let multibyte = self.is_multibyte();
        let extent = TextExtent::from_usize(emacs_char_count_bytes(bytes, multibyte), bytes.len());
        self.insert_measured_emacs_bytes(EmacsBytePos::new(pos), bytes, extent);
    }

    #[cfg(test)]
    pub fn insert_emacs_bytes_both(&mut self, pos: usize, bytes: &[u8], nchars: usize) {
        self.insert_measured_emacs_bytes(
            EmacsBytePos::new(pos),
            bytes,
            TextExtent::from_usize(nchars, bytes.len()),
        );
    }

    pub(crate) fn insert_measured_emacs_bytes(
        &mut self,
        pos: EmacsBytePos,
        bytes: &[u8],
        extent: TextExtent,
    ) {
        if bytes.is_empty() {
            return;
        }
        let mut storage = self.storage.borrow_mut();
        Self::note_virtual_gap_insert(&mut storage, pos, extent);
        storage
            .backend
            .insert_measured_emacs_bytes(pos, bytes, extent);
        Self::refresh_backend_metrics(&mut storage);
    }

    #[cfg(test)]
    pub fn delete_range(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let byte_range = EmacsByteRange::from_usize(start, end);
        let char_range = self.byte_range_to_char_range(byte_range);
        let range = TextEditRange::new(byte_range, char_range.start(), char_range.end());
        self.delete_measured_range(range);
    }

    #[cfg(test)]
    pub fn delete_range_both(&mut self, start: usize, end: usize, nchars: usize) {
        let start_char = self.emacs_byte_pos_to_char_pos(EmacsBytePos::new(start));
        let range = TextEditRange::new(
            EmacsByteRange::from_usize(start, end),
            start_char,
            CharPos0::new(start_char.get() + nchars),
        );
        self.delete_measured_range(range);
    }

    pub(crate) fn delete_measured_range(&mut self, range: TextEditRange) {
        if range.is_empty() {
            return;
        }
        let mut storage = self.storage.borrow_mut();
        Self::note_virtual_gap_delete(&mut storage, range);
        storage.backend.delete_measured_range(range);
        Self::refresh_backend_metrics(&mut storage);
    }

    pub(crate) fn replace_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]) {
        if replacement.old_range().is_empty() && bytes.is_empty() {
            return;
        }
        let mut storage = self.storage.borrow_mut();
        Self::note_virtual_gap_replace(&mut storage, replacement);
        storage.backend.replace_measured_range(replacement, bytes);
        Self::refresh_backend_metrics(&mut storage);
    }

    #[cfg(test)]
    pub fn replace_same_len_emacs_bytes(&mut self, start: usize, end: usize, replacement: &[u8]) {
        self.replace_same_len_emacs_byte_range(EmacsByteRange::from_usize(start, end), replacement);
    }

    pub(crate) fn replace_same_len_emacs_byte_range(
        &mut self,
        range: EmacsByteRange,
        replacement: &[u8],
    ) {
        if range.is_empty() {
            return;
        }
        let mut storage = self.storage.borrow_mut();
        storage
            .backend
            .replace_same_len_emacs_byte_range(range, replacement);
        Self::refresh_backend_metrics(&mut storage);
    }

    pub fn shared_clone(&self) -> Self {
        Self {
            storage: Rc::clone(&self.storage),
        }
    }

    pub(crate) fn shares_storage_with(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.storage, &other.storage)
    }

    pub(crate) fn dump_text(&self) -> Vec<u8> {
        self.storage.borrow().backend.dump_text()
    }

    pub(crate) fn from_dump(text: Vec<u8>, multibyte: bool) -> Self {
        Self::from_backend(TextBackend::from_dump(
            text,
            multibyte,
            ImplementedBufferTextBackendKind::GapBuffer,
        ))
    }

    pub(crate) fn from_dump_with_backend_kind(
        text: Vec<u8>,
        multibyte: bool,
        kind: ImplementedBufferTextBackendKind,
    ) -> Self {
        Self::from_backend(TextBackend::from_dump(text, multibyte, kind))
    }

    pub fn set_modification_state(
        &self,
        modified_tick: i64,
        chars_modified_tick: i64,
        save_modified_tick: i64,
    ) {
        let mut storage = self.storage.borrow_mut();
        storage.modified_tick = modified_tick;
        storage.chars_modified_tick = chars_modified_tick;
        storage.save_modified_tick = save_modified_tick;
    }

    pub fn set_modified_tick(&self, tick: i64) {
        self.storage.borrow_mut().modified_tick = tick;
    }

    pub fn set_save_modified_tick(&self, tick: i64) {
        self.storage.borrow_mut().save_modified_tick = tick;
    }

    pub fn increment_modified_tick(&self, delta: i64) {
        self.storage.borrow_mut().modified_tick += delta;
    }

    pub fn record_char_modification(&self, delta: i64) {
        let mut storage = self.storage.borrow_mut();
        storage.modified_tick += delta;
        storage.chars_modified_tick = storage.modified_tick;
    }

    pub fn range_contains_char_code(&self, start: usize, end: usize, code: u32) -> bool {
        if start >= end {
            return false;
        }
        // Walk buffer bytes directly, avoiding the storage-form conversion
        // previously done through text_range(). For multibyte buffers each
        // Emacs char is decoded via emacs_char::string_char; for unibyte
        // buffers each byte is one "character" in the range 0..=0xFF.
        let mut bytes = Vec::with_capacity(end - start);
        self.storage
            .borrow()
            .backend
            .copy_emacs_byte_range_to(EmacsByteRange::from_usize(start, end), &mut bytes);
        if self.is_multibyte() {
            let mut pos = 0;
            while pos < bytes.len() {
                let (c, len) = crate::emacs_core::emacs_char::string_char(&bytes[pos..]);
                if c == code {
                    return true;
                }
                pos += len.max(1);
            }
            false
        } else {
            if code > 0xFF {
                return false;
            }
            bytes.iter().any(|&b| b as u32 == code)
        }
    }

    pub fn text_props_is_empty(&self) -> bool {
        self.storage.borrow().text_props.is_empty()
    }

    pub fn text_props_snapshot(&self) -> TextPropertyTable {
        self.storage.borrow().text_props.clone()
    }

    pub fn text_props_replace(&self, table: TextPropertyTable) {
        self.storage.borrow_mut().text_props = table;
    }

    pub fn replace_storage(&self, text: &str, multibyte: bool, text_props: TextPropertyTable) {
        let bytes =
            crate::emacs_core::string_escape::storage_string_to_buffer_bytes(text, multibyte);
        let string = if multibyte {
            crate::heap_types::LispString::from_emacs_bytes(bytes)
        } else {
            crate::heap_types::LispString::from_unibyte(bytes)
        };
        self.replace_lisp_string(&string, text_props);
    }
    pub fn replace_lisp_string(
        &self,
        text: &crate::heap_types::LispString,
        text_props: TextPropertyTable,
    ) {
        let mut storage = self.storage.borrow_mut();
        let kind = storage.backend.kind();
        storage.backend = TextBackend::from_emacs_bytes(text.as_bytes(), text.is_multibyte(), kind);
        Self::refresh_backend_metrics(&mut storage);
        storage.virtual_gap_pos = storage
            .backend
            .gap_compat_char_pos()
            .unwrap_or_else(|| CharPos0::new(storage.metrics.chars()));
        storage.virtual_gap_size = storage.backend.gap_compat_size().unwrap_or(GAP_BYTES_DFL);
        storage.text_props = text_props;
        // Wholesale content replacement: invalidate position caches. If the new
        // content happens to have the same (total_chars, total_bytes) as the old,
        // a stale pos_cache entry could otherwise return a wrong bytepos.
        Self::invalidate_position_caches(&mut storage);
    }

    /// Walk the intrusive marker chain and remap each marker's (bytepos,
    /// charpos) through the caller-supplied closure. Used by
    /// `set-buffer-multibyte` to translate marker positions across a
    /// wholesale gap-buffer replacement (the boundary arithmetic lives in
    /// the caller; this helper only handles chain traversal).
    ///
    /// The closure receives the marker's current `bytepos` and returns
    /// the new `(bytepos, charpos)` pair.
    pub fn remap_markers_through<F>(&self, mut remap: F)
    where
        F: FnMut(usize) -> (usize, usize),
    {
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: `curr` walks live chain-owned MarkerObj pointers from
        // `markers_head` until null. Each non-null node was spliced in via
        // `chain_splice_at_head`, so its `data.next_marker` is a valid
        // chain link or null.
        unsafe {
            while !curr.is_null() {
                let data = &mut (*curr).data;
                let (new_byte, new_char) = remap(data.bytepos);
                data.bytepos = new_byte;
                data.charpos = new_char;
                curr = data.next_marker;
            }
        }
    }

    /// Like [`Self::remap_markers_through`], but exposes both old byte and
    /// old character positions to the caller.  GNU's `transpose_markers`
    /// updates these two cached positions independently because text motion is
    /// byte-based while interval and marker semantics are character-based.
    pub fn remap_markers_through_byte_char<F>(&self, mut remap: F)
    where
        F: FnMut(usize, usize) -> (usize, usize),
    {
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: same intrusive-chain invariant as `remap_markers_through`.
        unsafe {
            while !curr.is_null() {
                let data = &mut (*curr).data;
                let (new_byte, new_char) = remap(data.bytepos, data.charpos);
                data.bytepos = new_byte;
                data.charpos = new_char;
                curr = data.next_marker;
            }
        }
    }

    pub fn text_props_put_property(
        &self,
        start: usize,
        end: usize,
        name: Value,
        value: Value,
    ) -> bool {
        // GNU intervals are character-indexed; BufferText owns the conversion
        // from buffer byte offsets into interval positions.
        let (range, object_len) = {
            let storage = self.storage.borrow();
            (
                Self::byte_range_to_char_range_with_storage(
                    &storage,
                    EmacsByteRange::from_usize(start, end),
                ),
                storage.metrics.chars(),
            )
        };
        self.storage
            .borrow_mut()
            .text_props
            .put_property_for_object_len(
                range.start_usize(),
                range.end_usize(),
                object_len,
                name,
                value,
            )
    }

    pub fn text_props_get_property(&self, pos: usize, name: Value) -> Option<Value> {
        let pos = self.byte_range_to_char_range(EmacsByteRange::from_usize(pos, pos));
        self.storage
            .borrow()
            .text_props
            .get_property(pos.start_usize(), name)
    }

    pub fn text_props_get_properties(&self, pos: usize) -> HashMap<Value, Value> {
        let pos = self.byte_range_to_char_range(EmacsByteRange::from_usize(pos, pos));
        self.storage
            .borrow()
            .text_props
            .get_properties(pos.start_usize())
    }

    pub fn text_props_get_properties_ordered(&self, pos: usize) -> Vec<(Value, Value)> {
        let pos = self.byte_range_to_char_range(EmacsByteRange::from_usize(pos, pos));
        self.storage
            .borrow()
            .text_props
            .get_properties_ordered(pos.start_usize())
    }

    pub fn text_props_get_properties_plist_value(&self, pos: usize) -> Value {
        let pos = self.byte_range_to_char_range(EmacsByteRange::from_usize(pos, pos));
        self.storage
            .borrow()
            .text_props
            .get_properties_plist_value(pos.start_usize())
    }

    pub fn text_props_range_has_all_properties(
        &self,
        start: usize,
        end: usize,
        properties: &[(Value, Value)],
    ) -> bool {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(start, end));
        self.storage.borrow().text_props.range_has_all_properties(
            range.start_usize(),
            range.end_usize(),
            properties,
        )
    }

    pub fn text_props_range_has_any_property_named(
        &self,
        start: usize,
        end: usize,
        names: &[Value],
    ) -> bool {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(start, end));
        self.storage
            .borrow()
            .text_props
            .range_has_any_property_named(range.start_usize(), range.end_usize(), names)
    }

    pub fn text_props_range_has_any_interval(&self, start: usize, end: usize) -> bool {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(start, end));
        self.storage
            .borrow()
            .text_props
            .range_has_any_interval(range.start_usize(), range.end_usize())
    }

    pub fn text_props_remove_property(&self, start: usize, end: usize, name: Value) -> bool {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(start, end));
        self.storage.borrow_mut().text_props.remove_property(
            range.start_usize(),
            range.end_usize(),
            name,
        )
    }

    pub fn text_props_remove_all(&self, start: usize, end: usize) {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(start, end));
        self.storage
            .borrow_mut()
            .text_props
            .remove_all_properties(range.start_usize(), range.end_usize());
    }

    pub fn text_props_set_properties(&self, start: usize, end: usize, plist: Vec<(Value, Value)>) {
        let (range, object_len) = {
            let storage = self.storage.borrow();
            (
                Self::byte_range_to_char_range_with_storage(
                    &storage,
                    EmacsByteRange::from_usize(start, end),
                ),
                storage.metrics.chars(),
            )
        };
        self.storage
            .borrow_mut()
            .text_props
            .set_properties_for_object_len(
                range.start_usize(),
                range.end_usize(),
                object_len,
                plist,
            );
    }

    pub fn text_props_next_change(&self, pos: usize) -> Option<usize> {
        let char_pos = self
            .byte_range_to_char_range(EmacsByteRange::from_usize(pos, pos))
            .start_usize();
        let next = {
            self.storage
                .borrow()
                .text_props
                .next_property_change(char_pos)
        };
        next.map(|next| self.char_pos_to_emacs_byte_pos(CharPos0::new(next)).get())
    }

    pub fn text_props_previous_change(&self, pos: usize) -> Option<usize> {
        let char_pos = self
            .byte_range_to_char_range(EmacsByteRange::from_usize(pos, pos))
            .start_usize();
        let prev = {
            self.storage
                .borrow()
                .text_props
                .previous_property_change(char_pos)
        };
        prev.map(|prev| self.char_pos_to_emacs_byte_pos(CharPos0::new(prev)).get())
    }

    pub fn text_props_next_interval_boundary(&self, pos: usize) -> Option<usize> {
        let char_pos = self
            .byte_range_to_char_range(EmacsByteRange::from_usize(pos, pos))
            .start_usize();
        let next = {
            self.storage
                .borrow()
                .text_props
                .next_interval_boundary(char_pos)
        };
        next.map(|next| self.char_pos_to_emacs_byte_pos(CharPos0::new(next)).get())
    }

    pub fn text_props_first_interval_pos_with_property_eq(
        &self,
        start: usize,
        end: usize,
        name: Value,
        value: Value,
    ) -> Option<usize> {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(start, end));
        let pos = self
            .storage
            .borrow()
            .text_props
            .first_interval_pos_with_property_eq(
                range.start_usize(),
                range.end_usize(),
                name,
                value,
            )?;
        Some(self.char_pos_to_emacs_byte_pos(CharPos0::new(pos)).get())
    }

    pub fn text_props_previous_interval_boundary(&self, pos: usize) -> Option<usize> {
        let char_pos = self
            .byte_range_to_char_range(EmacsByteRange::from_usize(pos, pos))
            .start_usize();
        let prev = {
            self.storage
                .borrow()
                .text_props
                .previous_interval_boundary(char_pos)
        };
        prev.map(|prev| self.char_pos_to_emacs_byte_pos(CharPos0::new(prev)).get())
    }

    pub fn text_props_append_shifted(&self, other: &TextPropertyTable, byte_offset: usize) {
        let char_offset = self
            .byte_range_to_char_range(EmacsByteRange::from_usize(byte_offset, byte_offset))
            .start_usize();
        self.storage
            .borrow_mut()
            .text_props
            .append_shifted(other, char_offset);
    }

    pub fn text_props_merge_missing_shifted(&self, other: &TextPropertyTable, byte_offset: usize) {
        let char_offset = self
            .byte_range_to_char_range(EmacsByteRange::from_usize(byte_offset, byte_offset))
            .start_usize();
        self.storage
            .borrow_mut()
            .text_props
            .merge_missing_shifted(other, char_offset);
    }

    pub fn text_props_merge_adjacent_equal_around(&self, byte_start: usize, byte_end: usize) {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(byte_start, byte_end));
        self.storage
            .borrow_mut()
            .text_props
            .merge_adjacent_equal_properties_around(range.start_usize(), range.end_usize());
    }

    pub fn text_props_slice(&self, start: usize, end: usize) -> TextPropertyTable {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(start, end));
        self.storage
            .borrow()
            .text_props
            .slice(range.start_usize(), range.end_usize())
    }

    pub fn text_props_intervals_snapshot(&self) -> Vec<PropertyInterval> {
        self.storage.borrow().text_props.intervals_snapshot()
    }

    pub fn text_props_object_interval_runs(
        &self,
        len: usize,
    ) -> Vec<(usize, usize, Vec<(Value, Value)>)> {
        self.storage.borrow().text_props.object_interval_runs(len)
    }

    pub(crate) fn text_props_try_for_each_interval_in_range<E>(
        &self,
        start: usize,
        end: usize,
        f: impl FnMut(usize, usize, &[(Value, Value)]) -> Result<(), E>,
    ) -> Result<(), E> {
        let range = self.byte_range_to_char_range(EmacsByteRange::from_usize(start, end));
        self.storage
            .borrow()
            .text_props
            .try_for_each_interval_in_range(range.start_usize(), range.end_usize(), f)
    }

    pub(crate) fn adjust_text_props_for_insert_at(&self, pos: CharPos0, len: CharLen) {
        self.storage
            .borrow_mut()
            .text_props
            .adjust_for_insert(pos.get(), len.get());
    }

    pub(crate) fn adjust_text_props_for_delete_range(&self, range: CharRange) {
        self.storage
            .borrow_mut()
            .text_props
            .adjust_for_delete(range.start_usize(), range.end_usize());
    }

    pub(crate) fn adjust_text_props_for_replace_at(
        &self,
        start: CharPos0,
        old_len: CharLen,
        new_len: CharLen,
    ) {
        self.storage.borrow_mut().text_props.adjust_for_replace(
            start.get(),
            old_len.get(),
            new_len.get(),
        );
    }

    pub fn trace_text_prop_roots(&self, roots: &mut Vec<Value>) {
        self.storage.borrow().text_props.trace_roots(roots);
    }

    /// Register a marker in this buffer. Updates `LispMarker` fields
    /// authoritatively (buffer/bytepos/charpos/marker_id/insertion_type)
    /// and splices the marker into this buffer's intrusive chain at head.
    ///
    /// **Precondition:** `marker_ptr.data.next_marker` is null, i.e. the
    /// marker is not currently on any chain. Callers re-binding a marker
    /// must `chain_unlink` from the old buffer first; the
    /// `debug_assert!` in `chain_splice_at_head` catches violations.
    pub(crate) fn register_marker(
        &self,
        marker_ptr: *mut crate::tagged::header::MarkerObj,
        buffer_id: BufferId,
        marker_id: u64,
        byte_pos: usize,
        char_pos: usize,
        insertion_type: InsertionType,
    ) {
        // Update LispMarker so its fields are authoritative before the
        // chain ever exposes this marker.
        //
        // SAFETY: `marker_ptr` is a live MarkerObj allocated via
        // `TaggedHeap::alloc_marker`; writes through a raw pointer are
        // sound for the heap's lifetime. The chain precondition is
        // enforced by `chain_splice_at_head`'s debug_assert below.
        unsafe {
            (*marker_ptr).data.buffer = Some(buffer_id);
            (*marker_ptr).data.marker_id = Some(marker_id);
            (*marker_ptr).data.bytepos = byte_pos;
            (*marker_ptr).data.charpos = char_pos;
            (*marker_ptr).data.last_position_valid = true;
            (*marker_ptr).data.insertion_type = insertion_type == InsertionType::After;
        }
        self.chain_splice_at_head(marker_ptr);
    }

    /// Walk the intrusive marker chain head→tail and invoke `f` on each
    /// live `LispMarker` by reference. Read-only counterpart to
    /// `chain_walk_mut`; used by pdump (v26) to serialize the chain
    /// without materializing an intermediate Vec.
    ///
    /// SAFETY: walks live chain-owned MarkerObj pointers from
    /// `storage.markers_head` until null; each `(*curr).data` reference
    /// stays valid for the duration of the call because the GC sweep
    /// runs `unchain_dead_markers` between mark and free.
    pub fn chain_walk_data<F: FnMut(&crate::heap_types::LispMarker)>(&self, mut f: F) {
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        unsafe {
            while !curr.is_null() {
                let data = &(*curr).data;
                f(data);
                curr = data.next_marker;
            }
        }
    }

    /// Return GNU `record_marker_adjustments` entries for markers whose
    /// Lisp character positions are in the deleted range `[from, to]`.
    pub fn marker_adjustments_for_delete(
        &self,
        from_char: usize,
        to_char: usize,
    ) -> Vec<(crate::emacs_core::value::Value, i64)> {
        let from1 = from_char as i64 + 1;
        let to1 = to_char as i64 + 1;
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        let mut adjustments = Vec::new();
        unsafe {
            while !curr.is_null() {
                let data = &(*curr).data;
                let charpos1 = data.charpos as i64 + 1;
                if from1 <= charpos1 && charpos1 <= to1 {
                    let target = if data.insertion_type { to1 } else { from1 };
                    let adjustment = target - charpos1;
                    if adjustment != 0 {
                        adjustments.push((
                            crate::emacs_core::value::Value::from_veclike_ptr(
                                curr as *const crate::tagged::header::VecLikeHeader,
                            ),
                            adjustment,
                        ));
                    }
                }
                curr = data.next_marker;
            }
        }
        adjustments
    }

    /// Read the byte position of a marker by id.
    pub fn marker_bytepos(&self, marker_id: u64) -> usize {
        let ptr = self.chain_find_by_id(marker_id);
        if ptr.is_null() {
            0
        } else {
            unsafe { (*ptr).data.bytepos }
        }
    }

    /// Read the char position of a marker by id.
    pub fn marker_charpos(&self, marker_id: u64) -> usize {
        let ptr = self.chain_find_by_id(marker_id);
        if ptr.is_null() {
            0
        } else {
            unsafe { (*ptr).data.charpos }
        }
    }

    pub(crate) fn move_marker_to_position(
        &self,
        marker_id: u64,
        bytepos: EmacsBytePos,
        charpos: CharPos0,
    ) {
        let ptr = self.chain_find_by_id(marker_id);
        if ptr.is_null() {
            return;
        }
        unsafe {
            (*ptr).data.bytepos = bytepos.get();
            (*ptr).data.charpos = charpos.get();
        }
    }

    /// Walk this buffer's intrusive marker chain and return the raw
    /// MarkerObj pointer for the first node whose `marker_id` matches,
    /// or null when none found. Used by pdump load (v26) to resolve
    /// `BufferStateMarkers` (pt/begv/zv) ids back to chain pointers
    /// after the chain has been reconstructed.
    ///
    /// Pointer lifetime: the returned `*mut MarkerObj` is only valid while
    /// `self`'s chain still holds it. Any subsequent splice/unlink on this
    /// buffer's chain, or a GC cycle that runs `unchain_dead_markers`, may
    /// detach the node from the chain — callers must use the pointer
    /// before doing anything that could mutate the chain, and must not
    /// re-enter the chain (or invoke arbitrary Lisp) between lookup and
    /// use.
    pub fn chain_find_by_id(&self, marker_id: u64) -> *mut crate::tagged::header::MarkerObj {
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: chain walks live chain-owned MarkerObj pointers from
        // `storage.markers_head` until null.
        unsafe {
            while !curr.is_null() {
                if (*curr).data.marker_id == Some(marker_id) {
                    return curr;
                }
                curr = (*curr).data.next_marker;
            }
        }
        std::ptr::null_mut()
    }

    /// Walk the intrusive chain and return the LispMarker-derived fields
    /// `(bytepos, charpos, insertion_type)` for the marker with the given
    /// id, or `None` if no live chain node carries that id.
    ///
    /// Production code should prefer reading `LispMarker` directly off a
    /// Lisp `Value`. This helper exists for internal buffer-manager
    /// callers (e.g. `clone_marker_in_buffer`) that track markers by id
    /// without holding the Lisp value.
    pub fn marker_chain_lookup(&self, marker_id: u64) -> Option<(usize, usize, InsertionType)> {
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: chain walks live chain-owned MarkerObj pointers until null.
        unsafe {
            while !curr.is_null() {
                let data = &(*curr).data;
                if data.marker_id == Some(marker_id) {
                    let ins = if data.insertion_type {
                        InsertionType::After
                    } else {
                        InsertionType::Before
                    };
                    return Some((data.bytepos, data.charpos, ins));
                }
                curr = data.next_marker;
            }
        }
        None
    }

    pub fn remove_marker(&self, marker_id: u64) {
        // Post-T8: `unchain_dead_markers` splices unmarked MarkerObjs
        // out of this buffer's chain between the mark and sweep GC
        // phases, so a chain walk between GC cycles never dereferences
        // a freed allocation. Walk the chain directly and splice the
        // matching node.
        let marker_ptr: Option<*mut crate::tagged::header::MarkerObj> = {
            let storage = self.storage.borrow();
            let mut curr = storage.markers_head;
            let mut found = None;
            // SAFETY: chain walks live chain-owned MarkerObj pointers
            // from `storage.markers_head` until null.
            unsafe {
                while !curr.is_null() {
                    if (*curr).data.marker_id == Some(marker_id) {
                        found = Some(curr);
                        break;
                    }
                    curr = (*curr).data.next_marker;
                }
            }
            found
        };
        if let Some(ptr) = marker_ptr {
            self.chain_unlink(ptr);
            // SAFETY: `ptr` was read from this buffer's chain; chain-
            // owned allocations stay live until the next GC sweep.
            // `chain_unlink` left it detached; field writes are sound.
            unsafe {
                (*ptr).data.buffer = None;
                // GNU `unchain_marker` (marker.c:684) preserves charpos so
                // `marker-last-position` can still report the marker's last
                // attached location.  `last_position_valid` stays true.
            }
        }
    }

    pub fn update_marker_insertion_type(&self, marker_id: u64, insertion_type: InsertionType) {
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: chain walks live chain-owned MarkerObj pointers until null.
        unsafe {
            while !curr.is_null() {
                if (*curr).data.marker_id == Some(marker_id) {
                    (*curr).data.insertion_type = insertion_type == InsertionType::After;
                    return;
                }
                curr = (*curr).data.next_marker;
            }
        }
    }

    /// Return true iff a marker with `marker_id` is currently spliced
    /// into this buffer's chain. Used by BufferManager to pick the
    /// correct buffer when updating insertion type across buffers.
    pub fn has_marker(&self, marker_id: u64) -> bool {
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: chain walks live chain-owned MarkerObj pointers until null.
        unsafe {
            while !curr.is_null() {
                if (*curr).data.marker_id == Some(marker_id) {
                    return true;
                }
                curr = (*curr).data.next_marker;
            }
        }
        false
    }

    pub(crate) fn adjust_markers_for_insert_extent(
        &self,
        insert_pos: EmacsBytePos,
        extent: TextExtent,
    ) {
        let byte_len = extent.emacs_bytes().get();
        if byte_len == 0 {
            return;
        }
        let insert_pos = insert_pos.get();
        let char_len = extent.chars().get();
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: `curr` walks live chain-owned MarkerObj pointers from
        // `markers_head` until null. Each non-null node was spliced in via
        // `chain_splice_at_head`, so its `data.next_marker` is a valid
        // chain link or null.
        unsafe {
            while !curr.is_null() {
                let data = &mut (*curr).data;
                if data.bytepos > insert_pos {
                    data.bytepos += byte_len;
                    data.charpos += char_len;
                } else if data.bytepos == insert_pos && data.insertion_type {
                    // insertion_type == true means "after" in GNU terms.
                    data.bytepos += byte_len;
                    data.charpos += char_len;
                }
                curr = data.next_marker;
            }
        }
    }

    /// Like normal insert marker adjustment, but ignores `insertion_type` for
    /// markers AT `insert_pos`. Used by the GNU-equivalent replace path, where
    /// markers that ended up at `from_byte` (after the prior delete collapsed
    /// inside-region markers there) must NOT advance past the inserted text —
    /// matching GNU `adjust_markers_for_replace` (insdel.c:341).
    pub(crate) fn adjust_markers_for_insert_extent_strict_after(
        &self,
        insert_pos: EmacsBytePos,
        extent: TextExtent,
    ) {
        let byte_len = extent.emacs_bytes().get();
        if byte_len == 0 {
            return;
        }
        let insert_pos = insert_pos.get();
        let char_len = extent.chars().get();
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: same invariant as adjust_markers_for_insert.
        unsafe {
            while !curr.is_null() {
                let data = &mut (*curr).data;
                if data.bytepos > insert_pos {
                    data.bytepos += byte_len;
                    data.charpos += char_len;
                }
                curr = data.next_marker;
            }
        }
    }

    pub(crate) fn adjust_markers_for_delete_range(&self, range: TextEditRange) {
        if range.is_empty() {
            return;
        }
        let start = range.byte_start_usize();
        let end = range.byte_end_usize();
        let start_char = range.char_start_usize();
        let byte_len = range.byte_len().get();
        let char_len = range.char_len().get();
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: same invariant as adjust_markers_for_insert.
        unsafe {
            while !curr.is_null() {
                let data = &mut (*curr).data;
                if data.bytepos >= end {
                    data.bytepos -= byte_len;
                    data.charpos -= char_len;
                } else if data.bytepos > start {
                    data.bytepos = start;
                    data.charpos = start_char;
                }
                curr = data.next_marker;
            }
        }
    }

    pub(crate) fn adjust_markers_for_replace_range(
        &self,
        old_range: TextEditRange,
        new_extent: TextExtent,
    ) {
        let new_byte_len = new_extent.emacs_bytes().get();
        let new_char_len = new_extent.chars().get();
        if old_range.is_empty() {
            self.adjust_markers_for_insert_extent(old_range.byte_start(), new_extent);
            return;
        }

        let start = old_range.byte_start_usize();
        let end = old_range.byte_end_usize();
        let start_char = old_range.char_start_usize();
        let old_byte_len = old_range.byte_len().get();
        let old_char_len = old_range.char_len().get();
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: same invariant as adjust_markers_for_insert.
        unsafe {
            while !curr.is_null() {
                let data = &mut (*curr).data;
                if data.bytepos >= end {
                    data.bytepos = data.bytepos + new_byte_len - old_byte_len;
                    data.charpos = data.charpos + new_char_len - old_char_len;
                } else if data.bytepos > start {
                    data.bytepos = start;
                    data.charpos = start_char;
                }
                curr = data.next_marker;
            }
        }
    }

    pub(crate) fn advance_markers_at_position(&self, pos: EmacsBytePos, extent: TextExtent) {
        let byte_len = extent.emacs_bytes().get();
        if byte_len == 0 {
            return;
        }
        let pos = pos.get();
        let char_len = extent.chars().get();
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: same invariant as adjust_markers_for_insert.
        unsafe {
            while !curr.is_null() {
                let data = &mut (*curr).data;
                if data.bytepos == pos {
                    data.bytepos += byte_len;
                    data.charpos += char_len;
                }
                curr = data.next_marker;
            }
        }
    }

    /// Unlink every chain node whose `LispMarker.buffer` is in `killed`.
    /// Sole entry point for kill-buffer marker cleanup: covers both the
    /// kill-root case (killed_set contains the root and all its
    /// indirects, so every marker on the shared chain matches) and the
    /// kill-indirect case (killed_set is just the dying indirect; root
    /// and sibling-indirect markers stay attached).
    pub fn remove_markers_for_buffers(&self, killed: &std::collections::HashSet<BufferId>) {
        let mut storage = self.storage.borrow_mut();
        let mut prev_slot: *mut *mut crate::tagged::header::MarkerObj = &mut storage.markers_head;
        // SAFETY: analogous to `chain_unlink`. Every non-null `*prev_slot`
        // was installed via `chain_splice_at_head`, i.e. a live GC-managed
        // MarkerObj with a valid `data.next_marker` link.
        unsafe {
            while !(*prev_slot).is_null() {
                let curr = *prev_slot;
                let data = &mut (*curr).data;
                let belongs_to_killed = data.buffer.map(|id| killed.contains(&id)).unwrap_or(false);
                if belongs_to_killed {
                    *prev_slot = data.next_marker;
                    data.next_marker = std::ptr::null_mut();
                    data.buffer = None;
                    // Preserve charpos/bytepos and last_position_valid so
                    // `marker-last-position` keeps GNU semantics across
                    // kill-buffer (cf. unchain_marker, marker.c:684).
                } else {
                    prev_slot = &mut data.next_marker;
                }
            }
        }
    }

    /// Retarget marker owners after `buffer-swap-text` moves this whole
    /// text object to another buffer. GNU swaps the marker chains with
    /// `struct buffer_text`, then rewrites each marker's `buffer` slot to
    /// the new owning buffer.
    pub(crate) fn retarget_markers_for_buffer_swap(&self, from: BufferId, to: BufferId) {
        let storage = self.storage.borrow();
        let mut curr = storage.markers_head;
        // SAFETY: same intrusive-chain invariant as `chain_unlink`; each node
        // was linked through this BufferText and remains live while present.
        unsafe {
            while !curr.is_null() {
                let data = &mut (*curr).data;
                if data.buffer == Some(from) {
                    data.buffer = Some(to);
                }
                curr = data.next_marker;
            }
        }
    }

    /// Raw pointer to the `markers_head` slot inside this buffer's
    /// storage. ONLY for GC use — bypasses RefCell's runtime borrow
    /// checks. Callers must hold exclusive access to the tagged heap and
    /// have no outstanding storage borrows (GC is stop-the-world).
    ///
    /// Used by `TaggedHeap::unchain_dead_markers` to splice unmarked
    /// MarkerObj nodes out of the intrusive per-buffer chain before
    /// `sweep_objects` frees them.
    pub unsafe fn markers_head_slot_raw(&self) -> *mut *mut crate::tagged::header::MarkerObj {
        let storage_ptr: *mut BufferTextStorage = self.storage.as_ptr();
        unsafe { &mut (*storage_ptr).markers_head as *mut _ }
    }

    /// Splice `marker` at the head of this buffer's marker chain.
    /// Overwrites `marker.next_marker` with the old head.
    /// Caller sets `marker.buffer` / `marker.bytepos` / `marker.charpos` —
    /// this helper only manipulates chain topology.
    ///
    /// **Precondition:** `marker.next_marker` must be null (marker is not
    /// currently on any chain). Violating this silently truncates the
    /// other chain. `debug_assert!` enforces it in debug builds.
    pub(crate) fn chain_splice_at_head(&self, marker: *mut crate::tagged::header::MarkerObj) {
        let mut storage = self.storage.borrow_mut();
        let old_head = storage.markers_head;
        unsafe {
            // SAFETY: `marker` must be a live MarkerObj allocated via
            // TaggedHeap::alloc_marker and not currently on any other chain
            // (see precondition above). Writing through the pointer is sound
            // because the heap retains ownership for the lifetime of the
            // MarkerObj.
            debug_assert!(
                (*marker).data.next_marker.is_null(),
                "chain_splice_at_head: marker is already on a chain"
            );
            (*marker).data.next_marker = old_head;
        }
        storage.markers_head = marker;
    }

    /// Unlink `marker` from this buffer's chain. Silent no-op if not present.
    /// Does NOT clear `marker.buffer` / positions — caller owns semantic cleanup.
    ///
    /// Unlike GNU `unchain_marker` (marker.c:684), which hard-asserts that
    /// the marker is in the chain, we tolerate absent markers. This is
    /// defensive: callers currently include code paths that may be
    /// double-invoked during GC sweep and kill-buffer cleanup in T8/T9.
    pub(crate) fn chain_unlink(&self, marker: *mut crate::tagged::header::MarkerObj) {
        let mut storage = self.storage.borrow_mut();
        let mut prev_slot: *mut *mut crate::tagged::header::MarkerObj = &mut storage.markers_head;
        // SAFETY: `prev_slot` walks the intrusive chain starting at
        // `storage.markers_head`. Every non-null `*prev_slot` is a
        // `*mut MarkerObj` previously installed via `chain_splice_at_head`,
        // i.e. a live GC-managed allocation whose `.data.next_marker` is
        // the next chain slot. We never read past a null terminator, and
        // mutations only rewrite chain-owned `next_marker` fields.
        unsafe {
            while !(*prev_slot).is_null() {
                let curr = *prev_slot;
                if curr == marker {
                    *prev_slot = (*curr).data.next_marker;
                    (*curr).data.next_marker = std::ptr::null_mut();
                    return;
                }
                prev_slot = &mut (*curr).data.next_marker;
            }
        }
    }

    /// Walk the chain from head to tail, collecting raw pointers in order.
    /// Test-only helper.
    #[cfg(test)]
    pub fn chain_walk_collect(&self) -> Vec<*mut crate::tagged::header::MarkerObj> {
        let storage = self.storage.borrow();
        let mut out = Vec::new();
        let mut curr = storage.markers_head;
        // SAFETY: Same invariant as `chain_unlink` — `curr` walks live
        // chain-owned MarkerObj pointers from `storage.markers_head`
        // until a null terminator.
        unsafe {
            while !curr.is_null() {
                out.push(curr);
                curr = (*curr).data.next_marker;
            }
        }
        out
    }

    /// Convert a character position to a logical Emacs byte offset using an
    /// anchor-bracketed cached search. Mirrors GNU `buf_charpos_to_bytepos`
    /// (`src/marker.c:167`).
    pub fn char_pos_to_emacs_byte_pos(&self, target: CharPos0) -> EmacsBytePos {
        let storage = self.storage.borrow();
        let metrics = storage.metrics;
        let total_chars = storage.metrics.chars();
        let total_bytes = storage.metrics.emacs_bytes();

        if target.get() >= total_chars {
            return metrics.emacs_byte_end();
        }

        // Unibyte fast path: char == byte, no scan needed.
        if total_chars == total_bytes {
            return EmacsBytePos::new(target.get());
        }

        // Wholesale-invalidate the anchor cache when the buffer changed.
        if storage.anchor_cache_key.get() != metrics {
            storage.anchor_cache.borrow_mut().clear();
            storage.anchor_cache_key.set(metrics);
        }

        let mut best_below = TextPositionAnchor::default();
        let mut best_above = TextPositionAnchor::new(metrics.char_end(), metrics.emacs_byte_end());

        if let Some(anchor) = storage.backend.position_conversion_hint() {
            consider_char_anchor(target, anchor, &mut best_below, &mut best_above);
        }

        let cached = storage.pos_cache.get();
        if cached.is_valid_for(metrics) {
            consider_char_anchor(target, cached.anchor, &mut best_below, &mut best_above);
        }

        for &anchor in storage.anchor_cache.borrow().iter() {
            consider_char_anchor(target, anchor, &mut best_below, &mut best_above);
        }

        let mut distance: usize = POSITION_DISTANCE_BASE;
        // T7: marker chain walk. The chain carries the same (char, byte)
        // pairs that the deleted Vec<MarkerEntry> used to.
        //
        // SAFETY: `curr` walks live chain-owned MarkerObj pointers from
        // `storage.markers_head` until null. Each non-null node was
        // spliced in via `chain_splice_at_head`, so its `data.next_marker`
        // is a valid chain link or null.
        let mut curr = storage.markers_head;
        unsafe {
            while !curr.is_null() {
                let data = &(*curr).data;
                consider_char_anchor(
                    target,
                    TextPositionAnchor::from_usize(data.charpos, data.bytepos),
                    &mut best_below,
                    &mut best_above,
                );
                if best_above.char_pos.get().saturating_sub(target.get()) < distance
                    || target.get().saturating_sub(best_below.char_pos.get()) < distance
                {
                    break;
                }
                distance = distance.saturating_add(POSITION_DISTANCE_INCR);
                curr = data.next_marker;
            }
        }

        let walked_below = target.get().saturating_sub(best_below.char_pos.get());
        let walked_above = best_above.char_pos.get().saturating_sub(target.get());
        let result = if walked_below <= walked_above {
            scan_forward(&storage.backend, best_below, target)
        } else {
            scan_backward(&storage.backend, best_above, target)
        };

        // Mirror GNU marker.c:238-241: insert an anchor when the scan actually
        // walked more than POSITION_ANCHOR_STRIDE positions.
        let walked = walked_below.min(walked_above);
        if walked > POSITION_ANCHOR_STRIDE {
            storage
                .anchor_cache
                .borrow_mut()
                .push(TextPositionAnchor::new(target, result));
        }

        storage.pos_cache.set(PositionCache {
            epoch: metrics,
            anchor: TextPositionAnchor::new(target, result),
        });
        result
    }

    /// Convert a logical Emacs byte position to a character position. Symmetric
    /// to `buf_charpos_to_bytepos` — shares the same anchor + cache machinery.
    pub fn emacs_byte_pos_to_char_pos(&self, target: EmacsBytePos) -> CharPos0 {
        let storage = self.storage.borrow();
        let metrics = storage.metrics;
        let total_chars = storage.metrics.chars();
        let total_bytes = storage.metrics.emacs_bytes();

        if target.get() >= total_bytes {
            return metrics.char_end();
        }

        // Unibyte fast path: char == byte, no scan needed.
        if total_chars == total_bytes {
            return CharPos0::new(target.get());
        }

        // Wholesale-invalidate the anchor cache when the buffer changed.
        if storage.anchor_cache_key.get() != metrics {
            storage.anchor_cache.borrow_mut().clear();
            storage.anchor_cache_key.set(metrics);
        }

        let mut best_below = TextPositionAnchor::default();
        let mut best_above = TextPositionAnchor::new(metrics.char_end(), metrics.emacs_byte_end());

        if let Some(anchor) = storage.backend.position_conversion_hint() {
            consider_byte_anchor(target, anchor, &mut best_below, &mut best_above);
        }

        let cached = storage.pos_cache.get();
        if cached.is_valid_for(metrics) {
            consider_byte_anchor(target, cached.anchor, &mut best_below, &mut best_above);
        }

        for &anchor in storage.anchor_cache.borrow().iter() {
            consider_byte_anchor(target, anchor, &mut best_below, &mut best_above);
        }

        let mut distance: usize = POSITION_DISTANCE_BASE;
        // T7: marker chain walk. See sibling comment in
        // `buf_charpos_to_bytepos` for the SAFETY rationale.
        let mut curr = storage.markers_head;
        unsafe {
            while !curr.is_null() {
                let data = &(*curr).data;
                consider_byte_anchor(
                    target,
                    TextPositionAnchor::from_usize(data.charpos, data.bytepos),
                    &mut best_below,
                    &mut best_above,
                );
                if best_above.emacs_byte_pos.get().saturating_sub(target.get()) < distance
                    || target.get().saturating_sub(best_below.emacs_byte_pos.get()) < distance
                {
                    break;
                }
                distance = distance.saturating_add(POSITION_DISTANCE_INCR);
                curr = data.next_marker;
            }
        }

        let walked_below = target.get().saturating_sub(best_below.emacs_byte_pos.get());
        let walked_above = best_above.emacs_byte_pos.get().saturating_sub(target.get());
        let result = if walked_below <= walked_above {
            scan_forward_bytes(&storage.backend, best_below, target)
        } else {
            scan_backward_bytes(&storage.backend, best_above, target)
        };

        // Mirror GNU marker.c:238-241: insert an anchor when the scan actually
        // walked more than POSITION_ANCHOR_STRIDE positions.
        // Store as (charpos, bytepos) like the char→byte direction to keep
        // anchor_cache entries in one canonical order.
        let walked = walked_below.min(walked_above);
        if walked > POSITION_ANCHOR_STRIDE {
            storage
                .anchor_cache
                .borrow_mut()
                .push(TextPositionAnchor::new(result, target));
        }

        storage.pos_cache.set(PositionCache {
            epoch: metrics,
            anchor: TextPositionAnchor::new(result, target),
        });
        result
    }

    #[cfg(test)]
    pub fn buf_charpos_to_bytepos(&self, target: usize) -> usize {
        self.char_pos_to_emacs_byte_pos(CharPos0::new(target)).get()
    }

    #[cfg(test)]
    pub fn buf_bytepos_to_charpos(&self, target: usize) -> usize {
        self.emacs_byte_pos_to_char_pos(EmacsBytePos::new(target))
            .get()
    }

    #[cfg(test)]
    pub fn anchor_cache_len(&self) -> usize {
        self.storage.borrow().anchor_cache.borrow().len()
    }
}

impl fmt::Display for BufferText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.storage.borrow().backend.fmt(f)
    }
}

impl fmt::Debug for BufferText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufferText")
            .field("len", &self.len())
            .field("chars", &self.char_count())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Position conversion helpers
// ---------------------------------------------------------------------------

/// GNU `marker.c:162` — initial bracket-bail distance.
const POSITION_DISTANCE_BASE: usize = 50;
/// GNU `marker.c:162` — bracket-bail distance grows by this per marker checked.
const POSITION_DISTANCE_INCR: usize = 50;
/// Auto-insert an anchor when a scan walks more than this many positions.
/// Mirrors GNU `marker.c:238-241` (5000-char threshold).
const POSITION_ANCHOR_STRIDE: usize = 5000;

/// Update `(best_below, best_above)` in place by character position.
fn consider_char_anchor(
    target: CharPos0,
    anchor: TextPositionAnchor,
    best_below: &mut TextPositionAnchor,
    best_above: &mut TextPositionAnchor,
) {
    if anchor.char_pos <= target && anchor.char_pos > best_below.char_pos {
        *best_below = anchor;
    }
    if anchor.char_pos >= target && anchor.char_pos < best_above.char_pos {
        *best_above = anchor;
    }
}

/// Walk forward from `anchor` to reach `target` chars.
/// Returns the byte position.
fn scan_forward(
    backend: &TextBackend,
    anchor: TextPositionAnchor,
    target: CharPos0,
) -> EmacsBytePos {
    let mut cp = anchor.char_pos.get();
    let mut bp = anchor.emacs_byte_pos.get();
    let total_bytes = backend.metrics().emacs_bytes();
    while cp < target.get() {
        if !backend.is_multibyte() {
            bp += 1;
            cp += 1;
            continue;
        }
        let mut tmp = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
        let available = (total_bytes - bp).min(tmp.len());
        for (i, slot) in tmp[..available].iter_mut().enumerate() {
            *slot = backend.byte_at_emacs_byte_pos(EmacsBytePos::new(bp + i));
        }
        let (_, len) = crate::emacs_core::emacs_char::string_char(&tmp[..available]);
        bp += len;
        cp += 1;
    }
    EmacsBytePos::new(bp)
}

/// Walk backward from `anchor` to reach `target` chars.
/// Returns the byte position.
fn scan_backward(
    backend: &TextBackend,
    anchor: TextPositionAnchor,
    target: CharPos0,
) -> EmacsBytePos {
    let mut cp = anchor.char_pos.get();
    let mut bp = anchor.emacs_byte_pos.get();
    while cp > target.get() {
        if !backend.is_multibyte() {
            bp -= 1;
            cp -= 1;
            continue;
        }
        let mut prev = bp - 1;
        while prev > 0 && (backend.byte_at_emacs_byte_pos(EmacsBytePos::new(prev)) & 0xC0) == 0x80 {
            prev -= 1;
        }
        bp = prev;
        cp -= 1;
    }
    EmacsBytePos::new(bp)
}

/// Update `(best_below, best_above)` in place by byte position.
fn consider_byte_anchor(
    target: EmacsBytePos,
    anchor: TextPositionAnchor,
    best_below: &mut TextPositionAnchor,
    best_above: &mut TextPositionAnchor,
) {
    if anchor.emacs_byte_pos <= target && anchor.emacs_byte_pos > best_below.emacs_byte_pos {
        *best_below = anchor;
    }
    if anchor.emacs_byte_pos >= target && anchor.emacs_byte_pos < best_above.emacs_byte_pos {
        *best_above = anchor;
    }
}

/// Walk forward from `anchor` to reach `target` bytepos.
/// Returns the char position.
fn scan_forward_bytes(
    backend: &TextBackend,
    anchor: TextPositionAnchor,
    target: EmacsBytePos,
) -> CharPos0 {
    let mut bp = anchor.emacs_byte_pos.get();
    let mut cp = anchor.char_pos.get();
    let total_bytes = backend.metrics().emacs_bytes();
    while bp < target.get() {
        if !backend.is_multibyte() {
            bp += 1;
            cp += 1;
            continue;
        }
        let mut tmp = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
        let available = (total_bytes - bp).min(tmp.len());
        for (i, slot) in tmp[..available].iter_mut().enumerate() {
            *slot = backend.byte_at_emacs_byte_pos(EmacsBytePos::new(bp + i));
        }
        let (_, len) = crate::emacs_core::emacs_char::string_char(&tmp[..available]);
        bp += len;
        cp += 1;
    }
    CharPos0::new(cp)
}

/// Walk backward from `anchor` to reach `target` bytepos.
/// Returns the char position.
fn scan_backward_bytes(
    backend: &TextBackend,
    anchor: TextPositionAnchor,
    target: EmacsBytePos,
) -> CharPos0 {
    let mut bp = anchor.emacs_byte_pos.get();
    let mut cp = anchor.char_pos.get();
    while bp > target.get() {
        if !backend.is_multibyte() {
            bp -= 1;
            cp -= 1;
            continue;
        }
        let mut prev = bp - 1;
        while prev > 0 && (backend.byte_at_emacs_byte_pos(EmacsBytePos::new(prev)) & 0xC0) == 0x80 {
            prev -= 1;
        }
        bp = prev;
        cp -= 1;
    }
    CharPos0::new(cp)
}

#[cfg(test)]
#[path = "buffer_text_test.rs"]
mod tests;

#[cfg(test)]
#[path = "buffer_text_chain_test.rs"]
mod chain_test;
