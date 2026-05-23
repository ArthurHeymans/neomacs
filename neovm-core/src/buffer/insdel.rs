//! Structural buffer edit pipeline.
//!
//! This module is the first source-ownership extraction toward a GNU
//! `insdel.c`-style boundary. It rehomes the existing `Buffer` edit core
//! without changing behavior.

use super::{Buffer, BufferId, BufferManager, TextPropertyTable};
use crate::buffer::undo;
use crate::heap_types::LispString;

#[inline]
fn emacs_char_count(bytes: &[u8], multibyte: bool) -> usize {
    if multibyte {
        crate::emacs_core::emacs_char::chars_in_multibyte(bytes)
    } else {
        bytes.len()
    }
}

#[inline]
fn lisp_string_from_buffer_bytes(bytes: Vec<u8>, multibyte: bool) -> LispString {
    if multibyte {
        LispString::from_emacs_bytes(bytes)
    } else {
        LispString::from_unibyte(bytes)
    }
}

#[inline]
fn encode_char_code_for_buffer_bytes(code: u32, multibyte: bool) -> Vec<u8> {
    if multibyte {
        let mut buf = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
        let len = crate::emacs_core::emacs_char::char_string(code, &mut buf);
        buf[..len].to_vec()
    } else {
        assert!(
            code <= 0xFF,
            "unibyte insertion produced non-byte character code {code:#X}"
        );
        vec![code as u8]
    }
}

fn convert_lisp_string_for_buffer_mode(text: &LispString, target_multibyte: bool) -> LispString {
    if text.is_multibyte() == target_multibyte {
        return text.clone();
    }

    if !target_multibyte {
        // GNU: insert_from_gap for unibyte buffers sets nchars=nbytes,
        // storing each byte of the multibyte internal representation as
        // a separate character.  Do NOT mask character codes with 0xFF
        // — that would truncate non-ASCII chars (e.g., decode-coding-region
        // of BIG5 data would lose the decoded characters).
        return lisp_string_from_buffer_bytes(text.as_bytes().to_vec(), false);
    }

    let mut codes = crate::emacs_core::builtins::lisp_string_char_codes(text);
    for code in &mut codes {
        if *code > 0x7F {
            *code = crate::emacs_core::emacs_char::unibyte_to_char(*code as u8);
        }
    }

    let mut bytes = Vec::new();
    for code in codes {
        bytes.extend_from_slice(&encode_char_code_for_buffer_bytes(code, target_multibyte));
    }
    lisp_string_from_buffer_bytes(bytes, target_multibyte)
}

#[inline]
fn transpose_position(pos: usize, start1: usize, end1: usize, start2: usize, end2: usize) -> usize {
    if pos < start1 || pos >= end2 {
        pos
    } else if pos < end1 {
        pos + (end2 - end1)
    } else if pos < start2 {
        let diff = (end2 - start2) as isize - (end1 - start1) as isize;
        (pos as isize + diff) as usize
    } else {
        pos - (start2 - start1)
    }
}

impl Buffer {
    fn insert_bytes_internal(&mut self, bytes: &[u8], char_len: usize, before_markers: bool) {
        self.insert_bytes_internal_full(bytes, char_len, before_markers, false);
    }

    /// Same as `insert_bytes_internal` but, when `strict_after_markers` is
    /// true, ignores `insertion_type` for markers exactly at the insertion
    /// site. Used by the GNU-equivalent replace path so that markers
    /// collapsed to the replacement start (by the prior delete) do not get
    /// pushed past the inserted text — see GNU `adjust_markers_for_replace`
    /// (insdel.c:341).
    fn insert_bytes_internal_full(
        &mut self,
        bytes: &[u8],
        char_len: usize,
        before_markers: bool,
        strict_after_markers: bool,
    ) {
        let insert_pos = self.pt_byte;
        let insert_char_pos = self.pt;
        if bytes.is_empty() {
            return;
        }
        let byte_len = bytes.len();

        // GNU `record_insert` always calls `record_point`, and that path
        // records the first-change sentinel when the buffer was unmodified.
        self.undo_prepare_change(insert_pos, self.pt_byte);
        let mut ul = self.get_undo_list();
        if !undo::undo_list_is_disabled(&ul) {
            undo::undo_list_record_insert(
                &mut ul,
                insert_char_pos,
                char_len,
                self.undo_state.point_before_command_or_undo(),
            );
            self.set_undo_list(ul);
        }

        self.text
            .insert_emacs_bytes_both(insert_pos, bytes, char_len);
        self.apply_byte_insert_side_effects(
            insert_pos,
            insert_char_pos,
            byte_len,
            char_len,
            true,
            false,
            true,
            true,
            true,
            before_markers,
            strict_after_markers,
        );
        if before_markers {
            self.text.advance_markers_at(insert_pos, byte_len, char_len);
        }
    }

    fn apply_byte_insert_side_effects(
        &mut self,
        insert_pos: usize,
        insert_char_pos: usize,
        byte_len: usize,
        char_len: usize,
        update_state_fields: bool,
        shift_begv: bool,
        advance_point_at_insert: bool,
        adjust_shared_markers: bool,
        adjust_shared_text_props: bool,
        overlay_before_markers: bool,
        strict_after_markers: bool,
    ) {
        if byte_len == 0 {
            return;
        }

        if update_state_fields {
            if self.pt_byte > insert_pos || (advance_point_at_insert && self.pt_byte == insert_pos)
            {
                self.pt_byte += byte_len;
                self.pt += char_len;
            }
            if shift_begv && self.begv_byte > insert_pos {
                self.begv_byte += byte_len;
                self.begv += char_len;
            }
            if self.zv_byte >= insert_pos {
                self.zv_byte += byte_len;
                self.zv += char_len;
            }
        }
        if adjust_shared_markers {
            if strict_after_markers {
                self.text
                    .adjust_markers_for_insert_strict_after(insert_pos, byte_len, char_len);
            } else {
                self.text
                    .adjust_markers_for_insert(insert_pos, byte_len, char_len);
            }
        }
        debug_assert_eq!(
            self.text.emacs_byte_to_char(insert_pos),
            insert_char_pos,
            "insert-side-effect char position drifted from the source edit site"
        );
        if adjust_shared_text_props {
            self.text
                .adjust_text_props_for_insert(insert_char_pos, char_len);
        }
        self.overlays
            .adjust_for_insert(insert_pos, byte_len, overlay_before_markers);
        self.record_char_modification(char_len);
    }

    fn apply_byte_delete_side_effects(
        &mut self,
        start: usize,
        end: usize,
        start_char: usize,
        end_char: usize,
        update_state_fields: bool,
        shift_begv: bool,
        adjust_shared_markers: bool,
        adjust_shared_text_props: bool,
    ) {
        if start >= end {
            return;
        }
        let byte_len = end - start;
        let char_len = end_char - start_char;

        if update_state_fields {
            if self.pt_byte >= end {
                self.pt_byte -= byte_len;
                self.pt -= char_len;
            } else if self.pt_byte > start {
                self.pt_byte = start;
                self.pt = start_char;
            }

            if shift_begv {
                if self.begv_byte >= end {
                    self.begv_byte -= byte_len;
                    self.begv -= char_len;
                } else if self.begv_byte > start {
                    self.begv_byte = start;
                    self.begv = start_char;
                }
            }

            if self.zv_byte >= end {
                self.zv_byte -= byte_len;
                self.zv -= char_len;
            } else if self.zv_byte > start {
                self.zv_byte = start;
                self.zv = start_char;
            }
        }

        if adjust_shared_markers {
            self.text
                .adjust_markers_for_delete(start, end, start_char, end_char);
        }

        if adjust_shared_text_props {
            self.text.adjust_text_props_for_delete(start_char, end_char);
        }
        self.overlays.adjust_for_delete(start, end);
        self.record_char_modification(char_len);
    }

    fn apply_same_len_edit_side_effects(
        &mut self,
        changed_chars: usize,
        preserve_modified_state: bool,
    ) {
        let old_state = self.modified_state_value();
        self.record_char_modification(changed_chars);
        if preserve_modified_state && old_state.is_nil() {
            self.text.set_save_modified_tick(self.text.modified_tick());
        }
    }

    fn modification_tick_delta(changed_chars: usize) -> i64 {
        if changed_chars == 0 {
            1
        } else {
            changed_chars.ilog2() as i64 + 1
        }
    }

    /// GNU `modiff` increments logarithmically with edit size, and
    /// `chars_modiff` is reset to the new `modiff` on each character change.
    fn record_char_modification(&mut self, changed_chars: usize) {
        self.text
            .record_char_modification(Self::modification_tick_delta(changed_chars));
    }

    /// Prepare to record a buffer change: ensure the first-change sentinel
    /// has been recorded if needed.
    fn undo_ensure_first_change(&mut self) {
        let mut ul = self.get_undo_list();
        if self.undo_state.recorded_first_change() && undo::undo_list_contains_first_change(&ul) {
            return;
        }
        if undo::undo_list_is_disabled(&ul) {
            return;
        }
        if self.modified_tick() > self.save_modified_tick() {
            return;
        }
        undo::undo_list_record_first_change(&mut ul);
        self.set_undo_list(ul);
        self.undo_state.set_recorded_first_change(true);
    }

    /// Prepare undo recording for a buffer edit at `beg` with point at `pt`.
    fn undo_prepare_change(&mut self, beg: usize, pt: usize) {
        let ul = self.get_undo_list();
        if undo::undo_list_is_disabled(&ul) {
            return;
        }
        let _ = (beg, pt);
        self.undo_ensure_first_change();
    }

    /// Insert `text` at point, advancing point past the inserted text.
    ///
    /// Markers at the insertion site move according to their `InsertionType`.
    /// Returns the `(byte_len, char_len)` of the inserted text so callers
    /// can update sibling-buffer bookkeeping without re-measuring the
    /// storage-form input.
    fn insert_internal(&mut self, text: &str, before_markers: bool) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }
        let bytes = crate::emacs_core::string_escape::storage_string_to_buffer_bytes(
            text,
            self.get_multibyte(),
        );
        let char_len = emacs_char_count(&bytes, self.get_multibyte());
        let byte_len = bytes.len();
        self.insert_bytes_internal(&bytes, char_len, before_markers);
        (byte_len, char_len)
    }

    pub fn insert(&mut self, text: &str) -> (usize, usize) {
        self.insert_internal(text, false)
    }

    pub fn insert_before_markers(&mut self, text: &str) -> (usize, usize) {
        self.insert_internal(text, true)
    }

    pub fn insert_lisp_string(&mut self, text: &LispString) {
        let text = convert_lisp_string_for_buffer_mode(text, self.get_multibyte());
        self.insert_bytes_internal(text.as_bytes(), text.schars(), false);
    }

    pub fn insert_lisp_string_before_markers(&mut self, text: &LispString) {
        let text = convert_lisp_string_for_buffer_mode(text, self.get_multibyte());
        self.insert_bytes_internal(text.as_bytes(), text.schars(), true);
    }

    /// GNU-equivalent replace path: insert `text` at point but do NOT
    /// advance markers exactly at the insertion site even if their
    /// `insertion_type` is true. This matches GNU
    /// `adjust_markers_for_replace` (insdel.c:341), where markers at
    /// `from_byte` stay put regardless of insertion_type.
    pub fn insert_lisp_string_for_replace(&mut self, text: &LispString) {
        let text = convert_lisp_string_for_buffer_mode(text, self.get_multibyte());
        self.insert_bytes_internal_full(text.as_bytes(), text.schars(), false, true);
    }

    pub fn replace_region_lisp_string(&mut self, start: usize, end: usize, text: &LispString) {
        if start > end {
            return;
        }
        let text = convert_lisp_string_for_buffer_mode(text, self.get_multibyte());
        let new_bytes = text.as_bytes();
        let new_byte_len = new_bytes.len();
        let new_char_len = text.schars();

        if start == end {
            self.goto_byte(start);
            self.insert_bytes_internal(new_bytes, new_char_len, false);
            return;
        }

        let start_char = self.text.emacs_byte_to_char(start);
        let end_char = self.text.emacs_byte_to_char(end);
        let old_byte_len = end - start;
        let old_char_len = end_char - start_char;

        let old_pt_byte = self.pt_byte;
        let old_pt = self.pt;
        let mut deleted_bytes = Vec::new();
        self.text
            .copy_emacs_bytes_to(start, end, &mut deleted_bytes);
        let deleted_text = lisp_string_from_buffer_bytes(deleted_bytes, self.get_multibyte());

        self.undo_prepare_change(start, old_pt_byte);
        let mut ul = self.get_undo_list();
        if !undo::undo_list_is_disabled(&ul) {
            undo::undo_list_record_insert(
                &mut ul,
                start_char + deleted_text.schars(),
                new_char_len,
                self.undo_state.point_before_command_or_undo(),
            );
            undo::undo_list_record_delete(
                &mut ul,
                start_char,
                deleted_text,
                old_pt,
                self.undo_state.point_before_command_or_undo(),
            );
            self.set_undo_list(ul);
        }

        self.text.delete_range_both(start, end, old_char_len);
        self.text
            .insert_emacs_bytes_both(start, new_bytes, new_char_len);

        if start < old_pt_byte || old_pt_byte == end {
            let clamped = old_pt_byte.min(end);
            self.pt_byte = old_pt_byte + start + new_byte_len - clamped;
            let clamped_char = old_pt.min(end_char);
            self.pt = old_pt + start_char + new_char_len - clamped_char;
        } else if old_pt_byte > end {
            self.pt_byte = old_pt_byte + new_byte_len - old_byte_len;
            self.pt = old_pt + new_char_len - old_char_len;
        }

        if self.begv_byte > end {
            self.begv_byte = self.begv_byte + new_byte_len - old_byte_len;
            self.begv = self.begv + new_char_len - old_char_len;
        } else if self.begv_byte > start {
            self.begv_byte = start;
            self.begv = start_char;
        }

        if self.zv_byte >= end {
            self.zv_byte = self.zv_byte + new_byte_len - old_byte_len;
            self.zv = self.zv + new_char_len - old_char_len;
        } else if self.zv_byte > start {
            self.zv_byte = start + new_byte_len;
            self.zv = start_char + new_char_len;
        }

        self.text.adjust_markers_for_replace(
            start,
            end,
            start_char,
            end_char,
            new_byte_len,
            new_char_len,
        );
        self.text.adjust_text_props_for_delete(start_char, end_char);
        self.text
            .adjust_text_props_for_insert(start_char, new_char_len);
        self.overlays
            .adjust_for_replace(start, old_byte_len, new_byte_len);
        self.record_char_modification(old_char_len.max(new_char_len));
    }

    /// Delete the byte range `[start, end)`.
    ///
    /// Adjusts point, mark, markers, and the narrowing boundary.
    pub fn delete_region(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let start_char = self.text.emacs_byte_to_char(start);
        let end_char = self.text.emacs_byte_to_char(end);
        // Record undo: save the deleted text for restoration.
        let mut deleted_bytes = Vec::new();
        self.text
            .copy_emacs_bytes_to(start, end, &mut deleted_bytes);
        let deleted_text = lisp_string_from_buffer_bytes(deleted_bytes, self.get_multibyte());
        // GNU `record_delete` always calls `record_point`, and that path
        // records the first-change sentinel when the buffer was unmodified.
        self.undo_prepare_change(start, self.pt_byte);
        let mut ul = self.get_undo_list();
        if !undo::undo_list_is_disabled(&ul) {
            undo::undo_list_record_delete(
                &mut ul,
                start_char,
                deleted_text,
                self.pt,
                self.undo_state.point_before_command_or_undo(),
            );
            self.set_undo_list(ul);
        }

        self.text
            .delete_range_both(start, end, end_char - start_char);
        self.apply_byte_delete_side_effects(
            start, end, start_char, end_char, true, false, true, true,
        );
    }

    /// Replace every occurrence of `from_code` with the Emacs-encoded
    /// bytes in `to_bytes` in the byte range `[start, end)`.
    ///
    /// The replacement is performed in place, so callers must ensure the
    /// matched character's Emacs-byte length equals `to_bytes.len()`.
    pub fn subst_char_in_region(
        &mut self,
        start: usize,
        end: usize,
        from_code: u32,
        to_bytes: &[u8],
        noundo: bool,
    ) -> bool {
        if start >= end {
            return false;
        }
        let changed_chars = self.text.emacs_byte_to_char(end) - self.text.emacs_byte_to_char(start);

        // Copy the region's raw Emacs bytes and build a replacement by
        // walking chars and substituting the matched ones with to_bytes.
        use crate::emacs_core::emacs_char;
        let mut region_bytes = Vec::with_capacity(end - start);
        self.text.copy_emacs_bytes_to(start, end, &mut region_bytes);
        let mut replacement_bytes = Vec::with_capacity(region_bytes.len());
        let mut changed = false;
        if self.get_multibyte() {
            let mut pos = 0;
            while pos < region_bytes.len() {
                let (code, len) = emacs_char::string_char(&region_bytes[pos..]);
                let clen = len.max(1);
                if code == from_code {
                    debug_assert_eq!(
                        clen,
                        to_bytes.len(),
                        "subst_char_in_region: matched char byte length ({}) must equal replacement length ({})",
                        clen,
                        to_bytes.len()
                    );
                    replacement_bytes.extend_from_slice(to_bytes);
                    changed = true;
                } else {
                    replacement_bytes.extend_from_slice(&region_bytes[pos..pos + clen]);
                }
                pos += clen;
            }
        } else {
            // Unibyte: each byte is one character. Replacement must be a
            // single byte whose value matches to_bytes[0].
            if from_code > 0xFF || to_bytes.len() != 1 {
                return false;
            }
            let from_byte = from_code as u8;
            for &b in &region_bytes {
                if b == from_byte {
                    replacement_bytes.push(to_bytes[0]);
                    changed = true;
                } else {
                    replacement_bytes.push(b);
                }
            }
        }
        if !changed {
            return false;
        }

        if !noundo {
            self.undo_prepare_change(start, self.pt_byte);
            let mut ul = self.get_undo_list();
            if !undo::undo_list_is_disabled(&ul) {
                let start_char = self.text.emacs_byte_to_char(start);
                let deleted =
                    lisp_string_from_buffer_bytes(region_bytes.clone(), self.get_multibyte());
                undo::undo_list_record_delete(
                    &mut ul,
                    start_char,
                    deleted,
                    self.pt,
                    self.undo_state.point_before_command_or_undo(),
                );
                undo::undo_list_record_insert(
                    &mut ul,
                    start_char,
                    changed_chars,
                    self.undo_state.point_before_command_or_undo(),
                );
                self.set_undo_list(ul);
            }
        }

        self.text
            .replace_same_len_emacs_bytes(start, end, &replacement_bytes);
        self.apply_same_len_edit_side_effects(changed_chars, false);
        true
    }

    fn transpose_region_properties(
        &self,
        start1_char: usize,
        end1_char: usize,
        start2_char: usize,
        end2_char: usize,
    ) -> TextPropertyTable {
        let len1 = end1_char - start1_char;
        let len2 = end2_char - start2_char;
        let props1 = self
            .text
            .text_props_snapshot()
            .slice(start1_char, end1_char);
        let props2 = self
            .text
            .text_props_snapshot()
            .slice(start2_char, end2_char);
        let props_mid = if len1 == len2 {
            TextPropertyTable::new()
        } else {
            self.text
                .text_props_snapshot()
                .slice(end1_char, start2_char)
        };

        let mut props = self.text.text_props_snapshot();
        if len1 == len2 {
            props.remove_all_properties(start1_char, end1_char);
            props.remove_all_properties(start2_char, end2_char);
        } else {
            props.remove_all_properties(start1_char, end2_char);
            props.append_shifted(&props_mid, start1_char + len2);
        }
        props.append_shifted(&props1, end2_char - len1);
        props.append_shifted(&props2, start1_char);
        props
    }

    /// GNU `Ftranspose_regions` core: swap two non-overlapping current-buffer
    /// regions without changing buffer size.  Text movement is byte-based,
    /// while property and marker movement follows GNU's character positions.
    #[allow(clippy::too_many_arguments)]
    pub fn transpose_regions(
        &mut self,
        start1_char: usize,
        end1_char: usize,
        start2_char: usize,
        end2_char: usize,
        start1_byte: usize,
        end1_byte: usize,
        start2_byte: usize,
        end2_byte: usize,
        leave_markers: bool,
    ) {
        let mut region1 = Vec::with_capacity(end1_byte - start1_byte);
        let mut mid = Vec::with_capacity(start2_byte - end1_byte);
        let mut region2 = Vec::with_capacity(end2_byte - start2_byte);
        self.text
            .copy_emacs_bytes_to(start1_byte, end1_byte, &mut region1);
        self.text
            .copy_emacs_bytes_to(end1_byte, start2_byte, &mut mid);
        self.text
            .copy_emacs_bytes_to(start2_byte, end2_byte, &mut region2);

        let mut old_span = Vec::with_capacity(end2_byte - start1_byte);
        old_span.extend_from_slice(&region1);
        old_span.extend_from_slice(&mid);
        old_span.extend_from_slice(&region2);

        let mut replacement = Vec::with_capacity(old_span.len());
        replacement.extend_from_slice(&region2);
        replacement.extend_from_slice(&mid);
        replacement.extend_from_slice(&region1);

        self.undo_prepare_change(start1_byte, self.pt_byte);
        let mut undo_list = self.get_undo_list();
        if !undo::undo_list_is_disabled(&undo_list) {
            let deleted = lisp_string_from_buffer_bytes(old_span, self.get_multibyte());
            undo::undo_list_record_delete(
                &mut undo_list,
                start1_char,
                deleted,
                self.pt,
                self.undo_state.point_before_command_or_undo(),
            );
            undo::undo_list_record_insert(
                &mut undo_list,
                start1_char,
                end2_char - start1_char,
                self.undo_state.point_before_command_or_undo(),
            );
            self.set_undo_list(undo_list);
        }

        let replacement_props =
            self.transpose_region_properties(start1_char, end1_char, start2_char, end2_char);
        let new_point_byte =
            transpose_position(self.pt_byte, start1_byte, end1_byte, start2_byte, end2_byte);
        let new_point_char =
            transpose_position(self.pt, start1_char, end1_char, start2_char, end2_char);

        self.text
            .replace_same_len_emacs_bytes(start1_byte, end2_byte, &replacement);
        self.text.text_props_replace(replacement_props);
        if leave_markers {
            self.text
                .remap_markers_through_byte_char(|old_byte, old_char| {
                    if old_byte > start1_byte && old_byte <= end2_byte {
                        (self.text.char_to_emacs_byte(old_char), old_char)
                    } else {
                        (old_byte, old_char)
                    }
                });
        } else {
            self.text
                .remap_markers_through_byte_char(|old_byte, old_char| {
                    (
                        transpose_position(
                            old_byte,
                            start1_byte,
                            end1_byte,
                            start2_byte,
                            end2_byte,
                        ),
                        transpose_position(
                            old_char,
                            start1_char,
                            end1_char,
                            start2_char,
                            end2_char,
                        ),
                    )
                });
        }

        self.pt_byte = new_point_byte;
        self.pt = new_point_char;
        self.apply_same_len_edit_side_effects(end2_char - start1_char, false);
    }
}

/// Structural text mutation entry points for buffers and indirect-buffer
/// siblings. This is the closest Rust ownership boundary to GNU `insdel.c`.
impl BufferManager {
    fn adjust_shared_insert_metadata(
        buf: &mut Buffer,
        insert_pos: usize,
        insert_char_pos: usize,
        byte_len: usize,
        char_len: usize,
        update_state_fields: bool,
        overlay_before_markers: bool,
    ) {
        Self::adjust_shared_insert_metadata_full(
            buf,
            insert_pos,
            insert_char_pos,
            byte_len,
            char_len,
            update_state_fields,
            overlay_before_markers,
            false,
        );
    }

    fn adjust_shared_insert_metadata_full(
        buf: &mut Buffer,
        insert_pos: usize,
        insert_char_pos: usize,
        byte_len: usize,
        char_len: usize,
        update_state_fields: bool,
        overlay_before_markers: bool,
        strict_after_markers: bool,
    ) {
        buf.apply_byte_insert_side_effects(
            insert_pos,
            insert_char_pos,
            byte_len,
            char_len,
            update_state_fields,
            true,
            false,
            false,
            false,
            overlay_before_markers,
            strict_after_markers,
        );
    }

    fn adjust_shared_delete_metadata(
        buf: &mut Buffer,
        start: usize,
        end: usize,
        start_char: usize,
        end_char: usize,
        update_state_fields: bool,
    ) {
        buf.apply_byte_delete_side_effects(
            start,
            end,
            start_char,
            end_char,
            update_state_fields,
            true,
            false,
            false,
        );
    }

    fn adjust_shared_replace_metadata(
        buf: &mut Buffer,
        start: usize,
        end: usize,
        start_char: usize,
        end_char: usize,
        new_byte_len: usize,
        new_char_len: usize,
        update_state_fields: bool,
    ) {
        let old_byte_len = end - start;
        let old_char_len = end_char - start_char;
        let old_pt_byte = buf.pt_byte;
        let old_pt = buf.pt;

        if update_state_fields {
            if start < old_pt_byte || old_pt_byte == end {
                let clamped = old_pt_byte.min(end);
                buf.pt_byte = old_pt_byte + start + new_byte_len - clamped;
                let clamped_char = old_pt.min(end_char);
                buf.pt = old_pt + start_char + new_char_len - clamped_char;
            } else if old_pt_byte > end {
                buf.pt_byte = old_pt_byte + new_byte_len - old_byte_len;
                buf.pt = old_pt + new_char_len - old_char_len;
            }

            if buf.begv_byte > end {
                buf.begv_byte = buf.begv_byte + new_byte_len - old_byte_len;
                buf.begv = buf.begv + new_char_len - old_char_len;
            } else if buf.begv_byte > start {
                buf.begv_byte = start;
                buf.begv = start_char;
            }

            if buf.zv_byte >= end {
                buf.zv_byte = buf.zv_byte + new_byte_len - old_byte_len;
                buf.zv = buf.zv + new_char_len - old_char_len;
            } else if buf.zv_byte > start {
                buf.zv_byte = start + new_byte_len;
                buf.zv = start_char + new_char_len;
            }
        }

        buf.text.adjust_markers_for_replace(
            start,
            end,
            start_char,
            end_char,
            new_byte_len,
            new_char_len,
        );
        buf.overlays
            .adjust_for_replace(start, old_byte_len, new_byte_len);
        buf.record_char_modification(old_char_len.max(new_char_len));
    }

    fn adjust_shared_same_len_edit_metadata(
        buf: &mut Buffer,
        changed_chars: usize,
        preserve_modified_state: bool,
    ) {
        buf.apply_same_len_edit_side_effects(changed_chars, preserve_modified_state);
    }

    fn refresh_shared_buffer_state_cache(
        &mut self,
        buffer_id: BufferId,
        update_state_fields: bool,
    ) -> Option<()> {
        if !update_state_fields && self.buffer_has_state_markers(buffer_id) {
            self.fetch_buffer_state_markers(buffer_id)?;
        }
        Some(())
    }

    pub fn insert_into_buffer(&mut self, id: BufferId, text: &str) -> Option<()> {
        if text.is_empty() {
            return Some(());
        }
        let root_id = self.shared_text_root_id(id)?;
        let shared_ids = self.buffers_sharing_root_ids(root_id);
        let source = self.buffers.get(&id)?;
        let insert_pos = source.pt_byte;
        let insert_char_pos = source.pt;

        let (byte_len, char_len) = self.buffers.get_mut(&id)?.insert(text);

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_insert_metadata(
                sibling,
                insert_pos,
                insert_char_pos,
                byte_len,
                char_len,
                update_state_fields,
                false,
            );
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }

    pub fn insert_lisp_string_into_buffer(
        &mut self,
        id: BufferId,
        text: &LispString,
    ) -> Option<()> {
        self.insert_lisp_string_into_buffer_full(id, text, false)
    }

    /// GNU-equivalent replace path: like `insert_lisp_string_into_buffer`
    /// but doesn't push markers exactly at point past the inserted text,
    /// even if their `insertion_type` is true. Used by
    /// `replace_buffer_region_lisp_string_in_manager` to match GNU
    /// `adjust_markers_for_replace` (insdel.c:341) semantics.
    pub fn insert_lisp_string_into_buffer_for_replace(
        &mut self,
        id: BufferId,
        text: &LispString,
    ) -> Option<()> {
        self.insert_lisp_string_into_buffer_full(id, text, true)
    }

    fn insert_lisp_string_into_buffer_full(
        &mut self,
        id: BufferId,
        text: &LispString,
        strict_after_markers: bool,
    ) -> Option<()> {
        if text.is_empty() {
            return Some(());
        }
        let byte_len = text.sbytes();
        let char_len = text.schars();

        let root_id = self.shared_text_root_id(id)?;
        let shared_ids = self.buffers_sharing_root_ids(root_id);
        let source = self.buffers.get(&id)?;
        let insert_pos = source.pt_byte;
        let insert_char_pos = source.pt;

        if strict_after_markers {
            self.buffers
                .get_mut(&id)?
                .insert_lisp_string_for_replace(text);
        } else {
            self.buffers.get_mut(&id)?.insert_lisp_string(text);
        }

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_insert_metadata_full(
                sibling,
                insert_pos,
                insert_char_pos,
                byte_len,
                char_len,
                update_state_fields,
                false,
                strict_after_markers,
            );
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }

    pub fn insert_into_buffer_before_markers(&mut self, id: BufferId, text: &str) -> Option<()> {
        if text.is_empty() {
            return Some(());
        }
        let root_id = self.shared_text_root_id(id)?;
        let shared_ids = self.buffers_sharing_root_ids(root_id);
        let source = self.buffers.get(&id)?;
        let insert_pos = source.pt_byte;
        let insert_char_pos = source.pt;

        let (byte_len, char_len) = self.buffers.get_mut(&id)?.insert_before_markers(text);

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_insert_metadata(
                sibling,
                insert_pos,
                insert_char_pos,
                byte_len,
                char_len,
                update_state_fields,
                true,
            );
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }

    pub fn insert_lisp_string_into_buffer_before_markers(
        &mut self,
        id: BufferId,
        text: &LispString,
    ) -> Option<()> {
        if text.is_empty() {
            return Some(());
        }
        let byte_len = text.sbytes();
        let char_len = text.schars();
        let root_id = self.shared_text_root_id(id)?;
        let shared_ids = self.buffers_sharing_root_ids(root_id);
        let source = self.buffers.get(&id)?;
        let insert_pos = source.pt_byte;
        let insert_char_pos = source.pt;

        self.buffers
            .get_mut(&id)?
            .insert_lisp_string_before_markers(text);

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_insert_metadata(
                sibling,
                insert_pos,
                insert_char_pos,
                byte_len,
                char_len,
                update_state_fields,
                true,
            );
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }

    pub fn delete_buffer_region(&mut self, id: BufferId, start: usize, end: usize) -> Option<()> {
        if start >= end {
            return Some(());
        }

        let root_id = self.shared_text_root_id(id)?;
        let shared_ids = self.buffers_sharing_root_ids(root_id);
        let source = self.buffers.get(&id)?;
        let start_char = source.text.emacs_byte_to_char(start);
        let end_char = source.text.emacs_byte_to_char(end);
        self.buffers.get_mut(&id)?.delete_region(start, end);

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_delete_metadata(
                sibling,
                start,
                end,
                start_char,
                end_char,
                update_state_fields,
            );
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }

    pub fn replace_buffer_region_lisp_string(
        &mut self,
        id: BufferId,
        start: usize,
        end: usize,
        text: &LispString,
    ) -> Option<()> {
        if start > end {
            return None;
        }

        if start == end {
            self.goto_buffer_byte(id, start)?;
            return self.insert_lisp_string_into_buffer(id, text);
        }

        let converted = {
            let source = self.buffers.get(&id)?;
            convert_lisp_string_for_buffer_mode(text, source.get_multibyte())
        };
        let new_byte_len = converted.sbytes();
        let new_char_len = converted.schars();
        let root_id = self.shared_text_root_id(id)?;
        let shared_ids = self.buffers_sharing_root_ids(root_id);
        let source = self.buffers.get(&id)?;
        let start_char = source.text.emacs_byte_to_char(start);
        let end_char = source.text.emacs_byte_to_char(end);

        self.buffers
            .get_mut(&id)?
            .replace_region_lisp_string(start, end, &converted);

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_replace_metadata(
                sibling,
                start,
                end,
                start_char,
                end_char,
                new_byte_len,
                new_char_len,
                update_state_fields,
            );
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }

    pub fn subst_char_in_buffer_region(
        &mut self,
        id: BufferId,
        start: usize,
        end: usize,
        from_code: u32,
        to_bytes: &[u8],
        noundo: bool,
    ) -> Option<bool> {
        if start >= end {
            return Some(false);
        }

        let root_id = self.shared_text_root_id(id)?;
        let shared_ids = self.buffers_sharing_root_ids(root_id);
        let changed_chars = {
            let source = self.buffers.get(&id)?;
            source.text.emacs_byte_to_char(end) - source.text.emacs_byte_to_char(start)
        };
        let changed = self
            .buffers
            .get_mut(&id)?
            .subst_char_in_region(start, end, from_code, to_bytes, noundo);
        if !changed {
            return Some(false);
        }

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_same_len_edit_metadata(sibling, changed_chars, false);
        }
        Some(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn transpose_buffer_regions(
        &mut self,
        id: BufferId,
        start1_char: usize,
        end1_char: usize,
        start2_char: usize,
        end2_char: usize,
        start1_byte: usize,
        end1_byte: usize,
        start2_byte: usize,
        end2_byte: usize,
        leave_markers: bool,
    ) -> Option<()> {
        let root_id = self.shared_text_root_id(id)?;
        let shared_ids = self.buffers_sharing_root_ids(root_id);

        self.buffers.get_mut(&id)?.transpose_regions(
            start1_char,
            end1_char,
            start2_char,
            end2_char,
            start1_byte,
            end1_byte,
            start2_byte,
            end2_byte,
            leave_markers,
        );

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            if update_state_fields {
                sibling.pt_byte = transpose_position(
                    sibling.pt_byte,
                    start1_byte,
                    end1_byte,
                    start2_byte,
                    end2_byte,
                );
                sibling.pt =
                    transpose_position(sibling.pt, start1_char, end1_char, start2_char, end2_char);
            }
            Self::adjust_shared_same_len_edit_metadata(sibling, end2_char - start1_char, false);
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }
}
