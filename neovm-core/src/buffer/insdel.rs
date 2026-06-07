//! Structural buffer edit pipeline.
//!
//! This module is the first source-ownership extraction toward a GNU
//! `insdel.c`-style boundary. It rehomes the existing `Buffer` edit core
//! without changing behavior.

use super::{Buffer, BufferId, BufferManager, TextPropertyTable};
use crate::buffer::edit_transaction::{
    BufferEditState, DeleteSideEffectPolicy, DeleteTextPlan, InsertMarkerAdjustment,
    InsertMarkerPlacement, InsertSideEffectPolicy, InsertTextPlan, MeasuredDeleteEdit,
    MeasuredInsertEdit, MeasuredReplaceEdit, MeasuredSameLenEdit, ReplaceSideEffectPolicy,
    ReplaceTextPlan, SameLenModifiedStatePolicy, SameLenSubstitutionPlan, SharedBufferStateUpdate,
    SharedTextEditMetadata, SharedTextEditScope, SharedTextEditStatePolicy,
    TranspositionStoragePlan, char_pos_for_emacs_byte, emacs_byte_for_char_pos,
    lisp_string_from_buffer_bytes, modification_tick_delta,
};
use crate::buffer::undo;
use crate::buffer::{
    CharLen, CharPos0, CharRange, EmacsBytePos, EmacsByteRange, TextEditRange, TextExtent,
    TextInsertion, TextPositionAnchor, TextReplacement, TextTransposition,
};
use crate::heap_types::LispString;

impl Buffer {
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
            return TextInsertion::at_anchor(self.point_anchor(), TextExtent::ZERO);
        }
        let plan = InsertTextPlan::from_storage_text(
            text,
            self.get_multibyte(),
            self.point_anchor(),
            marker_placement,
            InsertMarkerAdjustment::ByInsertionType,
        );
        self.execute_insert_text_plan(plan)
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
        let plan = InsertTextPlan::from_lisp_string(
            text,
            self.get_multibyte(),
            self.point_anchor(),
            marker_placement,
            marker_adjustment,
        );
        self.execute_insert_text_plan(plan)
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
        let plan = ReplaceTextPlan::from_lisp_string(old_range, text, self.get_multibyte());
        self.execute_replace_text_plan(plan)
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
        let plan = self.delete_text_plan_for_range(range);
        self.execute_delete_text_plan(plan)
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
        self.apply_same_len_edit_side_effects(edit, SameLenModifiedStatePolicy::RecordChange);
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
        self.apply_same_len_edit_side_effects(
            plan.edit(),
            SameLenModifiedStatePolicy::RecordChange,
        );
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

    fn shared_sibling_state_update(&self, sibling_id: BufferId) -> SharedBufferStateUpdate {
        if self.current == Some(sibling_id) || !self.buffer_has_state_markers(sibling_id) {
            SharedBufferStateUpdate::UpdateFields
        } else {
            SharedBufferStateUpdate::RefreshFromStateMarkers
        }
    }

    fn apply_shared_text_edit_to_siblings(
        &mut self,
        scope: SharedTextEditScope,
        edit: SharedTextEditMetadata,
    ) -> Option<()> {
        for sibling_id in scope.siblings() {
            let state_policy = edit
                .state_policy_for_shared_sibling(|| self.shared_sibling_state_update(sibling_id));
            {
                let sibling = self.buffers.get_mut(&sibling_id)?;
                sibling.apply_shared_text_edit_side_effects(edit, state_policy);
            }
            if let Some(state_update) = state_policy.state_update() {
                self.refresh_shared_buffer_state_cache(sibling_id, state_update)?;
            }
        }
        Some(())
    }

    fn refresh_shared_buffer_state_cache(
        &mut self,
        buffer_id: BufferId,
        state_update: SharedBufferStateUpdate,
    ) -> Option<()> {
        if state_update.needs_state_marker_refresh() {
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
                modified_state: SameLenModifiedStatePolicy::RecordChange,
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
                modified_state: SameLenModifiedStatePolicy::RecordChange,
            },
        )
    }
}
