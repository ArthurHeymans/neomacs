//! Structural buffer edit pipeline.
//!
//! This module is the first source-ownership extraction toward a GNU
//! `insdel.c`-style boundary. It rehomes the existing `Buffer` edit core
//! without changing behavior.

use super::{Buffer, BufferId, BufferManager, TextPropertyTable};
use crate::buffer::edit_transaction::{
    BufferEditState, DeleteSideEffectPolicy, InsertMarkerAdjustment, InsertMarkerPlacement,
    InsertSideEffectPolicy, MeasuredDeleteEdit, MeasuredInsertEdit, MeasuredReplaceEdit,
    ReplaceSideEffectPolicy, char_pos_for_emacs_byte, convert_lisp_string_for_buffer_mode,
    emacs_byte_for_char_pos, lisp_string_from_buffer_bytes, modification_tick_delta,
};
use crate::buffer::undo;
use crate::buffer::{
    CharPos0, CharRange, EmacsBytePos, EmacsByteRange, TextEditRange, TextExtent, TextInsertion,
    TextPositionAnchor, TextReplacement, TextTransposition,
};
use crate::heap_types::LispString;

#[derive(Clone, Debug)]
struct SharedTextEditScope {
    edited_id: BufferId,
    buffer_ids: Vec<BufferId>,
}

impl SharedTextEditScope {
    fn siblings(&self) -> impl Iterator<Item = BufferId> + '_ {
        self.buffer_ids
            .iter()
            .copied()
            .filter(|buffer_id| *buffer_id != self.edited_id)
    }
}

impl Buffer {
    fn edit_state(&self) -> BufferEditState {
        BufferEditState::from_usize(
            self.pt_byte,
            self.pt,
            self.begv_byte,
            self.begv,
            self.zv_byte,
            self.zv,
        )
    }

    fn set_edit_state(&mut self, state: BufferEditState) {
        self.pt_byte = state.point().emacs_byte_pos_usize();
        self.pt = state.point().char_pos_usize();
        self.begv_byte = state.begv().emacs_byte_pos_usize();
        self.begv = state.begv().char_pos_usize();
        self.zv_byte = state.zv().emacs_byte_pos_usize();
        self.zv = state.zv().char_pos_usize();
    }

    fn buffer_region_lisp_string(&self, range: EmacsByteRange) -> LispString {
        let mut bytes = Vec::new();
        self.text.copy_emacs_byte_range_to(range, &mut bytes);
        let mut string = lisp_string_from_buffer_bytes(bytes, self.get_multibyte());
        let props = self
            .text
            .text_props_slice(range.start_usize(), range.end_usize());
        if !props.is_empty() {
            *string.intervals_mut() = props;
        }
        string
    }

    fn insertion_at_point(&self, extent: TextExtent) -> TextInsertion {
        TextInsertion::new(
            EmacsBytePos::new(self.pt_byte),
            CharPos0::new(self.pt),
            extent,
        )
    }

    fn edit_range_at_emacs_byte_pos(&self, byte_pos: EmacsBytePos) -> TextEditRange {
        self.text.edit_range_at_emacs_byte_pos(byte_pos)
    }

    pub fn edit_range_for_emacs_byte_range(&self, byte_range: EmacsByteRange) -> TextEditRange {
        if byte_range.is_empty() {
            return self.edit_range_at_emacs_byte_pos(byte_range.start());
        }
        self.text.edit_range_for_emacs_byte_range(byte_range)
    }

    pub fn edit_range_for_char_range(&self, char_range: CharRange) -> TextEditRange {
        if char_range.is_empty() {
            let byte_pos = self.text.char_pos_to_emacs_byte_pos(char_range.start());
            return TextEditRange::empty_at(byte_pos, char_range.start());
        }
        self.text.edit_range_for_char_range(char_range)
    }

    pub fn text_transposition_for_char_ranges(
        &self,
        first: CharRange,
        second: CharRange,
    ) -> TextTransposition {
        TextTransposition::new(
            self.edit_range_for_char_range(first),
            self.edit_range_for_char_range(second),
        )
    }

    fn insert_bytes_internal(
        &mut self,
        bytes: &[u8],
        extent: TextExtent,
        marker_placement: InsertMarkerPlacement,
    ) -> TextInsertion {
        self.insert_bytes_internal_full(
            bytes,
            extent,
            marker_placement,
            InsertMarkerAdjustment::ByInsertionType,
        )
    }

    /// Same as `insert_bytes_internal`, but with explicit marker adjustment
    /// policy. The replacement path uses [`InsertMarkerAdjustment::StrictAfter`]
    /// so markers collapsed to the replacement start are not pushed past the
    /// inserted text, matching GNU `adjust_markers_for_replace`
    /// (insdel.c:341).
    fn insert_bytes_internal_full(
        &mut self,
        bytes: &[u8],
        extent: TextExtent,
        marker_placement: InsertMarkerPlacement,
        marker_adjustment: InsertMarkerAdjustment,
    ) -> TextInsertion {
        let insertion = self.insertion_at_point(extent);
        let edit = MeasuredInsertEdit::new(insertion, marker_placement, marker_adjustment);
        if edit.is_empty() {
            return edit.insertion();
        }
        let insert_pos = edit.byte_pos_usize();
        let insert_char_pos = edit.char_pos_usize();
        let char_len = edit.char_len_usize();

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
            .insert_measured_emacs_bytes(edit.byte_pos(), bytes, edit.extent());
        self.apply_byte_insert_side_effects(edit, InsertSideEffectPolicy::current_buffer());
        if edit.before_markers() {
            self.text
                .advance_markers_at_position(edit.byte_pos(), edit.extent());
        }
        edit.insertion()
    }

    fn apply_byte_insert_side_effects(
        &mut self,
        edit: MeasuredInsertEdit,
        policy: InsertSideEffectPolicy,
    ) {
        if edit.is_empty() {
            return;
        }
        let insertion = edit.insertion();
        let insert_pos = edit.byte_pos_usize();
        let insert_char_pos = edit.char_pos_usize();
        let byte_len = edit.byte_len_usize();
        let char_len = edit.char_len_usize();

        self.set_edit_state(edit.state_after(self.edit_state(), policy));
        if policy.adjust_shared_markers {
            if edit.marker_adjustment() == InsertMarkerAdjustment::StrictAfter {
                self.text
                    .adjust_markers_for_insert_extent_strict_after(edit.byte_pos(), edit.extent());
            } else {
                self.text
                    .adjust_markers_for_insert_extent(edit.byte_pos(), edit.extent());
            }
        }
        debug_assert_eq!(
            char_pos_for_emacs_byte(&self.text, EmacsBytePos::new(insert_pos)).get(),
            insert_char_pos,
            "insert-side-effect char position drifted from the source edit site"
        );
        if policy.adjust_shared_text_props {
            self.text
                .adjust_text_props_for_insert_at(insertion.char_pos(), insertion.extent().chars());
        }
        self.overlays.adjust_for_insert(
            insert_pos,
            byte_len,
            edit.marker_placement().before_markers(),
        );
        self.record_char_modification(char_len);
    }

    fn apply_byte_delete_side_effects(
        &mut self,
        edit: MeasuredDeleteEdit,
        policy: DeleteSideEffectPolicy,
    ) {
        let range = edit.range();
        if edit.is_empty() {
            return;
        }
        let start = edit.byte_start_usize();
        let end = edit.byte_end_usize();
        let char_len = edit.char_len_usize();

        self.set_edit_state(edit.state_after(self.edit_state(), policy));

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

    fn apply_replace_side_effects(
        &mut self,
        edit: MeasuredReplaceEdit,
        policy: ReplaceSideEffectPolicy,
    ) {
        if edit.is_empty() {
            return;
        }

        let replacement = edit.replacement();
        let old_range = replacement.old_range();
        let start = edit.old_byte_start_usize();
        let old_byte_len = edit.old_byte_len_usize();
        let new_byte_len = edit.new_byte_len_usize();

        self.set_edit_state(edit.state_after(self.edit_state(), policy));
        if policy.adjust_shared_markers {
            self.text
                .adjust_markers_for_replace_range(old_range, edit.new_extent());
        }
        if policy.adjust_shared_text_props {
            self.text.adjust_text_props_for_replace_at(
                edit.old_char_start(),
                edit.old_char_len(),
                edit.new_char_len(),
            );
        }
        self.overlays
            .adjust_for_replace(start, old_byte_len, new_byte_len);
        self.record_char_modification(edit.changed_chars_usize());
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
    /// Returns the measured insertion so callers can update sibling-buffer
    /// bookkeeping without re-measuring the storage-form input.
    fn insert_internal(
        &mut self,
        text: &str,
        marker_placement: InsertMarkerPlacement,
    ) -> TextInsertion {
        if text.is_empty() {
            return self.insertion_at_point(TextExtent::ZERO);
        }
        let bytes = crate::emacs_core::string_escape::storage_string_to_buffer_bytes(
            text,
            self.get_multibyte(),
        );
        let extent = TextExtent::from_emacs_bytes(&bytes, self.get_multibyte());
        self.insert_bytes_internal(&bytes, extent, marker_placement)
    }

    pub fn insert(&mut self, text: &str) -> TextInsertion {
        self.insert_internal(text, InsertMarkerPlacement::AfterMarkers)
    }

    pub fn insert_before_markers(&mut self, text: &str) -> TextInsertion {
        self.insert_internal(text, InsertMarkerPlacement::BeforeMarkers)
    }

    pub fn insert_lisp_string(&mut self, text: &LispString) -> TextInsertion {
        self.insert_lisp_string_full(
            text,
            InsertMarkerPlacement::AfterMarkers,
            InsertMarkerAdjustment::ByInsertionType,
        )
    }

    pub fn insert_lisp_string_before_markers(&mut self, text: &LispString) -> TextInsertion {
        self.insert_lisp_string_full(
            text,
            InsertMarkerPlacement::BeforeMarkers,
            InsertMarkerAdjustment::ByInsertionType,
        )
    }

    /// GNU-equivalent replace path: insert `text` at point but do NOT
    /// advance markers exactly at the insertion site even if their
    /// `insertion_type` is true. This matches GNU
    /// `adjust_markers_for_replace` (insdel.c:341), where markers at
    /// `from_byte` stay put regardless of insertion_type.
    pub fn insert_lisp_string_for_replace(&mut self, text: &LispString) -> TextInsertion {
        self.insert_lisp_string_full(
            text,
            InsertMarkerPlacement::AfterMarkers,
            InsertMarkerAdjustment::StrictAfter,
        )
    }

    fn insert_lisp_string_full(
        &mut self,
        text: &LispString,
        marker_placement: InsertMarkerPlacement,
        marker_adjustment: InsertMarkerAdjustment,
    ) -> TextInsertion {
        let text = convert_lisp_string_for_buffer_mode(text, self.get_multibyte());
        let insert_pos = self.pt_byte;
        let extent = TextExtent::from_usize(text.schars(), text.as_bytes().len());
        let insertion = self.insert_bytes_internal_full(
            text.as_bytes(),
            extent,
            marker_placement,
            marker_adjustment,
        );
        if text.has_intervals() {
            self.text
                .text_props_append_shifted(text.intervals(), insert_pos);
        }
        insertion
    }

    pub fn replace_region_lisp_string(
        &mut self,
        start: usize,
        end: usize,
        text: &LispString,
    ) -> TextReplacement {
        if start > end {
            return TextReplacement::default();
        }
        let old_range =
            self.edit_range_for_emacs_byte_range(EmacsByteRange::from_usize(start, end));
        self.replace_measured_region_lisp_string(old_range, text)
    }

    pub fn replace_measured_region_lisp_string(
        &mut self,
        old_range: TextEditRange,
        text: &LispString,
    ) -> TextReplacement {
        let text = convert_lisp_string_for_buffer_mode(text, self.get_multibyte());
        let new_bytes = text.as_bytes();
        let new_byte_len = new_bytes.len();
        let new_char_len = text.schars();
        let start = old_range.byte_start_usize();
        let end = old_range.byte_end_usize();

        if old_range.is_empty() {
            self.goto_byte(start);
            let insertion = self.insert_lisp_string(&text);
            debug_assert_eq!(old_range.byte_start(), insertion.byte_pos());
            debug_assert_eq!(old_range.char_start(), insertion.char_pos());
            return TextReplacement::new(old_range, insertion.extent());
        }

        let start_char = old_range.char_start();
        let end_char = old_range.char_end();
        let new_extent = TextExtent::from_usize(new_char_len, new_byte_len);
        let replacement = TextReplacement::new(old_range, new_extent);

        let old_pt_byte = self.pt_byte;
        let old_pt = self.pt;
        let deleted_text = self.buffer_region_lisp_string(old_range.byte_range());

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

        self.text.replace_measured_range(replacement, new_bytes);
        self.apply_replace_side_effects(
            MeasuredReplaceEdit::new(replacement),
            ReplaceSideEffectPolicy::current_buffer(),
        );
        if text.has_intervals() {
            self.text.text_props_append_shifted(text.intervals(), start);
        } else if new_char_len > 0 {
            self.text.text_props_set_properties_in_emacs_byte_range(
                EmacsByteRange::from_start_len(replacement.byte_start(), new_extent.emacs_bytes()),
                Vec::new(),
            );
        }
        replacement
    }

    /// Delete the byte range `[start, end)`.
    ///
    /// Adjusts point, mark, markers, and the narrowing boundary.
    pub fn delete_region(&mut self, start: usize, end: usize) -> TextEditRange {
        if start >= end {
            return TextEditRange::default();
        }
        let range = self.edit_range_for_emacs_byte_range(EmacsByteRange::from_usize(start, end));
        self.delete_measured_region(range)
    }

    pub fn delete_measured_region(&mut self, range: TextEditRange) -> TextEditRange {
        if range.is_empty() {
            return TextEditRange::default();
        }
        let start_char = range.char_start();
        let end_char = range.char_end();
        // Record undo: save the deleted text for restoration.
        let deleted_text = self.buffer_region_lisp_string(range.byte_range());
        // GNU `record_delete` always calls `record_point`, and that path
        // records the first-change sentinel when the buffer was unmodified.
        self.undo_prepare_change(range.byte_start_usize(), self.pt_byte);
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

        self.text.delete_measured_range(range);
        self.apply_byte_delete_side_effects(
            MeasuredDeleteEdit::new(range),
            DeleteSideEffectPolicy::current_buffer(),
        );
        range
    }

    /// Replace every occurrence of `from_code` with the Emacs-encoded
    /// bytes in `to_bytes` in the measured range.
    ///
    /// The replacement is performed in place, so callers must ensure the
    /// matched character's Emacs-byte length equals `to_bytes.len()`.
    pub fn subst_char_in_region(
        &mut self,
        range: TextEditRange,
        from_code: u32,
        to_bytes: &[u8],
        noundo: bool,
    ) -> bool {
        if range.byte_range().is_empty() {
            return false;
        }
        let changed_chars = range.char_len().get();
        let start = range.byte_start_usize();
        let end = range.byte_end_usize();

        // Copy the region's raw Emacs bytes and build a replacement by
        // walking chars and substituting the matched ones with to_bytes.
        use crate::emacs_core::emacs_char;
        let mut region_bytes = Vec::with_capacity(range.byte_len().get());
        self.text
            .copy_emacs_byte_range_to(range.byte_range(), &mut region_bytes);
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
                let mut deleted =
                    lisp_string_from_buffer_bytes(region_bytes.clone(), self.get_multibyte());
                let props = self.text.text_props_slice(start, end);
                if !props.is_empty() {
                    *deleted.intervals_mut() = props;
                }
                undo::undo_list_record_delete(
                    &mut ul,
                    range.char_start_usize(),
                    deleted,
                    self.pt,
                    self.undo_state.point_before_command_or_undo(),
                );
                undo::undo_list_record_insert(
                    &mut ul,
                    range.char_start_usize(),
                    changed_chars,
                    self.undo_state.point_before_command_or_undo(),
                );
                self.set_undo_list(ul);
            }
        }

        self.text.replace_same_len_measured_range(
            TextReplacement::new(
                range,
                TextExtent::from_emacs_bytes(&replacement_bytes, self.get_multibyte()),
            ),
            &replacement_bytes,
        );
        self.apply_same_len_edit_side_effects(changed_chars, false);
        true
    }

    pub fn subst_char_changed_range(
        &self,
        range: TextEditRange,
        from_code: u32,
    ) -> Option<TextEditRange> {
        if range.byte_range().is_empty() {
            return None;
        }

        let mut region_bytes = Vec::with_capacity(range.byte_len().get());
        self.text
            .copy_emacs_byte_range_to(range.byte_range(), &mut region_bytes);

        let range_start_byte = range.byte_start_usize();
        let range_start_char = range.char_start_usize();
        let mut first_changed: Option<TextPositionAnchor> = None;
        let mut last_changed = TextPositionAnchor::from_usize(range_start_char, range_start_byte);

        if self.get_multibyte() {
            use crate::emacs_core::emacs_char;

            let mut byte_offset = 0;
            let mut char_offset = 0;
            while byte_offset < region_bytes.len() {
                let (code, len) = emacs_char::string_char(&region_bytes[byte_offset..]);
                let clen = len.max(1);
                let char_pos = range_start_char + char_offset;
                let byte_pos = range_start_byte + byte_offset;
                if code == from_code {
                    first_changed
                        .get_or_insert_with(|| TextPositionAnchor::from_usize(char_pos, byte_pos));
                    last_changed = TextPositionAnchor::from_usize(char_pos + 1, byte_pos + clen);
                }
                byte_offset += clen;
                char_offset += 1;
            }
        } else {
            if from_code > 0xFF {
                return None;
            }
            let from_byte = from_code as u8;
            for (index, &byte) in region_bytes.iter().enumerate() {
                if byte == from_byte {
                    let char_pos = range_start_char + index;
                    let byte_pos = range_start_byte + index;
                    first_changed
                        .get_or_insert_with(|| TextPositionAnchor::from_usize(char_pos, byte_pos));
                    last_changed = TextPositionAnchor::from_usize(char_pos + 1, byte_pos + 1);
                }
            }
        }

        let first_changed = first_changed?;
        Some(TextEditRange::new(
            EmacsByteRange::new(
                first_changed.emacs_byte_pos(),
                last_changed.emacs_byte_pos(),
            ),
            first_changed.char_pos(),
            last_changed.char_pos(),
        ))
    }

    fn transpose_region_properties(&self, transposition: TextTransposition) -> TextPropertyTable {
        let first = transposition.first();
        let second = transposition.second();
        let start1_char = first.char_start_usize();
        let end1_char = first.char_end_usize();
        let start2_char = second.char_start_usize();
        let end2_char = second.char_end_usize();
        let len1 = transposition.first_char_len().get();
        let len2 = transposition.second_char_len().get();
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
    pub fn transpose_regions(&mut self, transposition: TextTransposition, leave_markers: bool) {
        let first = transposition.first();
        let second = transposition.second();
        let byte_span = transposition.byte_span();
        let start1_byte = first.byte_start_usize();
        let end1_byte = first.byte_end_usize();
        let start2_byte = second.byte_start_usize();
        let end2_byte = second.byte_end_usize();
        let start1_char = first.char_start_usize();
        let end1_char = first.char_end_usize();
        let start2_char = second.char_start_usize();
        let end2_char = second.char_end_usize();
        let mut region1 = Vec::with_capacity(first.byte_len().get());
        let mut mid = Vec::with_capacity(transposition.middle_byte_range().len().get());
        let mut region2 = Vec::with_capacity(second.byte_len().get());
        self.text
            .copy_emacs_byte_range_to(first.byte_range(), &mut region1);
        self.text
            .copy_emacs_byte_range_to(transposition.middle_byte_range(), &mut mid);
        self.text
            .copy_emacs_byte_range_to(second.byte_range(), &mut region2);

        let old_span = self.buffer_region_lisp_string(byte_span);

        let mut replacement = Vec::with_capacity(byte_span.len().get());
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

            if transposition.same_char_len() {
                if transposition.adjacent() {
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
                        self.buffer_region_lisp_string(first.byte_range()),
                        self.pt,
                        self.undo_state.point_before_command_or_undo(),
                    );
                    record_change(
                        &mut undo_list,
                        start2_char,
                        self.buffer_region_lisp_string(second.byte_range()),
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

        let replacement_props = self.transpose_region_properties(transposition);
        if transposition.same_char_len() {
            self.set_text_properties_with_undo(start1_byte, end1_byte, Vec::new());
            self.set_text_properties_with_undo(start2_byte, end2_byte, Vec::new());
        } else {
            self.set_text_properties_with_undo(start1_byte, end2_byte, Vec::new());
        }
        let new_point =
            transposition.transpose_anchor(TextPositionAnchor::from_usize(self.pt, self.pt_byte));

        self.text.replace_same_len_measured_range(
            TextReplacement::new(
                TextEditRange::new(
                    byte_span,
                    transposition.char_span().start(),
                    transposition.char_span().end(),
                ),
                TextExtent::from_emacs_bytes(&replacement, self.get_multibyte()),
            ),
            &replacement,
        );
        self.text.text_props_replace(replacement_props);
        if leave_markers {
            self.text
                .remap_markers_through_byte_char(|old_byte, old_char| {
                    if old_byte > start1_byte && old_byte <= end2_byte {
                        let refreshed = TextPositionAnchor::new(
                            CharPos0::new(old_char),
                            emacs_byte_for_char_pos(&self.text, CharPos0::new(old_char)),
                        );
                        (refreshed.emacs_byte_pos_usize(), refreshed.char_pos_usize())
                    } else {
                        (old_byte, old_char)
                    }
                });
        } else {
            self.text
                .remap_markers_through_byte_char(|old_byte, old_char| {
                    let moved = transposition
                        .transpose_anchor(TextPositionAnchor::from_usize(old_char, old_byte));
                    (moved.emacs_byte_pos_usize(), moved.char_pos_usize())
                });
        }

        self.pt_byte = new_point.emacs_byte_pos_usize();
        self.pt = new_point.char_pos_usize();
        self.apply_same_len_edit_side_effects(transposition.changed_chars().get(), false);
    }
}

/// Structural text mutation entry points for buffers and indirect-buffer
/// siblings. This is the closest Rust ownership boundary to GNU `insdel.c`.
impl BufferManager {
    pub fn edit_range_for_buffer_emacs_byte_range(
        &self,
        id: BufferId,
        byte_range: EmacsByteRange,
    ) -> Option<TextEditRange> {
        self.buffers
            .get(&id)
            .map(|buf| buf.edit_range_for_emacs_byte_range(byte_range))
    }

    pub fn edit_range_for_buffer_char_range(
        &self,
        id: BufferId,
        char_range: CharRange,
    ) -> Option<TextEditRange> {
        self.buffers
            .get(&id)
            .map(|buf| buf.edit_range_for_char_range(char_range))
    }

    fn shared_text_edit_scope(&self, edited_id: BufferId) -> Option<SharedTextEditScope> {
        let root_id = self.shared_text_root_id(edited_id)?;
        Some(SharedTextEditScope {
            edited_id,
            buffer_ids: self.buffers_sharing_root_ids(root_id),
        })
    }

    fn shared_sibling_updates_state_fields(&self, sibling_id: BufferId) -> bool {
        self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id)
    }

    fn apply_shared_text_edit_to_siblings<F>(
        &mut self,
        scope: SharedTextEditScope,
        mut apply: F,
    ) -> Option<()>
    where
        F: FnMut(&mut Buffer, bool),
    {
        for sibling_id in scope.siblings() {
            let update_state_fields = self.shared_sibling_updates_state_fields(sibling_id);
            {
                let sibling = self.buffers.get_mut(&sibling_id)?;
                apply(sibling, update_state_fields);
            }
            self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
        }
        Some(())
    }

    fn adjust_shared_insert_metadata(
        buf: &mut Buffer,
        edit: MeasuredInsertEdit,
        update_state_fields: bool,
    ) {
        buf.apply_byte_insert_side_effects(
            edit,
            InsertSideEffectPolicy::shared_buffer(update_state_fields),
        );
    }

    fn adjust_shared_delete_metadata(
        buf: &mut Buffer,
        edit: MeasuredDeleteEdit,
        update_state_fields: bool,
    ) {
        buf.apply_byte_delete_side_effects(
            edit,
            DeleteSideEffectPolicy::shared_buffer(update_state_fields),
        );
    }

    fn adjust_shared_replace_metadata(
        buf: &mut Buffer,
        edit: MeasuredReplaceEdit,
        update_state_fields: bool,
    ) {
        buf.apply_replace_side_effects(
            edit,
            ReplaceSideEffectPolicy::shared_buffer(update_state_fields),
        );
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
        let scope = self.shared_text_edit_scope(id)?;
        let insertion = self.buffers.get_mut(&id)?.insert(text);
        let edit =
            MeasuredInsertEdit::by_insertion_type(insertion, InsertMarkerPlacement::AfterMarkers);

        self.apply_shared_text_edit_to_siblings(scope, |sibling, update_state_fields| {
            Self::adjust_shared_insert_metadata(sibling, edit, update_state_fields);
        })
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
        let scope = self.shared_text_edit_scope(id)?;

        let insertion = if marker_adjustment == InsertMarkerAdjustment::StrictAfter {
            self.buffers
                .get_mut(&id)?
                .insert_lisp_string_for_replace(text)
        } else {
            self.buffers.get_mut(&id)?.insert_lisp_string(text)
        };
        let edit = MeasuredInsertEdit::new(
            insertion,
            InsertMarkerPlacement::AfterMarkers,
            marker_adjustment,
        );

        self.apply_shared_text_edit_to_siblings(scope, |sibling, update_state_fields| {
            Self::adjust_shared_insert_metadata(sibling, edit, update_state_fields);
        })
    }

    pub fn insert_into_buffer_before_markers(&mut self, id: BufferId, text: &str) -> Option<()> {
        if text.is_empty() {
            return Some(());
        }
        let scope = self.shared_text_edit_scope(id)?;
        let insertion = self.buffers.get_mut(&id)?.insert_before_markers(text);
        let edit =
            MeasuredInsertEdit::by_insertion_type(insertion, InsertMarkerPlacement::BeforeMarkers);

        self.apply_shared_text_edit_to_siblings(scope, |sibling, update_state_fields| {
            Self::adjust_shared_insert_metadata(sibling, edit, update_state_fields);
        })
    }

    pub fn insert_lisp_string_into_buffer_before_markers(
        &mut self,
        id: BufferId,
        text: &LispString,
    ) -> Option<()> {
        if text.is_empty() {
            return Some(());
        }
        let scope = self.shared_text_edit_scope(id)?;

        let insertion = self
            .buffers
            .get_mut(&id)?
            .insert_lisp_string_before_markers(text);
        let edit =
            MeasuredInsertEdit::by_insertion_type(insertion, InsertMarkerPlacement::BeforeMarkers);

        self.apply_shared_text_edit_to_siblings(scope, |sibling, update_state_fields| {
            Self::adjust_shared_insert_metadata(sibling, edit, update_state_fields);
        })
    }

    pub fn delete_buffer_region(&mut self, id: BufferId, start: usize, end: usize) -> Option<()> {
        if start >= end {
            return Some(());
        }
        self.delete_buffer_emacs_byte_range(id, EmacsByteRange::from_usize(start, end))
    }

    pub fn delete_buffer_emacs_byte_range(
        &mut self,
        id: BufferId,
        byte_range: EmacsByteRange,
    ) -> Option<()> {
        if byte_range.is_empty() {
            return Some(());
        }
        let range = self.edit_range_for_buffer_emacs_byte_range(id, byte_range)?;
        self.delete_buffer_measured_region(id, range)
    }

    pub fn delete_buffer_char_range(&mut self, id: BufferId, char_range: CharRange) -> Option<()> {
        if char_range.is_empty() {
            return Some(());
        }
        let range = self.edit_range_for_buffer_char_range(id, char_range)?;
        self.delete_buffer_measured_region(id, range)
    }

    pub fn delete_buffer_measured_region(
        &mut self,
        id: BufferId,
        range: TextEditRange,
    ) -> Option<()> {
        if range.is_empty() {
            return Some(());
        }

        let scope = self.shared_text_edit_scope(id)?;
        let range = self.buffers.get_mut(&id)?.delete_measured_region(range);
        let edit = MeasuredDeleteEdit::new(range);

        self.apply_shared_text_edit_to_siblings(scope, |sibling, update_state_fields| {
            Self::adjust_shared_delete_metadata(sibling, edit, update_state_fields);
        })
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
        let range = self
            .edit_range_for_buffer_emacs_byte_range(id, EmacsByteRange::from_usize(start, end))?;
        self.replace_buffer_measured_region_lisp_string(id, range, text)
    }

    pub fn replace_buffer_measured_region_lisp_string(
        &mut self,
        id: BufferId,
        range: TextEditRange,
        text: &LispString,
    ) -> Option<()> {
        if range.is_empty() {
            self.goto_buffer_byte(id, range.byte_start_usize())?;
            return self.insert_lisp_string_into_buffer(id, text);
        }

        let scope = self.shared_text_edit_scope(id)?;
        let replacement = self
            .buffers
            .get_mut(&id)?
            .replace_measured_region_lisp_string(range, text);
        let edit = MeasuredReplaceEdit::new(replacement);

        self.apply_shared_text_edit_to_siblings(scope, |sibling, update_state_fields| {
            Self::adjust_shared_replace_metadata(sibling, edit, update_state_fields);
        })
    }

    pub fn subst_char_in_buffer_region(
        &mut self,
        id: BufferId,
        range: TextEditRange,
        from_code: u32,
        to_bytes: &[u8],
        noundo: bool,
    ) -> Option<bool> {
        if range.byte_range().is_empty() {
            return Some(false);
        }

        let scope = self.shared_text_edit_scope(id)?;
        let changed_chars = range.char_len().get();
        let changed = self
            .buffers
            .get_mut(&id)?
            .subst_char_in_region(range, from_code, to_bytes, noundo);
        if !changed {
            return Some(false);
        }

        for sibling_id in scope.siblings() {
            let sibling = self.buffers.get_mut(&sibling_id)?;
            Self::adjust_shared_same_len_edit_metadata(sibling, changed_chars, false);
        }
        Some(true)
    }

    pub fn transpose_buffer_regions(
        &mut self,
        id: BufferId,
        transposition: TextTransposition,
        leave_markers: bool,
    ) -> Option<()> {
        let scope = self.shared_text_edit_scope(id)?;

        self.buffers
            .get_mut(&id)?
            .transpose_regions(transposition, leave_markers);

        self.apply_shared_text_edit_to_siblings(scope, |sibling, update_state_fields| {
            if update_state_fields {
                let point = transposition
                    .transpose_anchor(TextPositionAnchor::from_usize(sibling.pt, sibling.pt_byte));
                sibling.pt_byte = point.emacs_byte_pos_usize();
                sibling.pt = point.char_pos_usize();
            }
            Self::adjust_shared_same_len_edit_metadata(
                sibling,
                transposition.changed_chars().get(),
                false,
            );
        })
    }
}
