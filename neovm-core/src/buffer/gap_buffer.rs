//! A raw Emacs-byte gap buffer for efficient text editing.
//!
//! The gap buffer stores text in a contiguous `Vec<u8>` with a movable "gap"
//! (unused region) that makes insertions and deletions near the gap O(1)
//! amortized. The gap is relocated to the edit site before each mutation so
//! that sequential edits in the same neighborhood avoid large copies.
//!
//! All public positions are byte positions into the logical text (i.e. the
//! text with the gap removed) unless a parameter is explicitly named
//! `char_pos`. The underlying bytes are Emacs internal bytes, not sentinel-
//! encoded Rust strings.

use std::fmt;

use crate::buffer::text::{
    emacs_byte_to_char_in_slice, emacs_char_count_bytes, emacs_char_to_byte_in_slice,
};
use crate::buffer::{
    CharPos0, EmacsBytePos, EmacsByteRange, TextEditRange, TextExtent, TextReplacement,
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
        }
    }

    /// Create a gap buffer pre-loaded with raw Emacs bytes.
    pub fn from_emacs_bytes(text: &[u8], multibyte: bool) -> Self {
        let gap = GAP_BYTES_DFL;
        let char_count = emacs_char_count_bytes(text, multibyte);
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
        self.copy_bytes_to(0, self.len(), &mut logical);
        self.gap_start_chars = emacs_char_count_bytes(&logical[..self.gap_start], self.multibyte);
        self.total_chars = emacs_char_count_bytes(&logical, self.multibyte);
        self.gap_start_bytes = self.gap_start;
        self.total_bytes = logical.len();
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
    pub fn byte_at(&self, pos: usize) -> u8 {
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

    /// Return the logical Emacs byte at `pos`, or `None` if out of range.
    pub fn emacs_byte_at(&self, pos: usize) -> Option<u8> {
        (pos < self.total_bytes).then(|| self.byte_at(pos))
    }

    /// Return the `char` whose first byte starts at logical byte position `pos`,
    /// or `None` if `pos >= self.len()`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` does not lie on a UTF-8 character boundary.
    pub fn char_at(&self, pos: usize) -> Option<char> {
        self.char_code_at(pos).and_then(char::from_u32)
    }

    /// Return the Emacs character code whose first byte begins at logical
    /// byte position `pos`, or `None` if `pos >= self.len()`.
    pub fn char_code_at(&self, pos: usize) -> Option<u32> {
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

    // -----------------------------------------------------------------------
    // Range extraction
    // -----------------------------------------------------------------------

    /// Extract the text in the logical byte range `[start, end)` as a `String`.
    ///
    /// # Panics
    ///
    /// Panics if `start > end` or `end > self.len()`.
    pub fn text_range(&self, start: usize, end: usize) -> String {
        self.text_emacs_byte_range(EmacsByteRange::from_usize(start, end))
    }

    pub(crate) fn text_emacs_byte_range(&self, range: EmacsByteRange) -> String {
        let start = range.start_usize();
        let end = range.end_usize();
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
        self.copy_bytes_to(start, end, &mut out);
        crate::emacs_core::string_escape::emacs_bytes_to_storage_string(&out, self.multibyte)
    }

    /// Copy bytes in the logical range `[start, end)` into `out`.
    ///
    /// `out` is cleared first, then the bytes are appended.
    /// This is more efficient than `text_range()` when you don't need
    /// a `String` — it avoids the UTF-8 validation and String allocation.
    ///
    /// # Panics
    /// Panics if `start > end` or `end > self.len()`.
    pub fn copy_bytes_to(&self, start: usize, end: usize, out: &mut Vec<u8>) {
        self.copy_emacs_byte_range_to(EmacsByteRange::from_usize(start, end), out);
    }

    pub(crate) fn copy_emacs_byte_range_to(&self, range: EmacsByteRange, out: &mut Vec<u8>) {
        let start = range.start_usize();
        let end = range.end_usize();
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
        let start = range.start_usize();
        let end = range.end_usize();
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
        let start = range.start_usize();
        let end = range.end_usize();
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
        let start = range.start_usize();
        let end = range.end_usize();
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

    /// Insert raw Emacs bytes at logical byte position `pos`.
    ///
    /// Convenience wrapper that counts characters in `bytes`. If the caller
    /// already knows `nchars`, prefer `insert_emacs_bytes_both`.
    pub fn insert_emacs_bytes(&mut self, pos: usize, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.insert_measured_emacs_bytes(
            EmacsBytePos::new(pos),
            bytes,
            TextExtent::from_emacs_bytes(bytes, self.multibyte),
        );
    }

    /// Insert raw Emacs bytes at logical byte position `pos`, given the
    /// pre-computed character count.
    ///
    /// `nchars` **must** equal `chars_in_multibyte(bytes)` (or `bytes.len()` in
    /// unibyte mode). Passing a wrong value corrupts the char/byte counters.
    ///
    /// Mirrors GNU `insert_1_both` (`src/insdel.c:891`).
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
            nchars,
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
    }

    /// Insert `s` at logical byte position `pos`.
    ///
    /// After the call the gap sits immediately after the newly inserted text,
    /// so consecutive inserts at the same position are fast.
    ///
    /// # Panics
    ///
    /// Panics if `pos > self.len()` or `pos` is not on a UTF-8 boundary.
    pub fn insert_str(&mut self, pos: usize, s: &str) {
        if s.is_empty() {
            return;
        }
        let bytes =
            crate::emacs_core::string_escape::storage_string_to_buffer_bytes(s, self.multibyte);
        self.insert_emacs_bytes(pos, &bytes);
    }

    /// Delete the logical byte range `[start, end)`.
    ///
    /// Wrapper that counts deleted chars. Prefer `delete_range_both` if the
    /// caller already knows the count.
    pub fn delete_range(&mut self, start: usize, end: usize) {
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
        // delete_range_both lets callers skip.
        let mut tmp = Vec::with_capacity(end - start);
        self.copy_bytes_to(start, end, &mut tmp);
        let nchars = emacs_char_count_bytes(&tmp, self.multibyte);
        self.delete_range_both(start, end, nchars);
    }

    /// Delete the logical byte range `[start, end)`, given pre-computed char
    /// count of the region.
    ///
    /// Mirrors GNU `del_range_2` (`src/insdel.c:1991`).
    pub fn delete_range_both(&mut self, start: usize, end: usize, nchars: usize) {
        let start_char = self
            .emacs_byte_pos_to_char_pos(EmacsBytePos::new(start))
            .get();
        self.delete_measured_range(TextEditRange::from_usize(
            start,
            end,
            start_char,
            start_char + nchars,
        ));
    }

    pub(crate) fn delete_measured_range(&mut self, range: TextEditRange) {
        let start = range.byte_start_usize();
        let end = range.byte_end_usize();
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

    /// Overwrite the logical byte range `[start, end)` with raw Emacs bytes.
    pub fn replace_same_len_emacs_bytes(&mut self, start: usize, end: usize, replacement: &[u8]) {
        self.replace_same_len_emacs_byte_range(EmacsByteRange::from_usize(start, end), replacement);
    }

    pub(crate) fn replace_same_len_emacs_byte_range(
        &mut self,
        range: EmacsByteRange,
        replacement: &[u8],
    ) {
        let start = range.start_usize();
        let end = range.end_usize();
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
            replacement.len(),
            end - start,
            "replace_same_len_range: replacement Emacs-byte length ({}) must match replaced length ({})",
            replacement.len(),
            end - start
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

        self.move_gap_to(end);
        let old_chars = emacs_char_count_bytes(&self.buf[start..end], self.multibyte);
        let new_chars = emacs_char_count_bytes(replacement, self.multibyte);
        let old_bytes = end - start;
        let new_bytes = replacement.len();
        self.buf[start..end].copy_from_slice(replacement);
        if old_chars != new_chars {
            let delta = new_chars as isize - old_chars as isize;
            self.gap_start_chars = self.gap_start_chars.saturating_add_signed(delta);
            self.total_chars = self.total_chars.saturating_add_signed(delta);
        }
        if old_bytes != new_bytes {
            let delta = new_bytes as isize - old_bytes as isize;
            self.gap_start_bytes = self.gap_start_bytes.saturating_add_signed(delta);
            self.total_bytes = self.total_bytes.saturating_add_signed(delta);
        }
    }

    // -----------------------------------------------------------------------
    // Gap management
    // -----------------------------------------------------------------------

    /// Move the gap so that `gap_start == pos`.
    ///
    /// Wrapper that computes the char delta by scanning moved bytes. Prefer
    /// `move_gap_both` when the caller knows the target char position.
    pub fn move_gap_to(&mut self, pos: usize) {
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
            self.gap_start_chars - moved
        } else {
            let moved = emacs_char_count_bytes(
                &self.buf[self.gap_end..self.gap_end + (pos - self.gap_start)],
                self.multibyte,
            );
            self.gap_start_chars + moved
        };
        self.move_gap_both(pos, charpos);
    }

    /// Move the gap so that `gap_start == bytepos` and `gap_start_chars == charpos`.
    ///
    /// `charpos` **must** be the logical character position corresponding to
    /// `bytepos`. Passing a wrong value corrupts the char counters.
    ///
    /// Mirrors GNU `move_gap_both` (`src/insdel.c:88`).
    pub fn move_gap_both(&mut self, bytepos: usize, charpos: usize) {
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
        if byte_pos <= self.gap_start {
            return CharPos0::new(emacs_byte_to_char_in_slice(
                &self.buf[..self.gap_start],
                byte_pos,
                self.multibyte,
                "byte_to_char pre-gap",
            ));
        }

        let rel_pos = byte_pos - self.gap_start;
        CharPos0::new(
            self.gap_start_chars
                + emacs_byte_to_char_in_slice(
                    &self.buf[self.gap_end..],
                    rel_pos,
                    self.multibyte,
                    "byte_to_char post-gap",
                ),
        )
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

        if char_pos <= self.gap_start_chars {
            return EmacsBytePos::new(emacs_char_to_byte_in_slice(
                &self.buf[..self.gap_start],
                char_pos,
                self.multibyte,
            ));
        }

        if char_pos <= self.total_chars {
            return EmacsBytePos::new(
                self.gap_start
                    + emacs_char_to_byte_in_slice(
                        &self.buf[self.gap_end..],
                        char_pos - self.gap_start_chars,
                        self.multibyte,
                    ),
            );
        }

        // Clamp to end of buffer instead of panicking — this can happen
        // when window_start / point are stale after buffer modification.
        tracing::debug!(
            "char_to_byte: char_pos ({char_pos}) exceeds char_count ({}), clamping",
            self.total_chars
        );
        EmacsBytePos::new(self.len())
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
        let char_count = emacs_char_count_bytes(&text, multibyte);
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
        f.write_str(&self.text_range(0, self.len()))
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
