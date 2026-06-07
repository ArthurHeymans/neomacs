//! Structural buffer edit pipeline.
//!
//! This module is the first source-ownership extraction toward a GNU
//! `insdel.c`-style boundary. It rehomes the existing `Buffer` edit core
//! without changing behavior.

use super::{Buffer, BufferId, BufferManager, TextPropertyTable};
use crate::buffer::edit_transaction::{
    BufferEditState, DeleteSideEffectPolicy, InsertMarkerAdjustment, InsertMarkerPlacement,
    InsertSideEffectPolicy, MeasuredDeleteEdit, MeasuredInsertEdit, MeasuredReplaceEdit,
    MeasuredSameLenEdit, ReplaceSideEffectPolicy, SameLenSubstitutionPlan, SharedTextEditMetadata,
    SharedTextEditScope, TranspositionStoragePlan, char_pos_for_emacs_byte,
    convert_lisp_string_for_buffer_mode, emacs_byte_for_char_pos, lisp_string_from_buffer_bytes,
    modification_tick_delta,
};
use crate::buffer::undo;
use crate::buffer::{
    CharLen, CharPos0, CharRange, EmacsByteLen, EmacsBytePos, EmacsByteRange, TextEditRange,
    TextExtent, TextInsertion, TextPositionAnchor, TextReplacement, TextTransposition,
};
use crate::heap_types::LispString;

impl Buffer {
    fn edit_state(&self) -> BufferEditState {
        BufferEditState::new(
            self.point_anchor(),
            self.point_min_anchor(),
            self.point_max_anchor(),
        )
    }

    fn set_edit_state(&mut self, state: BufferEditState) {
        self.set_accessible_region_anchors_unchecked(state.begv(), state.zv());
        self.set_point_anchor_unchecked(state.point());
    }

    fn buffer_region_lisp_string(&self, range: EmacsByteRange) -> LispString {
        let mut bytes = Vec::new();
        self.text.copy_emacs_byte_range_to(range, &mut bytes);
        let mut string = lisp_string_from_buffer_bytes(bytes, self.get_multibyte());
        let props = self.text.text_props_slice_emacs_byte_range(range);
        if !props.is_empty() {
            *string.intervals_mut() = props;
        }
        string
    }

    fn insertion_at_point(&self, extent: TextExtent) -> TextInsertion {
        TextInsertion::at_anchor(self.point_anchor(), extent)
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
        // GNU `record_insert` always calls `record_point`, and that path
        // records the first-change sentinel when the buffer was unmodified.
        self.undo_prepare_change(edit.byte_pos(), self.point_emacs_byte_pos());
        let mut ul = self.get_undo_list();
        if !undo::undo_list_is_disabled(&ul) {
            undo::undo_list_record_insert(
                &mut ul,
                edit.char_pos(),
                edit.char_len(),
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
        let char_len = edit.char_len();

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
            char_pos_for_emacs_byte(&self.text, edit.byte_pos()),
            edit.char_pos(),
            "insert-side-effect char position drifted from the source edit site"
        );
        if policy.adjust_shared_text_props {
            self.text
                .adjust_text_props_for_insert_at(insertion.char_pos(), insertion.extent().chars());
        }
        self.overlays
            .adjust_for_inserted_text(insertion, edit.marker_placement().before_markers());
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
        let char_len = edit.char_len();

        self.set_edit_state(edit.state_after(self.edit_state(), policy));

        if policy.adjust_shared_markers {
            self.text.adjust_markers_for_delete_range(range);
        }

        if policy.adjust_shared_text_props {
            self.text
                .adjust_text_props_for_delete_range(range.char_range());
        }
        self.overlays.adjust_for_deleted_text(range);
        self.record_char_modification(char_len);
    }

    fn apply_same_len_edit_side_effects(
        &mut self,
        edit: MeasuredSameLenEdit,
        preserve_modified_state: bool,
    ) {
        if edit.is_empty() {
            return;
        }
        let old_state = self.modified_state_value();
        self.record_char_modification(edit.changed_chars());
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
        self.overlays.adjust_for_replaced_text(replacement);
        self.record_char_modification(edit.changed_chars());
    }

    fn record_char_modification(&mut self, changed_chars: CharLen) {
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
    fn undo_prepare_change(&mut self, beg: EmacsBytePos, pt: EmacsBytePos) {
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
        let extent = TextExtent::new(
            CharLen::new(text.schars()),
            EmacsByteLen::new(text.as_bytes().len()),
        );
        let insertion = self.insert_bytes_internal_full(
            text.as_bytes(),
            extent,
            marker_placement,
            marker_adjustment,
        );
        if text.has_intervals() {
            self.text.text_props_append_shifted_at_emacs_byte_pos(
                text.intervals(),
                insertion.byte_pos(),
            );
        }
        insertion
    }

    pub fn replace_emacs_byte_range_lisp_string(
        &mut self,
        byte_range: EmacsByteRange,
        text: &LispString,
    ) -> TextReplacement {
        if byte_range.start() > byte_range.end() {
            return TextReplacement::default();
        }
        let old_range = self.edit_range_for_emacs_byte_range(byte_range);
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

        if old_range.is_empty() {
            self.goto_emacs_byte_pos(old_range.byte_start());
            let insertion = self.insert_lisp_string(&text);
            debug_assert_eq!(old_range.byte_start(), insertion.byte_pos());
            debug_assert_eq!(old_range.char_start(), insertion.char_pos());
            return TextReplacement::new(old_range, insertion.extent());
        }

        let start_char = old_range.char_start();
        let end_char = old_range.char_end();
        let new_extent =
            TextExtent::new(CharLen::new(new_char_len), EmacsByteLen::new(new_byte_len));
        let replacement = TextReplacement::new(old_range, new_extent);

        let old_point = self.point_anchor();
        let deleted_text = self.buffer_region_lisp_string(old_range.byte_range());

        self.undo_prepare_change(old_range.byte_start(), old_point.emacs_byte_pos());
        let mut ul = self.get_undo_list();
        if !undo::undo_list_is_disabled(&ul) {
            // GNU `replace_range` records the insertion before the deletion
            // at FROM + old-length, so primitive-undo reinserts the old text
            // before deleting the replacement.  That order keeps markers and
            // overlay endpoints on opposite sides of the replacement distinct.
            undo::undo_list_record_insert(
                &mut ul,
                end_char,
                CharLen::new(new_char_len),
                self.undo_state.point_before_command_or_undo(),
            );
            undo::undo_list_record_delete(
                &mut ul,
                start_char,
                deleted_text,
                old_point.char_pos(),
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
            self.text.text_props_append_shifted_at_emacs_byte_pos(
                text.intervals(),
                replacement.byte_start(),
            );
        } else if !new_extent.chars().is_empty() {
            self.text.text_props_set_properties_in_emacs_byte_range(
                EmacsByteRange::from_start_len(replacement.byte_start(), new_extent.emacs_bytes()),
                Vec::new(),
            );
        }
        replacement
    }

    fn replace_measured_region_lisp_string_edit(
        &mut self,
        old_range: TextEditRange,
        text: &LispString,
    ) -> MeasuredReplaceEdit {
        MeasuredReplaceEdit::new(self.replace_measured_region_lisp_string(old_range, text))
    }

    /// Delete an Emacs-byte range.
    ///
    /// Adjusts point, mark, markers, and the narrowing boundary.
    pub fn delete_emacs_byte_range(&mut self, byte_range: EmacsByteRange) -> TextEditRange {
        if byte_range.is_empty() {
            return TextEditRange::default();
        }
        let range = self.edit_range_for_emacs_byte_range(byte_range);
        self.delete_measured_region(range)
    }

    pub fn delete_measured_region(&mut self, range: TextEditRange) -> TextEditRange {
        if range.is_empty() {
            return TextEditRange::default();
        }
        // Record undo: save the deleted text for restoration.
        let deleted_text = self.buffer_region_lisp_string(range.byte_range());
        // GNU `record_delete` always calls `record_point`, and that path
        // records the first-change sentinel when the buffer was unmodified.
        self.undo_prepare_change(range.byte_start(), self.point_emacs_byte_pos());
        let mut ul = self.get_undo_list();
        if !undo::undo_list_is_disabled(&ul) {
            for (marker, adjustment) in self.text.marker_adjustments_for_delete(range) {
                undo::undo_list_record_marker_adjustment(&mut ul, marker, adjustment);
            }
            undo::undo_list_record_delete(
                &mut ul,
                range.char_start(),
                deleted_text,
                self.point_char_pos(),
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

    fn delete_measured_region_edit(&mut self, range: TextEditRange) -> MeasuredDeleteEdit {
        MeasuredDeleteEdit::new(self.delete_measured_region(range))
    }

    /// Replace every occurrence of `from_code` with the Emacs-encoded
    /// bytes in `to_bytes` in the measured range.
    ///
    /// The replacement is performed in place, so callers must ensure the
    /// matched character's Emacs-byte length equals `to_bytes.len()`.
    pub fn subst_char_in_region(
        &mut self,
        range: TextEditRange,
        modified_range: TextEditRange,
        from_code: u32,
        to_bytes: &[u8],
        noundo: bool,
    ) -> bool {
        let edit = MeasuredSameLenEdit::new(range, modified_range);
        if edit.is_empty() {
            return false;
        }

        let mut region_bytes = Vec::with_capacity(range.byte_len().get());
        self.text
            .copy_emacs_byte_range_to(range.byte_range(), &mut region_bytes);
        let Some(plan) = SameLenSubstitutionPlan::new(
            range,
            &region_bytes,
            self.get_multibyte(),
            from_code,
            to_bytes,
        ) else {
            return false;
        };

        if !noundo {
            self.undo_prepare_change(modified_range.byte_start(), self.point_emacs_byte_pos());
            let mut ul = self.get_undo_list();
            if !undo::undo_list_is_disabled(&ul) {
                for changed_range in plan.changed_ranges().iter().copied() {
                    let mut deleted = lisp_string_from_buffer_bytes(
                        region_bytes[changed_range.byte_index_range_relative_to(range)].to_vec(),
                        self.get_multibyte(),
                    );
                    let props = self
                        .text
                        .text_props_slice_emacs_byte_range(changed_range.byte_range());
                    if !props.is_empty() {
                        *deleted.intervals_mut() = props;
                    }
                    undo::undo_list_record_delete(
                        &mut ul,
                        changed_range.char_start(),
                        deleted,
                        self.point_char_pos(),
                        self.undo_state.point_before_command_or_undo(),
                    );
                    undo::undo_list_record_insert(
                        &mut ul,
                        changed_range.char_start(),
                        changed_range.char_len(),
                        self.undo_state.point_before_command_or_undo(),
                    );
                }
                self.set_undo_list(ul);
            }
        }

        self.text.replace_same_len_measured_range(
            plan.replacement_for_range(range, self.get_multibyte()),
            plan.replacement_bytes(),
        );
        self.apply_same_len_edit_side_effects(edit, false);
        true
    }

    pub fn subst_char_changed_range(
        &self,
        range: TextEditRange,
        from_code: u32,
        to_bytes: &[u8],
    ) -> Option<TextEditRange> {
        if range.byte_range().is_empty() {
            return None;
        }

        let mut region_bytes = Vec::with_capacity(range.byte_len().get());
        self.text
            .copy_emacs_byte_range_to(range.byte_range(), &mut region_bytes);

        SameLenSubstitutionPlan::new(
            range,
            &region_bytes,
            self.get_multibyte(),
            from_code,
            to_bytes,
        )
        .map(|plan| plan.first_to_last_changed_range())
    }

    fn transpose_region_properties(&self, transposition: TextTransposition) -> TextPropertyTable {
        let first = transposition.first();
        let second = transposition.second();
        let props1 = self
            .text
            .text_props_snapshot()
            .slice_char_range(first.char_range());
        let props2 = self
            .text
            .text_props_snapshot()
            .slice_char_range(second.char_range());
        let props_mid = if transposition.same_char_len() {
            TextPropertyTable::new()
        } else {
            self.text
                .text_props_snapshot()
                .slice_char_range(transposition.middle_char_range())
        };

        let mut props = self.text.text_props_snapshot();
        if transposition.same_char_len() {
            props.remove_all_properties_in_char_range(first.char_range());
            props.remove_all_properties_in_char_range(second.char_range());
        } else {
            props.remove_all_properties_in_char_range(transposition.char_span());
            props.append_shifted_at_char_pos(
                &props_mid,
                transposition.middle_destination_char_start(),
            );
        }
        props.append_shifted_at_char_pos(&props1, transposition.first_destination_char_start());
        props.append_shifted_at_char_pos(&props2, transposition.second_destination_char_start());
        props
    }

    /// GNU `Ftranspose_regions` core: swap two non-overlapping current-buffer
    /// regions without changing buffer size.  Text movement is byte-based,
    /// while property and marker movement follows GNU's character positions.
    pub fn transpose_regions(&mut self, transposition: TextTransposition, leave_markers: bool) {
        let first = transposition.first();
        let second = transposition.second();
        let byte_span = transposition.byte_span();
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

        let plan = TranspositionStoragePlan::new(transposition, &region1, &mid, &region2);

        self.undo_prepare_change(first.byte_start(), self.point_emacs_byte_pos());
        let mut undo_list = self.get_undo_list();
        if !undo::undo_list_is_disabled(&undo_list) {
            let record_change = |undo_list: &mut crate::emacs_core::value::Value,
                                 start_char: CharPos0,
                                 deleted: LispString,
                                 pt: CharPos0,
                                 point_before: Option<CharPos0>| {
                let len_chars = CharLen::new(deleted.schars());
                undo::undo_list_record_delete(undo_list, start_char, deleted, pt, point_before);
                undo::undo_list_record_insert(undo_list, start_char, len_chars, point_before);
            };

            if transposition.same_char_len() {
                if transposition.adjacent() {
                    record_change(
                        &mut undo_list,
                        first.char_start(),
                        old_span,
                        self.point_char_pos(),
                        self.undo_state.point_before_command_or_undo(),
                    );
                } else {
                    record_change(
                        &mut undo_list,
                        first.char_start(),
                        self.buffer_region_lisp_string(first.byte_range()),
                        self.point_char_pos(),
                        self.undo_state.point_before_command_or_undo(),
                    );
                    record_change(
                        &mut undo_list,
                        second.char_start(),
                        self.buffer_region_lisp_string(second.byte_range()),
                        self.point_char_pos(),
                        self.undo_state.point_before_command_or_undo(),
                    );
                }
            } else {
                record_change(
                    &mut undo_list,
                    first.char_start(),
                    old_span,
                    self.point_char_pos(),
                    self.undo_state.point_before_command_or_undo(),
                );
            }
            self.set_undo_list(undo_list);
        }

        let replacement_props = self.transpose_region_properties(transposition);
        if transposition.same_char_len() {
            self.set_text_properties_with_undo_range(first.byte_range(), Vec::new());
            self.set_text_properties_with_undo_range(second.byte_range(), Vec::new());
        } else {
            self.set_text_properties_with_undo_range(transposition.byte_span(), Vec::new());
        }
        let new_point = transposition.transpose_anchor(self.point_anchor());

        self.text
            .replace_same_len_measured_range(plan.replacement(), plan.replacement_bytes());
        self.text.text_props_replace(replacement_props);
        if leave_markers {
            self.text.remap_marker_anchors(|old_position| {
                let old_byte = old_position.emacs_byte_pos();
                if old_byte > first.byte_start() && old_byte <= second.byte_end() {
                    TextPositionAnchor::new(
                        old_position.char_pos(),
                        emacs_byte_for_char_pos(&self.text, old_position.char_pos()),
                    )
                } else {
                    old_position
                }
            });
        } else {
            self.text
                .remap_marker_anchors(|old_position| transposition.transpose_anchor(old_position));
        }

        self.set_point_anchor_unchecked(new_point);
        self.apply_same_len_edit_side_effects(plan.edit(), false);
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
        Some(SharedTextEditScope::new(
            edited_id,
            self.buffers_sharing_root_ids(root_id),
        ))
    }

    fn shared_sibling_updates_state_fields(&self, sibling_id: BufferId) -> bool {
        self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id)
    }

    fn apply_shared_text_edit_metadata(
        buf: &mut Buffer,
        edit: SharedTextEditMetadata,
        update_state_fields: bool,
    ) {
        match edit {
            SharedTextEditMetadata::Insert(edit) => buf.apply_byte_insert_side_effects(
                edit,
                InsertSideEffectPolicy::shared_buffer(update_state_fields),
            ),
            SharedTextEditMetadata::Delete(edit) => buf.apply_byte_delete_side_effects(
                edit,
                DeleteSideEffectPolicy::shared_buffer(update_state_fields),
            ),
            SharedTextEditMetadata::Replace(edit) => buf.apply_replace_side_effects(
                edit,
                ReplaceSideEffectPolicy::shared_buffer(update_state_fields),
            ),
            SharedTextEditMetadata::SameLen {
                edit,
                preserve_modified_state,
            } => buf.apply_same_len_edit_side_effects(edit, preserve_modified_state),
            SharedTextEditMetadata::Transposition {
                edit,
                transposition,
                preserve_modified_state,
            } => {
                if update_state_fields {
                    let point = transposition.transpose_anchor(buf.point_anchor());
                    buf.set_point_anchor_unchecked(point);
                }
                buf.apply_same_len_edit_side_effects(edit, preserve_modified_state);
            }
        }
    }

    fn apply_shared_text_edit_to_siblings(
        &mut self,
        scope: SharedTextEditScope,
        edit: SharedTextEditMetadata,
    ) -> Option<()> {
        let can_update_state_fields = edit.can_update_buffer_state_fields();
        for sibling_id in scope.siblings() {
            let update_state_fields =
                can_update_state_fields && self.shared_sibling_updates_state_fields(sibling_id);
            {
                let sibling = self.buffers.get_mut(&sibling_id)?;
                Self::apply_shared_text_edit_metadata(sibling, edit, update_state_fields);
            }
            if can_update_state_fields {
                self.refresh_shared_buffer_state_cache(sibling_id, update_state_fields)?;
            }
        }
        Some(())
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

        self.apply_shared_text_edit_to_siblings(scope, SharedTextEditMetadata::Insert(edit))
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

        self.apply_shared_text_edit_to_siblings(scope, SharedTextEditMetadata::Insert(edit))
    }

    pub fn insert_into_buffer_before_markers(&mut self, id: BufferId, text: &str) -> Option<()> {
        if text.is_empty() {
            return Some(());
        }
        let scope = self.shared_text_edit_scope(id)?;
        let insertion = self.buffers.get_mut(&id)?.insert_before_markers(text);
        let edit =
            MeasuredInsertEdit::by_insertion_type(insertion, InsertMarkerPlacement::BeforeMarkers);

        self.apply_shared_text_edit_to_siblings(scope, SharedTextEditMetadata::Insert(edit))
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

        self.apply_shared_text_edit_to_siblings(scope, SharedTextEditMetadata::Insert(edit))
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
        let edit = self
            .buffers
            .get_mut(&id)?
            .delete_measured_region_edit(range);

        self.apply_shared_text_edit_to_siblings(scope, SharedTextEditMetadata::Delete(edit))
    }

    #[cfg(test)]
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
        self.replace_buffer_emacs_byte_range_lisp_string(
            id,
            EmacsByteRange::new(EmacsBytePos::new(start), EmacsBytePos::new(end)),
            text,
        )
    }

    pub fn replace_buffer_emacs_byte_range_lisp_string(
        &mut self,
        id: BufferId,
        byte_range: EmacsByteRange,
        text: &LispString,
    ) -> Option<()> {
        if byte_range.start() > byte_range.end() {
            return None;
        }
        let range = self.edit_range_for_buffer_emacs_byte_range(id, byte_range)?;
        self.replace_buffer_measured_region_lisp_string(id, range, text)
    }

    pub fn replace_buffer_measured_region_lisp_string(
        &mut self,
        id: BufferId,
        range: TextEditRange,
        text: &LispString,
    ) -> Option<()> {
        if range.is_empty() {
            self.goto_buffer_emacs_byte_pos(id, range.byte_start())?;
            return self.insert_lisp_string_into_buffer(id, text);
        }

        let scope = self.shared_text_edit_scope(id)?;
        let edit = self
            .buffers
            .get_mut(&id)?
            .replace_measured_region_lisp_string_edit(range, text);

        self.apply_shared_text_edit_to_siblings(scope, SharedTextEditMetadata::Replace(edit))
    }

    pub fn subst_char_in_buffer_region(
        &mut self,
        id: BufferId,
        range: TextEditRange,
        modified_range: TextEditRange,
        from_code: u32,
        to_bytes: &[u8],
        noundo: bool,
    ) -> Option<bool> {
        if range.byte_range().is_empty() {
            return Some(false);
        }

        let scope = self.shared_text_edit_scope(id)?;
        let edit = MeasuredSameLenEdit::new(range, modified_range);
        let changed = self.buffers.get_mut(&id)?.subst_char_in_region(
            range,
            modified_range,
            from_code,
            to_bytes,
            noundo,
        );
        if !changed {
            return Some(false);
        }

        self.apply_shared_text_edit_to_siblings(
            scope,
            SharedTextEditMetadata::SameLen {
                edit,
                preserve_modified_state: false,
            },
        )?;
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
        let edit = MeasuredSameLenEdit::covering(transposition.span_edit_range());

        self.apply_shared_text_edit_to_siblings(
            scope,
            SharedTextEditMetadata::Transposition {
                edit,
                transposition,
                preserve_modified_state: false,
            },
        )
    }
}
