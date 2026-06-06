//! A raw Emacs-byte gap buffer for efficient text editing.
//!
//! The gap buffer stores text in a contiguous `Vec<u8>` with a movable "gap"
//! (unused region) that makes insertions and deletions near the gap O(1)
//! amortized. The gap is relocated to the edit site before each mutation so
//! that sequential edits in the same neighborhood avoid large copies.
//!
//! Raw internal helpers use byte positions into the logical text (i.e. the
//! text with the gap removed). Cross-module entrypoints use typed
//! `EmacsBytePos`/`EmacsByteRange` plus measured edit types, so callers cannot
//! accidentally mix Emacs-byte and character coordinates at the backend
//! boundary. The underlying bytes are Emacs internal bytes, not
//! sentinel-encoded Rust strings.

use std::cell::Cell;
use std::fmt;

use crate::buffer::text::{GapCompatState, emacs_char_count_bytes, emacs_char_to_byte_in_slice};
use crate::buffer::{
    CharLen, CharPos0, EmacsBytePos, EmacsByteRange, TextEditRange, TextExtent, TextPositionAnchor,
    TextReplacement,
};

/// Default extra gap bytes to pre-allocate on any growth.
/// Matches GNU Emacs `GAP_BYTES_DFL` (`src/buffer.h:205`).
pub(crate) const GAP_BYTES_DFL: usize = 2000;

/// Floor for the gap after shrinking — not enforced today because we don't
/// shrink yet, but kept as a named constant to match GNU's `GAP_BYTES_MIN`
/// (`src/buffer.h:210`).
#[allow(dead_code)]
pub(crate) const GAP_BYTES_MIN: usize = 20;

/// A gap buffer holding raw Emacs bytes.
///
/// Internally the backing store looks like:
///
/// ```text
///  [ text-before-gap | gap (unused) | text-after-gap ]
///    0..gap_start      gap_start..gap_end  gap_end..buf.len()
/// ```
///
/// The *logical* text is the concatenation of `buf[..gap_start]` and
/// `buf[gap_end..]`.
#[derive(Clone)]
pub struct GapBuffer {
    /// Raw backing store.
    buf: Vec<u8>,
    /// Whether the logical text should be interpreted as a multibyte buffer.
    multibyte: bool,
    /// Byte index where the gap begins (first unused byte).
    gap_start: usize,
    /// Byte index one past the last gap byte (first byte of text after gap).
    gap_end: usize,
    /// Number of logical Emacs characters before the gap.
    gap_start_chars: usize,
    /// Number of logical Emacs characters in the buffer.
    total_chars: usize,
    /// Number of logical Emacs bytes before the gap.
    gap_start_bytes: usize,
    /// Number of logical Emacs bytes in the buffer.
    total_bytes: usize,
    /// One-entry cache of a known `(logical_bytepos, logical_charpos)`
    /// correspondence (GNU `marker.c`'s `cached_bytepos`/`cached_charpos`).
    /// Used as an extra anchor so sequential position conversions (e.g.
    /// font-lock walking the buffer) scan O(distance between calls) instead of
    /// O(buffer) from the start.  Reset to `(0, 0)` whenever the logical text
    /// length changes; `(0, 0)` is always a valid anchor.
    byte_char_cache: Cell<(usize, usize)>,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl GapBuffer {
    /// Create an empty gap buffer with a default-sized gap.
    pub fn new() -> Self {
        Self::new_with_multibyte(true)
    }

    pub fn new_with_multibyte(multibyte: bool) -> Self {
        Self {
            buf: vec![0u8; GAP_BYTES_MIN],
            multibyte,
            gap_start: 0,
            gap_end: GAP_BYTES_MIN,
            gap_start_chars: 0,
            total_chars: 0,
            gap_start_bytes: 0,
            total_bytes: 0,
            byte_char_cache: Cell::new((0, 0)),
        }
    }

    /// Create a gap buffer pre-loaded with raw Emacs bytes.
    pub fn from_emacs_bytes(text: &[u8], multibyte: bool) -> Self {
        let gap = GAP_BYTES_DFL;
        let char_count = emacs_char_count_bytes(text, multibyte).get();
        let byte_count = text.len();
        let mut buf = Vec::with_capacity(text.len() + gap);
        buf.extend_from_slice(text);
        buf.resize(text.len() + gap, 0);
        Self {
            buf,
            multibyte,
            gap_start: text.len(),
            gap_end: text.len() + gap,
            gap_start_chars: char_count,
            total_chars: char_count,
            gap_start_bytes: byte_count,
            total_bytes: byte_count,
            byte_char_cache: Cell::new((0, 0)),
        }
    }

    pub(in crate::buffer) fn from_emacs_bytes_with_gap_compat_state(
        text: &[u8],
        multibyte: bool,
        gap_state: GapCompatState,
    ) -> Self {
        let total_chars = emacs_char_count_bytes(text, multibyte).get();
        let gap_start_chars = gap_state.pos().get();
        assert!(
            gap_start_chars <= total_chars,
            "from_emacs_bytes_with_gap_compat_state: gap char position {gap_start_chars} out of range ({total_chars})",
        );
        let gap_start_bytes = emacs_char_to_byte_in_slice(text, gap_start_chars, multibyte);
        let gap = gap_state.byte_len().get();
        let mut buf = Vec::with_capacity(text.len() + gap);
        buf.extend_from_slice(&text[..gap_start_bytes]);
        buf.resize(gap_start_bytes + gap, 0);
        buf.extend_from_slice(&text[gap_start_bytes..]);
        Self {
            buf,
            multibyte,
            gap_start: gap_start_bytes,
            gap_end: gap_start_bytes + gap,
            gap_start_chars,
            total_chars,
            gap_start_bytes,
            total_bytes: text.len(),
            byte_char_cache: Cell::new((0, 0)),
        }
    }

    /// Create a gap buffer pre-loaded with the contents of `s`.
    pub fn from_str(s: &str) -> Self {
        let decoded = super::text::storage_string_to_emacs_buffer_bytes(s);
        Self::from_emacs_bytes(decoded.bytes(), decoded.multibyte())
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Total length of the logical text in **bytes** (excluding the gap).
    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len() - self.gap_size()
    }

    /// Whether the buffer contains no text.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_multibyte(&self) -> bool {
        self.multibyte
    }

    pub fn set_multibyte(&mut self, multibyte: bool) {
        if self.multibyte == multibyte {
            return;
        }
        self.multibyte = multibyte;
        let mut logical = Vec::with_capacity(self.len());
        self.copy_emacs_byte_range_to(
            EmacsByteRange::new(EmacsBytePos::ZERO, EmacsBytePos::new(self.len())),
            &mut logical,
        );
        self.gap_start_chars =
            emacs_char_count_bytes(&logical[..self.gap_start], self.multibyte).get();
        self.total_chars = emacs_char_count_bytes(&logical, self.multibyte).get();
        self.gap_start_bytes = self.gap_start;
        self.total_bytes = logical.len();
        self.byte_char_cache.set((0, 0));
    }

    /// Number of logical Emacs characters in the buffer storage.
    pub fn char_count(&self) -> usize {
        self.total_chars
    }

    /// Number of logical Emacs bytes in the buffer.
    pub fn emacs_byte_len(&self) -> usize {
        self.total_bytes
    }

    /// GNU `GPT`: character position of the gap.
    pub fn gpt(&self) -> usize {
        self.gap_start_chars
    }

    /// GNU `Z`: character position of the end of buffer text.
    pub fn z(&self) -> usize {
        self.total_chars
    }

    /// GNU `GPT_BYTE`: logical Emacs byte position of the gap.
    pub fn gpt_byte(&self) -> usize {
        self.gap_start_bytes
    }

    /// GNU `Z_BYTE`: logical Emacs byte position of the end of buffer text.
    pub fn z_byte(&self) -> usize {
        self.total_bytes
    }

    /// Size of the gap in bytes.
    #[inline]
    pub fn gap_size(&self) -> usize {
        self.gap_end - self.gap_start
    }

    // -----------------------------------------------------------------------
    // Single-element access
    // -----------------------------------------------------------------------

    /// Return the byte at logical position `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos >= self.len()`.
    fn byte_at(&self, pos: usize) -> u8 {
        assert!(
            pos < self.len(),
            "byte_at: position {pos} out of range (len {})",
            self.len()
        );
        if pos < self.gap_start {
            self.buf[pos]
        } else {
            self.buf[pos + self.gap_size()]
        }
    }

    pub(crate) fn byte_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> u8 {
        self.byte_at(pos.get())
    }

    /// Return the logical Emacs byte at `pos`, or `None` if out of range.
    fn emacs_byte_at(&self, pos: usize) -> Option<u8> {
        (pos < self.total_bytes).then(|| self.byte_at(pos))
    }

    pub(crate) fn emacs_byte_at_pos(&self, pos: EmacsBytePos) -> Option<u8> {
        self.emacs_byte_at(pos.get())
    }

    pub(crate) fn char_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<char> {
        self.char_code_at_emacs_byte_pos(pos)
            .and_then(char::from_u32)
    }

    /// Return the Emacs character code whose first byte begins at logical
    /// byte position `pos`, or `None` if `pos >= self.len()`.
    fn char_code_at(&self, pos: usize) -> Option<u32> {
        if pos >= self.len() {
            return None;
        }
        assert!(
            self.is_char_boundary(pos),
            "char_code_at: byte position {pos} is not a character boundary"
        );
        if !self.multibyte {
            return Some(self.byte_at(pos) as u32);
        }

        let mut tmp = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
        let available = (self.len() - pos).min(tmp.len());
        for (i, slot) in tmp[..available].iter_mut().enumerate() {
            *slot = self.byte_at(pos + i);
        }
        Some(crate::emacs_core::emacs_char::string_char(&tmp[..available]).0)
    }

    pub(crate) fn char_code_at_emacs_byte_pos(&self, pos: EmacsBytePos) -> Option<u32> {
        self.char_code_at(pos.get())
    }

    // -----------------------------------------------------------------------
    // Range extraction
    // -----------------------------------------------------------------------

    pub(crate) fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String {
        let start = range.start().get();
        let end = range.end().get();
        assert!(start <= end, "text_range: start ({start}) > end ({end})");
        assert!(
            end <= self.len(),
            "text_range: end ({end}) > len ({})",
            self.len()
        );
        if start == end {
            return String::new();
        }
        let mut out = Vec::with_capacity(end - start);
        self.copy_emacs_byte_range_to(range, &mut out);
        crate::emacs_core::string_escape::emacs_bytes_to_storage_string(&out, self.multibyte)
    }

    pub(crate) fn copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>) {
        let start = range.start().get();
        let end = range.end().get();
        assert!(
            start <= end,
            "copy_emacs_bytes_to: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.total_bytes,
            "copy_emacs_bytes_to: end ({end}) > emacs len ({})",
            self.total_bytes
        );
        out.clear();
        if start == end {
            return;
        }
        out.reserve(end - start);

        // Intersection with segment A (logical 0..gap_start).
        if start < self.gap_start {
            let seg_end = end.min(self.gap_start);
            out.extend_from_slice(&self.buf[start..seg_end]);
        }

        // Intersection with segment B (logical gap_start..len).
        if end > self.gap_start {
            let seg_start = start.max(self.gap_start);
            let phys_start = seg_start + self.gap_size();
            let phys_end = end + self.gap_size();
            out.extend_from_slice(&self.buf[phys_start..phys_end]);
        }
    }

    pub(crate) fn for_each_emacs_byte_range_chunk<E>(
        &self,
        range: EmacsByteRange,
        mut f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        let start = range.start().get();
        let end = range.end().get();
        assert!(
            start <= end,
            "for_each_emacs_byte_chunk: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.total_bytes,
            "for_each_emacs_byte_chunk: end ({end}) > emacs len ({})",
            self.total_bytes
        );
        if start == end {
            return Ok(());
        }
        if end <= self.gap_start_bytes {
            return f(&self.buf[start..end]);
        }
        if start >= self.gap_start_bytes {
            let gap = self.gap_size();
            return f(&self.buf[start + gap..end + gap]);
        }

        f(&self.buf[start..self.gap_start])?;
        let gap = self.gap_size();
        f(&self.buf[self.gap_start + gap..end + gap])
    }

    pub(crate) fn has_contiguous_emacs_byte_range(&self, range: EmacsByteRange) -> bool {
        let start = range.start().get();
        let end = range.end().get();
        assert!(
            start <= end,
            "has_contiguous_emacs_bytes: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.total_bytes,
            "has_contiguous_emacs_bytes: end ({end}) > emacs len ({})",
            self.total_bytes
        );
        start == end || end <= self.gap_start_bytes || start >= self.gap_start_bytes
    }

    pub(crate) fn with_contiguous_emacs_byte_range<R>(
        &self,
        range: EmacsByteRange,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Option<R> {
        let start = range.start().get();
        let end = range.end().get();
        assert!(
            start <= end,
            "with_contiguous_emacs_bytes: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.total_bytes,
            "with_contiguous_emacs_bytes: end ({end}) > emacs len ({})",
            self.total_bytes
        );
        if start == end {
            return Some(f(&[]));
        }
        if end <= self.gap_start_bytes {
            return Some(f(&self.buf[start..end]));
        }
        if start >= self.gap_start_bytes {
            let gap = self.gap_size();
            return Some(f(&self.buf[start + gap..end + gap]));
        }
        None
    }

    // -----------------------------------------------------------------------
    // Mutation
    // -----------------------------------------------------------------------

    pub(crate) fn insert_emacs_bytes_at_emacs_byte_pos(&mut self, pos: EmacsBytePos, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.insert_measured_emacs_bytes(
            pos,
            bytes,
            TextExtent::from_emacs_bytes(bytes, self.multibyte),
        );
    }

    pub(crate) fn insert_emacs_bytes_at_emacs_byte_pos_with_char_len(
        &mut self,
        pos: EmacsBytePos,
        bytes: &[u8],
        char_len: CharLen,
    ) {
        self.insert_measured_emacs_bytes(
            pos,
            bytes,
            TextExtent::new(char_len, crate::buffer::EmacsByteLen::new(bytes.len())),
        );
    }

    pub(crate) fn insert_measured_emacs_bytes(
        &mut self,
        pos: EmacsBytePos,
        bytes: &[u8],
        extent: TextExtent,
    ) {
        let pos = pos.get();
        let nchars = extent.chars().get();
        assert!(
            pos <= self.len(),
            "insert_emacs_bytes_both: position {pos} out of range (len {})",
            self.len()
        );
        if bytes.is_empty() {
            return;
        }
        debug_assert!(
            pos == self.len() || self.is_char_boundary(pos),
            "insert_emacs_bytes_both: position {pos} is not on an Emacs character boundary"
        );
        debug_assert_eq!(
            extent.emacs_bytes().get(),
            bytes.len(),
            "insert_emacs_bytes_both: caller-supplied byte count mismatches actual"
        );
        debug_assert_eq!(
            CharLen::new(nchars),
            emacs_char_count_bytes(bytes, self.multibyte),
            "insert_emacs_bytes_both: caller-supplied nchars mismatches actual"
        );

        let inserted_bytes = bytes.len();
        self.move_gap_to(pos);
        self.ensure_gap(inserted_bytes);

        self.buf[self.gap_start..self.gap_start + inserted_bytes].copy_from_slice(bytes);
        self.gap_start += inserted_bytes;
        self.gap_start_chars += nchars;
        self.total_chars += nchars;
        self.gap_start_bytes += inserted_bytes;
        self.total_bytes += inserted_bytes;
        self.byte_char_cache.set((0, 0));
    }

    pub(crate) fn insert_storage_string_at_emacs_byte_pos(&mut self, pos: EmacsBytePos, s: &str) {
        if s.is_empty() {
            return;
        }
        let bytes =
            crate::emacs_core::string_escape::storage_string_to_buffer_bytes(s, self.multibyte);
        self.insert_emacs_bytes_at_emacs_byte_pos(pos, &bytes);
    }

    pub(crate) fn delete_emacs_byte_range(&mut self, range: EmacsByteRange) {
        let start = range.start().get();
        let end = range.end().get();
        assert!(start <= end, "delete_range: start ({start}) > end ({end})");
        assert!(
            end <= self.len(),
            "delete_range: end ({end}) > len ({})",
            self.len()
        );
        if start == end {
            return;
        }
        // Count chars in the about-to-be-deleted region. This is the scan that
        // delete_emacs_byte_range_with_char_len lets callers skip.
        let mut tmp = Vec::with_capacity(end - start);
        self.copy_emacs_byte_range_to(range, &mut tmp);
        let nchars = emacs_char_count_bytes(&tmp, self.multibyte);
        self.delete_emacs_byte_range_with_char_len(range, nchars);
    }

    /// Delete the logical byte range `[start, end)`, given pre-computed char
    /// count of the region.
    ///
    /// Mirrors GNU `del_range_2` (`src/insdel.c:1991`).
    pub(crate) fn delete_emacs_byte_range_with_char_len(
        &mut self,
        range: EmacsByteRange,
        char_len: CharLen,
    ) {
        let start_char = self.emacs_byte_pos_to_char_pos(range.start());
        self.delete_measured_range(TextEditRange::from_start_extent(
            range.start(),
            start_char,
            TextExtent::new(char_len, range.len()),
        ));
    }

    pub(crate) fn delete_measured_range(&mut self, range: TextEditRange) {
        let start = range.byte_start().get();
        let end = range.byte_end().get();
        let nchars = range.char_len().get();
        assert!(
            start <= end,
            "delete_range_both: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.len(),
            "delete_range_both: end ({end}) > len ({})",
            self.len()
        );
        if start == end {
            return;
        }
        debug_assert!(
            self.is_char_boundary(start),
            "delete_range_both: start ({start}) is not on an Emacs character boundary"
        );
        debug_assert!(
            end == self.len() || self.is_char_boundary(end),
            "delete_range_both: end ({end}) is not on an Emacs character boundary"
        );
        debug_assert_eq!(
            nchars,
            self.emacs_byte_pos_to_char_pos(EmacsBytePos::new(end))
                .get()
                - self
                    .emacs_byte_pos_to_char_pos(EmacsBytePos::new(start))
                    .get(),
            "delete_range_both: caller-supplied nchars mismatches actual"
        );

        self.move_gap_to(start);
        let deleted_bytes = end - start;
        // After move_gap_to(start), bytes [start, end) now live at
        // buf[gap_end .. gap_end + deleted_bytes]; extend the gap to swallow them.
        self.gap_end += deleted_bytes;
        self.total_chars -= nchars;
        self.total_bytes -= deleted_bytes;
        self.byte_char_cache.set((0, 0));
    }

    pub(crate) fn replace_measured_range(&mut self, replacement: TextReplacement, bytes: &[u8]) {
        let old_range = replacement.old_range();
        if old_range.is_empty() {
            self.insert_measured_emacs_bytes(
                replacement.byte_start(),
                bytes,
                replacement.new_extent(),
            );
            return;
        }
        if bytes.is_empty() {
            self.delete_measured_range(old_range);
            return;
        }
        self.delete_measured_range(old_range);
        self.insert_measured_emacs_bytes(replacement.byte_start(), bytes, replacement.new_extent());
    }

    pub(crate) fn replace_same_len_emacs_byte_range(
        &mut self,
        range: EmacsByteRange,
        replacement: &[u8],
    ) {
        let start_char = self.emacs_byte_pos_to_char_pos(range.start());
        let end_char = self.emacs_byte_pos_to_char_pos(range.end());
        self.replace_same_len_measured_range(
            TextReplacement::new(
                TextEditRange::from_start_end(
                    TextPositionAnchor::new(start_char, range.start()),
                    TextPositionAnchor::new(end_char, range.end()),
                ),
                TextExtent::from_emacs_bytes(replacement, self.multibyte),
            ),
            replacement,
        );
    }

    pub(crate) fn replace_same_len_measured_range(
        &mut self,
        replacement: TextReplacement,
        bytes: &[u8],
    ) {
        let start = replacement.old_range().byte_start().get();
        let end = replacement.old_range().byte_end().get();
        assert!(
            start <= end,
            "replace_same_len_range: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.len(),
            "replace_same_len_range: end ({end}) > len ({})",
            self.len()
        );
        assert_eq!(
            bytes.len(),
            end - start,
            "replace_same_len_range: replacement Emacs-byte length ({}) must match replaced length ({})",
            bytes.len(),
            end - start
        );
        assert_eq!(
            replacement.new_byte_len().get(),
            bytes.len(),
            "replace_same_len_range: measured new byte length ({}) mismatches replacement bytes ({})",
            replacement.new_byte_len().get(),
            bytes.len()
        );
        if start == end {
            return;
        }
        debug_assert!(
            self.is_char_boundary(start),
            "replace_same_len_range: start ({start}) is not on an Emacs character boundary"
        );
        debug_assert!(
            end == self.len() || self.is_char_boundary(end),
            "replace_same_len_range: end ({end}) is not on an Emacs character boundary"
        );

        let before_gap_len = if start < self.gap_start_bytes {
            end.min(self.gap_start_bytes) - start
        } else {
            0
        };
        let after_gap_len = if end > self.gap_start_bytes {
            end - start.max(self.gap_start_bytes)
        } else {
            0
        };
        let gap = self.gap_size();
        let old_before_chars = if before_gap_len == 0 {
            CharLen::ZERO
        } else {
            emacs_char_count_bytes(&self.buf[start..start + before_gap_len], self.multibyte)
        };
        let old_after_chars = if after_gap_len == 0 {
            CharLen::ZERO
        } else {
            let phys_start = start.max(self.gap_start_bytes) + gap;
            emacs_char_count_bytes(
                &self.buf[phys_start..phys_start + after_gap_len],
                self.multibyte,
            )
        };
        debug_assert_eq!(
            replacement.new_char_len(),
            emacs_char_count_bytes(bytes, self.multibyte),
            "replace_same_len_range: measured new char count mismatches replacement bytes"
        );
        debug_assert_eq!(
            replacement.old_char_len(),
            old_before_chars.add_len(old_after_chars),
            "replace_same_len_range: measured old char count mismatches storage"
        );

        if before_gap_len != 0 {
            self.buf[start..start + before_gap_len].copy_from_slice(&bytes[..before_gap_len]);
        }
        if after_gap_len != 0 {
            let src_start = before_gap_len;
            let phys_start = start.max(self.gap_start_bytes) + gap;
            self.buf[phys_start..phys_start + after_gap_len]
                .copy_from_slice(&bytes[src_start..src_start + after_gap_len]);
        }

        let old_chars = replacement.old_char_len().get();
        let new_chars = replacement.new_char_len().get();
        let new_before_chars = emacs_char_count_bytes(&bytes[..before_gap_len], self.multibyte);
        if old_before_chars != new_before_chars {
            let delta = new_before_chars.get() as isize - old_before_chars.get() as isize;
            self.gap_start_chars = self.gap_start_chars.saturating_add_signed(delta);
        }
        if old_chars != new_chars {
            let delta = new_chars as isize - old_chars as isize;
            self.total_chars = self.total_chars.saturating_add_signed(delta);
        }
        // An in-place replacement can shift char boundaries inside the replaced
        // range even when the total counts are unchanged, so invalidate the
        // position cache unconditionally.
        self.byte_char_cache.set((0, 0));
    }

    // -----------------------------------------------------------------------
    // Gap management
    // -----------------------------------------------------------------------

    /// Move the gap so that `gap_start == pos`.
    ///
    /// Wrapper that computes the char delta by scanning moved bytes. Prefer
    /// `move_gap_both` when the caller knows the target char position.
    fn move_gap_to(&mut self, pos: usize) {
        assert!(
            pos <= self.len(),
            "move_gap_to: position {pos} out of range (len {})",
            self.len()
        );
        if pos == self.gap_start {
            return;
        }
        // Derive the target char position by scanning moved bytes. The scan is
        // exactly what move_gap_both lets the caller skip.
        let charpos = if pos < self.gap_start {
            let moved = emacs_char_count_bytes(&self.buf[pos..self.gap_start], self.multibyte);
            self.gap_start_chars - moved.get()
        } else {
            let moved = emacs_char_count_bytes(
                &self.buf[self.gap_end..self.gap_end + (pos - self.gap_start)],
                self.multibyte,
            );
            self.gap_start_chars + moved.get()
        };
        self.move_gap_to_emacs_byte_pos_and_char_pos(
            EmacsBytePos::new(pos),
            CharPos0::new(charpos),
        );
    }

    pub(crate) fn move_gap_to_emacs_byte_pos_and_char_pos(
        &mut self,
        bytepos: EmacsBytePos,
        charpos: CharPos0,
    ) {
        let bytepos = bytepos.get();
        let charpos = charpos.get();
        assert!(
            bytepos <= self.len(),
            "move_gap_both: bytepos {bytepos} out of range (len {})",
            self.len()
        );
        if bytepos == self.gap_start {
            return;
        }
        let gap = self.gap_size();

        if bytepos < self.gap_start {
            let count = self.gap_start - bytepos;
            self.buf
                .copy_within(bytepos..bytepos + count, bytepos + gap);
        } else {
            let count = bytepos - self.gap_start;
            let src_start = self.gap_end;
            let dst_start = self.gap_start;
            self.buf
                .copy_within(src_start..src_start + count, dst_start);
        }
        self.gap_start = bytepos;
        self.gap_end = bytepos + gap;
        self.gap_start_chars = charpos;
        self.gap_start_bytes = bytepos;
    }

    /// Ensure the gap is at least `min_size` bytes. If it is already large
    /// enough this is a no-op; otherwise the backing buffer is reallocated.
    pub fn ensure_gap(&mut self, min_size: usize) {
        if self.gap_size() >= min_size {
            return;
        }
        // GNU insdel.c:483 (`make_gap_larger`): add GAP_BYTES_DFL beyond the
        // caller's requested need so a run of sequential inserts is amortized
        // O(1) rather than paying realloc on every ~64 bytes.
        let need = min_size - self.gap_size();
        let grow = need.saturating_add(GAP_BYTES_DFL);
        let old_gap_end = self.gap_end;
        let after_gap_len = self.buf.len() - old_gap_end;

        self.buf.resize(self.buf.len() + grow, 0);

        if after_gap_len > 0 {
            self.buf
                .copy_within(old_gap_end..old_gap_end + after_gap_len, old_gap_end + grow);
        }
        self.gap_end += grow;
    }

    // -----------------------------------------------------------------------
    // Position conversion
    // -----------------------------------------------------------------------

    /// Convert a logical Emacs byte position to a logical character position.
    ///
    /// Returns the number of complete characters before `byte_pos`.
    ///
    /// # Panics
    ///
    /// Panics if `byte_pos > self.len()` or is not on an Emacs character
    /// boundary.
    pub(crate) fn emacs_byte_pos_to_char_pos(&self, byte_pos: EmacsBytePos) -> CharPos0 {
        let byte_pos = byte_pos.get();
        assert!(
            byte_pos <= self.len(),
            "byte_to_char: byte_pos ({byte_pos}) > len ({})",
            self.len()
        );
        // GNU marker.c fast path (`if (Z == Z_byte) return bytepos`): when the
        // buffer has as many characters as bytes, every character is one byte,
        // so the char position equals the byte position.  Covers unibyte and
        // all-ASCII multibyte buffers in O(1) instead of scanning and decoding
        // from the buffer start.
        if self.total_chars == self.total_bytes {
            return CharPos0::new(byte_pos);
        }
        let (cache_byte, cache_char) = self.byte_char_cache.get();
        let cache = (cache_byte <= self.total_bytes && cache_char <= self.total_chars)
            .then_some((cache_byte, cache_char));
        let result = self.char_pos_from_byte_anchors(byte_pos, cache);
        // The cache must never change the answer: validate against the
        // cache-free computation in debug/test builds so any missed
        // invalidation fails loudly.
        debug_assert_eq!(
            result,
            self.char_pos_from_byte_anchors(byte_pos, None),
            "stale byte->char position cache at byte {byte_pos}"
        );
        self.byte_char_cache.set((byte_pos, result));
        CharPos0::new(result)
    }

    /// Char position for logical byte `target`, scanning from the nearest of
    /// the structural anchors `{0, gap, end}` plus an optional extra anchor
    /// (the cache).  Mirrors GNU `marker.c`'s nearest-anchor scan, so
    /// sequential conversions cost O(distance between calls), not O(buffer).
    fn char_pos_from_byte_anchors(&self, target: usize, extra: Option<(usize, usize)>) -> usize {
        let mut below = (0usize, 0usize);
        let mut above = (self.total_bytes, self.total_chars);
        for (b, c) in std::iter::once((self.gap_start_bytes, self.gap_start_chars)).chain(extra) {
            if b <= target && b > below.0 {
                below = (b, c);
            }
            if b >= target && b < above.0 {
                above = (b, c);
            }
        }
        if target - below.0 <= above.0 - target {
            below.1 + self.count_chars_in_logical_byte_range(below.0, target)
        } else {
            above.1 - self.count_chars_in_logical_byte_range(target, above.0)
        }
    }

    /// Count Emacs characters in the logical byte range `[lo, hi)`, mapping
    /// logical positions through the gap.  Both ends must be char boundaries
    /// (callers only pass known correspondences and the gap split, which are
    /// all char-aligned).
    fn count_chars_in_logical_byte_range(&self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi && hi <= self.total_bytes);
        let mut chars = 0;
        if lo < self.gap_start_bytes {
            let pre_hi = hi.min(self.gap_start_bytes);
            chars += emacs_char_count_bytes(&self.buf[lo..pre_hi], self.multibyte).get();
        }
        if hi > self.gap_start_bytes {
            let post_lo = lo.max(self.gap_start_bytes);
            let phys_lo = self.gap_end + (post_lo - self.gap_start_bytes);
            let phys_hi = self.gap_end + (hi - self.gap_start_bytes);
            chars += emacs_char_count_bytes(&self.buf[phys_lo..phys_hi], self.multibyte).get();
        }
        chars
    }

    /// Convert a char position to a logical Emacs byte position.
    ///
    /// `char_pos` is the number of characters from the start of the buffer.
    ///
    pub(crate) fn char_pos_to_emacs_byte_pos(&self, char_pos: CharPos0) -> EmacsBytePos {
        let char_pos = char_pos.get();
        if char_pos == 0 {
            return EmacsBytePos::new(0);
        }
        if char_pos > self.total_chars {
            // Clamp to end of buffer instead of panicking — this can happen
            // when window_start / point are stale after buffer modification.
            // Must precede the fast path below, which would otherwise return
            // the unclamped position for an all-single-byte buffer.
            tracing::debug!(
                "char_to_byte: char_pos ({char_pos}) exceeds char_count ({}), clamping",
                self.total_chars
            );
            return EmacsBytePos::new(self.total_bytes);
        }
        // GNU marker.c fast path: as many characters as bytes => every
        // character is one byte, so byte position equals char position.
        if self.total_chars == self.total_bytes {
            return EmacsBytePos::new(char_pos);
        }
        let (cache_byte, cache_char) = self.byte_char_cache.get();
        let cache = (cache_byte <= self.total_bytes && cache_char <= self.total_chars)
            .then_some((cache_byte, cache_char));
        let result = self.byte_pos_from_char_anchors(char_pos, cache);
        debug_assert_eq!(
            result,
            self.byte_pos_from_char_anchors(char_pos, None),
            "stale char->byte position cache at char {char_pos}"
        );
        self.byte_char_cache.set((result, char_pos));
        EmacsBytePos::new(result)
    }

    /// Byte position for char `target` (must be `<= total_chars`), scanning
    /// forward from the nearest known char anchor at or below it (structural
    /// anchors `{0, gap}` plus the optional cache).  Shares GNU `marker.c`'s
    /// cached correspondence with `char_pos_from_byte_anchors`.
    fn byte_pos_from_char_anchors(&self, target: usize, extra: Option<(usize, usize)>) -> usize {
        let mut below = (0usize, 0usize); // (byte, char)
        for (b, c) in std::iter::once((self.gap_start_bytes, self.gap_start_chars)).chain(extra) {
            if c <= target && c > below.1 {
                below = (b, c);
            }
        }
        below.0 + self.bytes_for_n_chars_from_logical_byte(below.0, target - below.1)
    }

    /// Logical byte span covering the next `nchars` characters starting at
    /// logical byte `start_byte` (a char boundary), mapped through the gap.
    fn bytes_for_n_chars_from_logical_byte(&self, start_byte: usize, nchars: usize) -> usize {
        if nchars == 0 {
            return 0;
        }
        let mut remaining = nchars;
        let mut consumed = 0;
        if start_byte < self.gap_start_bytes {
            let slice = &self.buf[start_byte..self.gap_start];
            let avail = emacs_char_count_bytes(slice, self.multibyte).get();
            if remaining <= avail {
                return emacs_char_to_byte_in_slice(slice, remaining, self.multibyte);
            }
            remaining -= avail;
            consumed = slice.len();
        }
        let post_phys = if start_byte >= self.gap_start_bytes {
            self.gap_end + (start_byte - self.gap_start_bytes)
        } else {
            self.gap_end
        };
        consumed + emacs_char_to_byte_in_slice(&self.buf[post_phys..], remaining, self.multibyte)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Check whether `pos` falls on a logical Emacs-character boundary in the
    /// text. O(1): single-byte bit test matching GNU's `CHAR_HEAD_P`
    /// (character.h). Multibyte trailing bytes have the form 10xxxxxx (0x80..=0xBF).
    /// Any other byte value is a character head.
    fn is_char_boundary(&self, pos: usize) -> bool {
        if !self.multibyte || pos == 0 || pos >= self.len() {
            return true;
        }
        // Multibyte trailing bytes have the form 10xxxxxx (0x80..=0xBF).
        // Any other byte value is a character head.
        (self.byte_at(pos) & 0xC0) != 0x80
    }

    // pdump accessors
    /// Extract the logical text content as a byte vector (for pdump).
    pub(crate) fn dump_text(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len());
        out.extend_from_slice(&self.buf[..self.gap_start]);
        out.extend_from_slice(&self.buf[self.gap_end..]);
        out
    }
    /// Reconstruct from text bytes (for pdump load).
    pub(crate) fn from_dump(text: Vec<u8>, multibyte: bool) -> Self {
        let len = text.len();
        let char_count = emacs_char_count_bytes(&text, multibyte).get();
        let byte_count = text.len();
        Self {
            buf: text,
            multibyte,
            gap_start: len,
            gap_end: len,
            gap_start_chars: char_count,
            total_chars: char_count,
            gap_start_bytes: byte_count,
            total_bytes: byte_count,
            byte_char_cache: Cell::new((0, 0)),
        }
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl Default for GapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GapBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text_emacs_byte_range(EmacsByteRange::new(
            EmacsBytePos::ZERO,
            EmacsBytePos::new(self.len()),
        )))
    }
}

impl fmt::Debug for GapBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GapBuffer")
            .field("len", &self.len())
            .field("char_count", &self.total_chars)
            .field("gap_start", &self.gap_start)
            .field("gap_start_chars", &self.gap_start_chars)
            .field("gap_start_bytes", &self.gap_start_bytes)
            .field("gap_end", &self.gap_end)
            .field("gap_size", &self.gap_size())
            .field("emacs_byte_len", &self.total_bytes)
            .field("text", &self.to_string())
            .finish()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[path = "gap_buffer_test.rs"]
mod tests;
