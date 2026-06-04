//! Structural buffer edit pipeline.
//!
//! This module is the first source-ownership extraction toward a GNU
//! `insdel.c`-style boundary. It rehomes the existing `Buffer` edit core
//! without changing behavior.

use super::{Buffer, BufferId, BufferManager, TextPropertyTable};
use crate::buffer::edit_transaction::{
    BufferEditState, DeleteSideEffectPolicy, InsertMarkerAdjustment, InsertSideEffectPolicy,
    char_pos_for_emacs_byte, convert_lisp_string_for_buffer_mode, emacs_byte_for_char_pos,
    emacs_char_count, lisp_string_from_buffer_bytes, modification_tick_delta,
    replace_state_after_edit, transpose_position,
};
use crate::buffer::undo;
use crate::buffer::{EmacsByteRange, TextEditRange, TextExtent, TextInsertion};
use crate::heap_types::LispString;

impl Buffer {
    fn edit_state(&self) -> BufferEditState {
        BufferEditState::new(
            self.pt_byte,
            self.pt,
            self.begv_byte,
            self.begv,
            self.zv_byte,
            self.zv,
        )
    }

    fn set_edit_state(&mut self, state: BufferEditState) {
        self.pt_byte = state.pt_byte;
        self.pt = state.pt;
        self.begv_byte = state.begv_byte;
        self.begv = state.begv;
        self.zv_byte = state.zv_byte;
        self.zv = state.zv;
    }

    fn buffer_region_lisp_string(&self, start: usize, end: usize) -> LispString {
        let mut bytes = Vec::new();
        self.text
            .copy_emacs_byte_range_to(EmacsByteRange::from_usize(start, end), &mut bytes);
        let mut string = lisp_string_from_buffer_bytes(bytes, self.get_multibyte());
        let props = self.text.text_props_slice(start, end);
        if !props.is_empty() {
            *string.intervals_mut() = props;
        }
        string
    }

    fn insert_bytes_internal(&mut self, bytes: &[u8], char_len: usize, before_markers: bool) {
        self.insert_bytes_internal_full(
            bytes,
            char_len,
            before_markers,
            InsertMarkerAdjustment::ByInsertionType,
        );
    }

    /// Same as `insert_bytes_internal`, but with explicit marker adjustment
    /// policy. The replacement path uses [`InsertMarkerAdjustment::StrictAfter`]
    /// so markers collapsed to the replacement start are not pushed past the
    /// inserted text, matching GNU `adjust_markers_for_replace`
    /// (insdel.c:341).
    fn insert_bytes_internal_full(
        &mut self,
        bytes: &[u8],
        char_len: usize,
        before_markers: bool,
        marker_adjustment: InsertMarkerAdjustment,
    ) {
        let insert_pos = self.pt_byte;
        let insert_char_pos = self.pt;
        if bytes.is_empty() {
            return;
        }
        let byte_len = bytes.len();
        let insertion = TextInsertion::from_usize(insert_pos, insert_char_pos, char_len, byte_len);

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
            .insert_measured_emacs_bytes(insertion.byte_pos(), bytes, insertion.extent());
        self.apply_byte_insert_side_effects(
            insertion,
            InsertSideEffectPolicy::current_buffer(before_markers, marker_adjustment),
        );
        if before_markers {
            self.text
                .advance_markers_at_position(insertion.byte_pos(), insertion.extent());
        }
    }

    fn apply_byte_insert_side_effects(
        &mut self,
        insertion: TextInsertion,
        policy: InsertSideEffectPolicy,
    ) {
        let insert_pos = insertion.byte_pos_usize();
        let insert_char_pos = insertion.char_pos_usize();
        let byte_len = insertion.extent().emacs_bytes().get();
        let char_len = insertion.extent().chars().get();
        if byte_len == 0 {
            return;
        }

        if policy.update_state_fields {
            if self.pt_byte > insert_pos
                || (policy.advance_point_at_insert && self.pt_byte == insert_pos)
            {
                self.pt_byte += byte_len;
                self.pt += char_len;
            }
            if policy.shift_begv && self.begv_byte > insert_pos {
                self.begv_byte += byte_len;
                self.begv += char_len;
            }
            if self.zv_byte >= insert_pos {
                self.zv_byte += byte_len;
                self.zv += char_len;
            }
        }
        if policy.adjust_shared_markers {
            if policy.marker_adjustment == InsertMarkerAdjustment::StrictAfter {
                self.text.adjust_markers_for_insert_extent_strict_after(
                    insertion.byte_pos(),
                    insertion.extent(),
                );
            } else {
                self.text
                    .adjust_markers_for_insert_extent(insertion.byte_pos(), insertion.extent());
            }
        }
        debug_assert_eq!(
            char_pos_for_emacs_byte(&self.text, insert_pos).get(),
            insert_char_pos,
            "insert-side-effect char position drifted from the source edit site"
        );
        if policy.adjust_shared_text_props {
            self.text
                .adjust_text_props_for_insert_at(insertion.char_pos(), insertion.extent().chars());
        }
        self.overlays
            .adjust_for_insert(insert_pos, byte_len, policy.overlay_before_markers);
        self.record_char_modification(char_len);
    }

    fn apply_byte_delete_side_effects(
        &mut self,
        range: TextEditRange,
        policy: DeleteSideEffectPolicy,
    ) {
        if range.is_empty() {
            return;
        }
        let start = range.byte_start_usize();
        let end = range.byte_end_usize();
        let start_char = range.char_start_usize();
        let byte_len = range.byte_len().get();
        let char_len = range.char_len().get();

        if policy.update_state_fields {
            if self.pt_byte >= end {
                self.pt_byte -= byte_len;
                self.pt -= char_len;
            } else if self.pt_byte > start {
                self.pt_byte = start;
                self.pt = start_char;
            }

            if policy.shift_begv {
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

        if policy.adjust_shared_markers {
            self.text.adjust_markers_for_delete_range(range);
        }

        if policy.adjust_shared_text_props {
            self.text
                .adjust_text_props_for_delete_range(range.char_range());
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

    fn record_char_modification(&mut self, changed_chars: usize) {
        self.text
            .record_char_modification(modification_tick_delta(changed_chars));
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
        let insert_pos = self.pt_byte;
        self.insert_bytes_internal(text.as_bytes(), text.schars(), false);
        if text.has_intervals() {
            self.text
                .text_props_append_shifted(text.intervals(), insert_pos);
        }
    }

    pub fn insert_lisp_string_before_markers(&mut self, text: &LispString) {
        let text = convert_lisp_string_for_buffer_mode(text, self.get_multibyte());
        let insert_pos = self.pt_byte;
        self.insert_bytes_internal(text.as_bytes(), text.schars(), true);
        if text.has_intervals() {
            self.text
                .text_props_append_shifted(text.intervals(), insert_pos);
        }
    }

    /// GNU-equivalent replace path: insert `text` at point but do NOT
    /// advance markers exactly at the insertion site even if their
    /// `insertion_type` is true. This matches GNU
    /// `adjust_markers_for_replace` (insdel.c:341), where markers at
    /// `from_byte` stay put regardless of insertion_type.
    pub fn insert_lisp_string_for_replace(&mut self, text: &LispString) {
        let text = convert_lisp_string_for_buffer_mode(text, self.get_multibyte());
        let insert_pos = self.pt_byte;
        self.insert_bytes_internal_full(
            text.as_bytes(),
            text.schars(),
            false,
            InsertMarkerAdjustment::StrictAfter,
        );
        if text.has_intervals() {
            self.text
                .text_props_append_shifted(text.intervals(), insert_pos);
        }
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
            self.insert_lisp_string(&text);
            return;
        }

        let start_char = char_pos_for_emacs_byte(&self.text, start);
        let end_char = char_pos_for_emacs_byte(&self.text, end);
        let old_byte_len = end - start;
        let old_char_len = end_char.get() - start_char.get();
        let old_range =
            TextEditRange::new(EmacsByteRange::from_usize(start, end), start_char, end_char);
        let new_extent = TextExtent::from_usize(new_char_len, new_byte_len);

        let old_state = self.edit_state();
        let old_pt_byte = self.pt_byte;
        let old_pt = self.pt;
        let deleted_text = self.buffer_region_lisp_string(start, end);

        self.undo_prepare_change(start, old_pt_byte);
        let mut ul = self.get_undo_list();
        if !undo::undo_list_is_disabled(&ul) {
            // GNU `replace_range` records the insertion before the deletion
            // at FROM + old-length, so primitive-undo reinserts the old text
            // before deleting the replacement.  That order keeps markers and
            // overlay endpoints on opposite sides of the replacement distinct.
            undo::undo_list_record_insert(
                &mut ul,
                end_char.get(),
                new_char_len,
                self.undo_state.point_before_command_or_undo(),
            );
            undo::undo_list_record_delete(
                &mut ul,
                start_char.get(),
                deleted_text,
                old_pt,
                self.undo_state.point_before_command_or_undo(),
            );
            self.set_undo_list(ul);
        }

        self.text.delete_measured_range(old_range);
        self.text
            .insert_measured_emacs_bytes(old_range.byte_start(), new_bytes, new_extent);
        self.set_edit_state(replace_state_after_edit(old_state, old_range, new_extent));

        self.text
            .adjust_markers_for_replace_range(old_range, new_extent);
        self.text.adjust_text_props_for_replace_at(
            old_range.char_start(),
            old_range.char_len(),
            new_extent.chars(),
        );
        if text.has_intervals() {
            self.text.text_props_append_shifted(text.intervals(), start);
        } else if new_char_len > 0 {
            self.text
                .text_props_set_properties(start, start + new_byte_len, Vec::new());
        }
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
        let start_char = char_pos_for_emacs_byte(&self.text, start);
        let end_char = char_pos_for_emacs_byte(&self.text, end);
        // Record undo: save the deleted text for restoration.
        let deleted_text = self.buffer_region_lisp_string(start, end);
        // GNU `record_delete` always calls `record_point`, and that path
        // records the first-change sentinel when the buffer was unmodified.
        self.undo_prepare_change(start, self.pt_byte);
        let mut ul = self.get_undo_list();
        if !undo::undo_list_is_disabled(&ul) {
            for (marker, adjustment) in self
                .text
                .marker_adjustments_for_delete(start_char.get(), end_char.get())
            {
                undo::undo_list_record_marker_adjustment(&mut ul, marker, adjustment);
            }
            undo::undo_list_record_delete(
                &mut ul,
                start_char.get(),
                deleted_text,
                self.pt,
                self.undo_state.point_before_command_or_undo(),
            );
            self.set_undo_list(ul);
        }

        let range =
            TextEditRange::new(EmacsByteRange::from_usize(start, end), start_char, end_char);
        self.text.delete_measured_range(range);
        self.apply_byte_delete_side_effects(range, DeleteSideEffectPolicy::current_buffer());
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
        let changed_chars = char_pos_for_emacs_byte(&self.text, end).get()
            - char_pos_for_emacs_byte(&self.text, start).get();

        // Copy the region's raw Emacs bytes and build a replacement by
        // walking chars and substituting the matched ones with to_bytes.
        use crate::emacs_core::emacs_char;
        let mut region_bytes = Vec::with_capacity(end - start);
        self.text
            .copy_emacs_byte_range_to(EmacsByteRange::from_usize(start, end), &mut region_bytes);
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
                let start_char = char_pos_for_emacs_byte(&self.text, start);
                let mut deleted =
                    lisp_string_from_buffer_bytes(region_bytes.clone(), self.get_multibyte());
                let props = self.text.text_props_slice(start, end);
                if !props.is_empty() {
                    *deleted.intervals_mut() = props;
                }
                undo::undo_list_record_delete(
                    &mut ul,
                    start_char.get(),
                    deleted,
                    self.pt,
                    self.undo_state.point_before_command_or_undo(),
                );
                undo::undo_list_record_insert(
                    &mut ul,
                    start_char.get(),
                    changed_chars,
                    self.undo_state.point_before_command_or_undo(),
                );
                self.set_undo_list(ul);
            }
        }

        self.text.replace_same_len_emacs_byte_range(
            EmacsByteRange::from_usize(start, end),
            &replacement_bytes,
        );
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
        self.text.copy_emacs_byte_range_to(
            EmacsByteRange::from_usize(start1_byte, end1_byte),
            &mut region1,
        );
        self.text
            .copy_emacs_byte_range_to(EmacsByteRange::from_usize(end1_byte, start2_byte), &mut mid);
        self.text.copy_emacs_byte_range_to(
            EmacsByteRange::from_usize(start2_byte, end2_byte),
            &mut region2,
        );

        let old_span = self.buffer_region_lisp_string(start1_byte, end2_byte);

        let mut replacement = Vec::with_capacity(end2_byte - start1_byte);
        replacement.extend_from_slice(&region2);
        replacement.extend_from_slice(&mid);
        replacement.extend_from_slice(&region1);

        self.undo_prepare_change(start1_byte, self.pt_byte);
        let mut undo_list = self.get_undo_list();
        if !undo::undo_list_is_disabled(&undo_list) {
            let record_change = |undo_list: &mut crate::emacs_core::value::Value,
                                 start_char: usize,
                                 deleted: LispString,
                                 pt: usize,
                                 point_before| {
                let len_chars = deleted.schars();
                undo::undo_list_record_delete(undo_list, start_char, deleted, pt, point_before);
                undo::undo_list_record_insert(undo_list, start_char, len_chars, point_before);
            };

            if end1_char - start1_char == end2_char - start2_char {
                if end1_char == start2_char {
                    record_change(
                        &mut undo_list,
                        start1_char,
                        old_span,
                        self.pt,
                        self.undo_state.point_before_command_or_undo(),
                    );
                } else {
                    record_change(
                        &mut undo_list,
                        start1_char,
                        self.buffer_region_lisp_string(start1_byte, end1_byte),
                        self.pt,
                        self.undo_state.point_before_command_or_undo(),
                    );
                    record_change(
                        &mut undo_list,
                        start2_char,
                        self.buffer_region_lisp_string(start2_byte, end2_byte),
                        self.pt,
                        self.undo_state.point_before_command_or_undo(),
                    );
                }
            } else {
                record_change(
                    &mut undo_list,
                    start1_char,
                    old_span,
                    self.pt,
                    self.undo_state.point_before_command_or_undo(),
                );
            }
            self.set_undo_list(undo_list);
        }

        let replacement_props =
            self.transpose_region_properties(start1_char, end1_char, start2_char, end2_char);
        if end1_char - start1_char == end2_char - start2_char {
            self.set_text_properties_with_undo(start1_byte, end1_byte, Vec::new());
            self.set_text_properties_with_undo(start2_byte, end2_byte, Vec::new());
        } else {
            self.set_text_properties_with_undo(start1_byte, end2_byte, Vec::new());
        }
        let new_point_byte =
            transpose_position(self.pt_byte, start1_byte, end1_byte, start2_byte, end2_byte);
        let new_point_char =
            transpose_position(self.pt, start1_char, end1_char, start2_char, end2_char);

        self.text.replace_same_len_emacs_byte_range(
            EmacsByteRange::from_usize(start1_byte, end2_byte),
            &replacement,
        );
        self.text.text_props_replace(replacement_props);
        if leave_markers {
            self.text
                .remap_markers_through_byte_char(|old_byte, old_char| {
                    if old_byte > start1_byte && old_byte <= end2_byte {
                        (
                            emacs_byte_for_char_pos(&self.text, old_char).get(),
                            old_char,
                        )
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
        insertion: TextInsertion,
        update_state_fields: bool,
        overlay_before_markers: bool,
    ) {
        Self::adjust_shared_insert_metadata_full(
            buf,
            insertion,
            update_state_fields,
            overlay_before_markers,
            InsertMarkerAdjustment::ByInsertionType,
        );
    }

    fn adjust_shared_insert_metadata_full(
        buf: &mut Buffer,
        insertion: TextInsertion,
        update_state_fields: bool,
        overlay_before_markers: bool,
        marker_adjustment: InsertMarkerAdjustment,
    ) {
        buf.apply_byte_insert_side_effects(
            insertion,
            InsertSideEffectPolicy::shared_buffer(
                update_state_fields,
                overlay_before_markers,
                marker_adjustment,
            ),
        );
    }

    fn adjust_shared_delete_metadata(
        buf: &mut Buffer,
        range: TextEditRange,
        update_state_fields: bool,
    ) {
        buf.apply_byte_delete_side_effects(
            range,
            DeleteSideEffectPolicy::shared_buffer(update_state_fields),
        );
    }

    fn adjust_shared_replace_metadata(
        buf: &mut Buffer,
        old_range: TextEditRange,
        new_extent: TextExtent,
        update_state_fields: bool,
    ) {
        let start = old_range.byte_start_usize();
        let old_byte_len = old_range.byte_len().get();
        let old_char_len = old_range.char_len().get();
        let new_byte_len = new_extent.emacs_bytes().get();
        let new_char_len = new_extent.chars().get();

        if update_state_fields {
            buf.set_edit_state(replace_state_after_edit(
                buf.edit_state(),
                old_range,
                new_extent,
            ));
        }

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
        let insertion = TextInsertion::from_usize(insert_pos, insert_char_pos, char_len, byte_len);

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_insert_metadata(sibling, insertion, update_state_fields, false);
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }

    pub fn insert_lisp_string_into_buffer(
        &mut self,
        id: BufferId,
        text: &LispString,
    ) -> Option<()> {
        self.insert_lisp_string_into_buffer_full(id, text, InsertMarkerAdjustment::ByInsertionType)
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
        self.insert_lisp_string_into_buffer_full(id, text, InsertMarkerAdjustment::StrictAfter)
    }

    fn insert_lisp_string_into_buffer_full(
        &mut self,
        id: BufferId,
        text: &LispString,
        marker_adjustment: InsertMarkerAdjustment,
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
        let insertion = TextInsertion::from_usize(insert_pos, insert_char_pos, char_len, byte_len);

        if marker_adjustment == InsertMarkerAdjustment::StrictAfter {
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
                insertion,
                update_state_fields,
                false,
                marker_adjustment,
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
        let insertion = TextInsertion::from_usize(insert_pos, insert_char_pos, char_len, byte_len);

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_insert_metadata(sibling, insertion, update_state_fields, true);
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
        let insertion = TextInsertion::from_usize(insert_pos, insert_char_pos, char_len, byte_len);

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
            Self::adjust_shared_insert_metadata(sibling, insertion, update_state_fields, true);
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
        let start_char = char_pos_for_emacs_byte(&source.text, start);
        let end_char = char_pos_for_emacs_byte(&source.text, end);
        let range =
            TextEditRange::new(EmacsByteRange::from_usize(start, end), start_char, end_char);
        self.buffers.get_mut(&id)?.delete_region(start, end);

        for sibling_id in shared_ids {
            if sibling_id == id {
                continue;
            }
            let update_state_fields =
                self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id);
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_delete_metadata(sibling, range, update_state_fields);
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
        let start_char = char_pos_for_emacs_byte(&source.text, start);
        let end_char = char_pos_for_emacs_byte(&source.text, end);
        let old_range =
            TextEditRange::new(EmacsByteRange::from_usize(start, end), start_char, end_char);
        let new_extent = TextExtent::from_usize(new_char_len, new_byte_len);

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
                old_range,
                new_extent,
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
            char_pos_for_emacs_byte(&source.text, end).get()
                - char_pos_for_emacs_byte(&source.text, start).get()
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
