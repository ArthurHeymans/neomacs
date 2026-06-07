//! GNU-shaped buffer edit transaction policy.
//!
//! This module names the semantic side-effect policy used by the structural
//! edit pipeline.  The actual executor still lives in `insdel.rs`; keeping
//! these types separate is the first step toward a central insert/delete/
//! replace transaction boundary.

use crate::buffer::text::TextExtentDelta;
use crate::buffer::{
    BufferId, BufferText, CharLen, CharPos0, EmacsByteLen, EmacsBytePos, EmacsByteRange,
    TextEditRange, TextExtent, TextInsertion, TextPositionAnchor, TextReplacement,
    TextTransposition,
};
use crate::heap_types::LispString;

#[inline]
pub(in crate::buffer) fn lisp_string_from_buffer_bytes(
    bytes: Vec<u8>,
    multibyte: bool,
) -> LispString {
    if multibyte {
        LispString::from_emacs_bytes(bytes)
    } else {
        LispString::from_unibyte(bytes)
    }
}

#[inline]
pub(in crate::buffer) fn char_pos_for_emacs_byte(
    text: &BufferText,
    byte_pos: EmacsBytePos,
) -> CharPos0 {
    text.emacs_byte_pos_to_char_pos(byte_pos)
}

#[inline]
pub(in crate::buffer) fn emacs_byte_for_char_pos(
    text: &BufferText,
    char_pos: CharPos0,
) -> EmacsBytePos {
    text.char_pos_to_emacs_byte_pos(char_pos)
}

#[inline]
pub(in crate::buffer) fn encode_char_code_for_buffer_bytes(code: u32, multibyte: bool) -> Vec<u8> {
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

pub(in crate::buffer) fn convert_lisp_string_for_buffer_mode(
    text: &LispString,
    target_multibyte: bool,
) -> LispString {
    if text.is_multibyte() == target_multibyte {
        return text.clone();
    }

    let mut codes = crate::emacs_core::builtins::lisp_string_char_codes(text);
    if target_multibyte {
        for code in &mut codes {
            if *code > 0x7F {
                *code = crate::emacs_core::emacs_char::unibyte_to_char(*code as u8);
            }
        }
    } else {
        for code in &mut codes {
            *code &= 0xFF;
        }
    }

    let mut bytes = Vec::new();
    for code in codes {
        bytes.extend_from_slice(&encode_char_code_for_buffer_bytes(code, target_multibyte));
    }
    let mut converted = lisp_string_from_buffer_bytes(bytes, target_multibyte);
    if text.has_intervals() {
        let intervals = text.intervals().clone();
        if !intervals.is_empty() {
            *converted.intervals_mut() = intervals;
        }
    }
    converted
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct BufferEditState {
    point: TextPositionAnchor,
    begv: TextPositionAnchor,
    zv: TextPositionAnchor,
}

impl BufferEditState {
    pub(in crate::buffer) const fn new(
        point: TextPositionAnchor,
        begv: TextPositionAnchor,
        zv: TextPositionAnchor,
    ) -> Self {
        Self { point, begv, zv }
    }

    pub(in crate::buffer) const fn point(self) -> TextPositionAnchor {
        self.point
    }

    pub(in crate::buffer) const fn begv(self) -> TextPositionAnchor {
        self.begv
    }

    pub(in crate::buffer) const fn zv(self) -> TextPositionAnchor {
        self.zv
    }

    fn set_point(&mut self, point: TextPositionAnchor) {
        self.point = point;
    }

    fn set_begv(&mut self, begv: TextPositionAnchor) {
        self.begv = begv;
    }

    fn set_zv(&mut self, zv: TextPositionAnchor) {
        self.zv = zv;
    }
}

fn anchor_byte_gt(anchor: TextPositionAnchor, other: TextPositionAnchor) -> bool {
    anchor.emacs_byte_pos() > other.emacs_byte_pos()
}

fn anchor_byte_ge(anchor: TextPositionAnchor, other: TextPositionAnchor) -> bool {
    anchor.emacs_byte_pos() >= other.emacs_byte_pos()
}

fn anchor_byte_eq(anchor: TextPositionAnchor, other: TextPositionAnchor) -> bool {
    anchor.emacs_byte_pos() == other.emacs_byte_pos()
}

fn move_after_insert(
    position: TextPositionAnchor,
    insertion: TextInsertion,
    move_at_insertion: bool,
) -> TextPositionAnchor {
    let start = insertion.start_anchor();
    if anchor_byte_gt(position, start) || (move_at_insertion && anchor_byte_eq(position, start)) {
        TextExtentDelta::insertion(insertion.extent()).apply_to_anchor(position)
    } else {
        position
    }
}

fn move_after_delete(
    position: TextPositionAnchor,
    range: TextEditRange,
    move_at_end: bool,
) -> TextPositionAnchor {
    let start = range.start_anchor();
    let end = range.end_anchor();
    if anchor_byte_gt(position, end) || (move_at_end && anchor_byte_eq(position, end)) {
        TextExtentDelta::deletion(range.extent()).apply_to_anchor(position)
    } else if anchor_byte_gt(position, start) {
        start
    } else {
        position
    }
}

fn move_after_replace_for_point(
    position: TextPositionAnchor,
    replacement: TextReplacement,
) -> TextPositionAnchor {
    let old_range = replacement.old_range();
    let start = replacement.old_start_anchor();
    let end = replacement.old_end_anchor();

    if anchor_byte_gt(position, start) && position.emacs_byte_pos() < end.emacs_byte_pos()
        || anchor_byte_eq(position, end)
    {
        TextExtentDelta::insertion(replacement.new_extent()).apply_to_anchor(start)
    } else if anchor_byte_gt(position, end) {
        TextExtentDelta::replacement(old_range.extent(), replacement.new_extent())
            .apply_to_anchor(position)
    } else {
        position
    }
}

fn move_after_replace_for_begv(
    position: TextPositionAnchor,
    replacement: TextReplacement,
) -> TextPositionAnchor {
    let old_range = replacement.old_range();
    let start = replacement.old_start_anchor();
    let end = replacement.old_end_anchor();

    if anchor_byte_gt(position, end) {
        TextExtentDelta::replacement(old_range.extent(), replacement.new_extent())
            .apply_to_anchor(position)
    } else if anchor_byte_gt(position, start) {
        start
    } else {
        position
    }
}

fn move_after_replace_for_zv(
    position: TextPositionAnchor,
    replacement: TextReplacement,
) -> TextPositionAnchor {
    let old_range = replacement.old_range();
    let start = replacement.old_start_anchor();
    let end = replacement.old_end_anchor();

    if anchor_byte_ge(position, end) {
        TextExtentDelta::replacement(old_range.extent(), replacement.new_extent())
            .apply_to_anchor(position)
    } else if anchor_byte_gt(position, start) {
        TextExtentDelta::insertion(replacement.new_extent()).apply_to_anchor(start)
    } else {
        position
    }
}

pub(in crate::buffer) fn replace_state_after_edit(
    mut state: BufferEditState,
    replacement: TextReplacement,
) -> BufferEditState {
    state.set_point(move_after_replace_for_point(state.point(), replacement));
    state.set_begv(move_after_replace_for_begv(state.begv(), replacement));
    state.set_zv(move_after_replace_for_zv(state.zv(), replacement));

    state
}

pub(in crate::buffer) fn insert_state_after_edit(
    mut state: BufferEditState,
    insertion: TextInsertion,
    policy: InsertSideEffectPolicy,
) -> BufferEditState {
    if !policy.state_fields.update_state_fields() {
        return state;
    }

    state.set_point(move_after_insert(
        state.point(),
        insertion,
        policy.point_at_insertion.advance_point_at_insertion(),
    ));
    if policy.accessible_start.shift_after_edit() {
        state.set_begv(move_after_insert(state.begv(), insertion, false));
    }
    state.set_zv(move_after_insert(state.zv(), insertion, true));

    state
}

pub(in crate::buffer) fn delete_state_after_edit(
    mut state: BufferEditState,
    range: TextEditRange,
    policy: DeleteSideEffectPolicy,
) -> BufferEditState {
    if !policy.state_fields.update_state_fields() {
        return state;
    }

    state.set_point(move_after_delete(state.point(), range, true));

    if policy.accessible_start.shift_after_edit() {
        state.set_begv(move_after_delete(state.begv(), range, true));
    }
    state.set_zv(move_after_delete(state.zv(), range, true));

    state
}

/// GNU `modiff` increments logarithmically with edit size, and
/// `chars_modiff` is reset to the new `modiff` on each character change.
pub(in crate::buffer) fn modification_tick_delta(changed_chars: CharLen) -> i64 {
    let changed_chars = changed_chars.get();
    if changed_chars == 0 {
        1
    } else {
        changed_chars.ilog2() as i64 + 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum InsertMarkerAdjustment {
    ByInsertionType,
    StrictAfter,
}

/// GNU insert marker placement mode.
///
/// GNU passes this as a `before_markers` boolean to `insert_1_both` and
/// `adjust_markers_for_insert`.  Keeping it as an enum at the Rust edit
/// boundary prevents callers from mixing up the marker-placement decision with
/// unrelated boolean side-effect toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum InsertMarkerPlacement {
    AfterMarkers,
    BeforeMarkers,
}

impl InsertMarkerPlacement {
    pub(in crate::buffer) const fn before_markers(self) -> bool {
        matches!(self, Self::BeforeMarkers)
    }
}

/// A fully measured GNU-style insert operation.
///
/// GNU `insert_1_both` receives the insertion point plus both `nchars` and
/// `nbytes` before touching the gap.  Marker placement is part of the edit
/// operation, not part of a later buffer-state policy, so keep it attached to
/// the measured insertion as it flows through current and indirect buffers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct MeasuredInsertEdit {
    insertion: TextInsertion,
    marker_placement: InsertMarkerPlacement,
    marker_adjustment: InsertMarkerAdjustment,
}

impl MeasuredInsertEdit {
    pub(in crate::buffer) const fn new(
        insertion: TextInsertion,
        marker_placement: InsertMarkerPlacement,
        marker_adjustment: InsertMarkerAdjustment,
    ) -> Self {
        Self {
            insertion,
            marker_placement,
            marker_adjustment,
        }
    }

    pub(in crate::buffer) const fn by_insertion_type(
        insertion: TextInsertion,
        marker_placement: InsertMarkerPlacement,
    ) -> Self {
        Self::new(
            insertion,
            marker_placement,
            InsertMarkerAdjustment::ByInsertionType,
        )
    }

    pub(in crate::buffer) const fn insertion(self) -> TextInsertion {
        self.insertion
    }

    pub(in crate::buffer) const fn is_empty(self) -> bool {
        self.insertion.extent().is_empty()
    }

    pub(in crate::buffer) const fn byte_pos(self) -> EmacsBytePos {
        self.insertion.byte_pos()
    }

    pub(in crate::buffer) const fn char_pos(self) -> CharPos0 {
        self.insertion.char_pos()
    }

    pub(in crate::buffer) const fn extent(self) -> TextExtent {
        self.insertion.extent()
    }

    pub(in crate::buffer) const fn byte_len(self) -> EmacsByteLen {
        self.insertion.extent().emacs_bytes()
    }

    pub(in crate::buffer) const fn char_len(self) -> CharLen {
        self.insertion.extent().chars()
    }

    pub(in crate::buffer) const fn marker_placement(self) -> InsertMarkerPlacement {
        self.marker_placement
    }

    pub(in crate::buffer) const fn marker_adjustment(self) -> InsertMarkerAdjustment {
        self.marker_adjustment
    }

    pub(in crate::buffer) const fn before_markers(self) -> bool {
        self.marker_placement.before_markers()
    }

    pub(in crate::buffer) fn state_after(
        self,
        state: BufferEditState,
        policy: InsertSideEffectPolicy,
    ) -> BufferEditState {
        insert_state_after_edit(state, self.insertion, policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct MeasuredDeleteEdit {
    range: TextEditRange,
}

impl MeasuredDeleteEdit {
    pub(in crate::buffer) const fn new(range: TextEditRange) -> Self {
        Self { range }
    }

    pub(in crate::buffer) const fn range(self) -> TextEditRange {
        self.range
    }

    pub(in crate::buffer) const fn is_empty(self) -> bool {
        self.range.is_empty()
    }

    pub(in crate::buffer) const fn char_len(self) -> CharLen {
        self.range.char_len()
    }

    pub(in crate::buffer) fn state_after(
        self,
        state: BufferEditState,
        policy: DeleteSideEffectPolicy,
    ) -> BufferEditState {
        delete_state_after_edit(state, self.range, policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct MeasuredReplaceEdit {
    replacement: TextReplacement,
}

impl MeasuredReplaceEdit {
    pub(in crate::buffer) const fn new(replacement: TextReplacement) -> Self {
        Self { replacement }
    }

    pub(in crate::buffer) const fn replacement(self) -> TextReplacement {
        self.replacement
    }

    pub(in crate::buffer) const fn is_empty(self) -> bool {
        self.replacement.old_range().is_empty() && self.replacement.new_extent().is_empty()
    }

    pub(in crate::buffer) const fn old_range(self) -> TextEditRange {
        self.replacement.old_range()
    }

    pub(in crate::buffer) const fn old_char_start(self) -> CharPos0 {
        self.replacement.old_range().char_start()
    }

    pub(in crate::buffer) const fn old_char_len(self) -> CharLen {
        self.replacement.old_char_len()
    }

    pub(in crate::buffer) const fn new_extent(self) -> TextExtent {
        self.replacement.new_extent()
    }

    pub(in crate::buffer) const fn new_char_len(self) -> CharLen {
        self.replacement.new_char_len()
    }

    pub(in crate::buffer) const fn changed_chars(self) -> CharLen {
        self.replacement.changed_chars()
    }

    pub(in crate::buffer) fn state_after(
        self,
        state: BufferEditState,
        policy: ReplaceSideEffectPolicy,
    ) -> BufferEditState {
        if policy.state_fields.update_state_fields() {
            replace_state_after_edit(state, self.replacement)
        } else {
            state
        }
    }
}

/// A same-byte-length text edit whose storage mutation span can differ from
/// the GNU-visible modified span.
///
/// GNU `subst-char-in-region` is the important case: it rewrites bytes across
/// the requested storage range, but `modify_text` starts at the first changed
/// character and runs through the original end.  `transpose-regions` uses the
/// same range for both.  Keeping both ranges typed avoids leaking raw
/// `changed_chars` counters through the edit pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct MeasuredSameLenEdit {
    storage_range: TextEditRange,
    modified_range: TextEditRange,
}

impl MeasuredSameLenEdit {
    pub(in crate::buffer) const fn new(
        storage_range: TextEditRange,
        modified_range: TextEditRange,
    ) -> Self {
        Self {
            storage_range,
            modified_range,
        }
    }

    pub(in crate::buffer) const fn covering(range: TextEditRange) -> Self {
        Self::new(range, range)
    }

    pub(in crate::buffer) const fn storage_range(self) -> TextEditRange {
        self.storage_range
    }

    pub(in crate::buffer) const fn modified_range(self) -> TextEditRange {
        self.modified_range
    }

    pub(in crate::buffer) const fn is_empty(self) -> bool {
        self.storage_range.is_empty() || self.modified_range.is_empty()
    }

    pub(in crate::buffer) const fn changed_chars(self) -> CharLen {
        self.modified_range.char_len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum SharedTextEditMetadata {
    Insert(MeasuredInsertEdit),
    Delete(MeasuredDeleteEdit),
    Replace(MeasuredReplaceEdit),
    SameLen {
        edit: MeasuredSameLenEdit,
        preserve_modified_state: bool,
    },
    Transposition {
        edit: MeasuredSameLenEdit,
        transposition: TextTransposition,
        preserve_modified_state: bool,
    },
}

impl SharedTextEditMetadata {
    pub(in crate::buffer) fn state_policy_for_shared_sibling(
        self,
        state_update: impl FnOnce() -> SharedBufferStateUpdate,
    ) -> SharedTextEditStatePolicy {
        match self {
            Self::Insert(_) | Self::Delete(_) | Self::Replace(_) | Self::Transposition { .. } => {
                SharedTextEditStatePolicy::StateFields(state_update())
            }
            Self::SameLen { .. } => SharedTextEditStatePolicy::NoStateFields,
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::buffer) struct SharedTextEditScope {
    edited_id: BufferId,
    buffer_ids: Vec<BufferId>,
}

impl SharedTextEditScope {
    pub(in crate::buffer) fn new(edited_id: BufferId, buffer_ids: Vec<BufferId>) -> Self {
        Self {
            edited_id,
            buffer_ids,
        }
    }

    pub(in crate::buffer) fn siblings(&self) -> impl Iterator<Item = BufferId> + '_ {
        self.buffer_ids
            .iter()
            .copied()
            .filter(|buffer_id| *buffer_id != self.edited_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum SharedBufferStateUpdate {
    UpdateFields,
    RefreshFromStateMarkers,
}

impl SharedBufferStateUpdate {
    pub(in crate::buffer) const fn update_state_fields(self) -> bool {
        matches!(self, Self::UpdateFields)
    }

    pub(in crate::buffer) const fn needs_state_marker_refresh(self) -> bool {
        matches!(self, Self::RefreshFromStateMarkers)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum SharedTextEditStatePolicy {
    NoStateFields,
    StateFields(SharedBufferStateUpdate),
}

impl SharedTextEditStatePolicy {
    pub(in crate::buffer) const fn state_update(self) -> Option<SharedBufferStateUpdate> {
        match self {
            Self::NoStateFields => None,
            Self::StateFields(state_update) => Some(state_update),
        }
    }

    pub(in crate::buffer) const fn structural_state_update(self) -> SharedBufferStateUpdate {
        match self {
            Self::StateFields(state_update) => state_update,
            Self::NoStateFields => {
                panic!("shared structural edit requires state update policy")
            }
        }
    }
}

/// Backend-neutral plan for GNU `subst-char-in-region`.
///
/// GNU scans the buffer once to find each single-character replacement, records
/// undo per changed character, then rewrites the original storage range in
/// place because FROM and TO have the same Emacs-byte length.  Keeping the
/// replacement bytes together with the per-character changed ranges prevents
/// callers from recomputing those paired byte/char spans independently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct SameLenSubstitutionPlan {
    replacement_bytes: Vec<u8>,
    changed_ranges: Vec<TextEditRange>,
}

impl SameLenSubstitutionPlan {
    pub(in crate::buffer) fn new(
        range: TextEditRange,
        region_bytes: &[u8],
        multibyte: bool,
        from_code: u32,
        to_bytes: &[u8],
    ) -> Option<Self> {
        let mut replacement_bytes = Vec::with_capacity(region_bytes.len());
        let mut changed_ranges = Vec::new();
        if multibyte {
            Self::append_multibyte_substitutions(
                range,
                region_bytes,
                from_code,
                to_bytes,
                &mut replacement_bytes,
                &mut changed_ranges,
            );
        } else {
            Self::append_unibyte_substitutions(
                range,
                region_bytes,
                from_code,
                to_bytes,
                &mut replacement_bytes,
                &mut changed_ranges,
            )?;
        }

        if changed_ranges.is_empty() {
            None
        } else {
            Some(Self {
                replacement_bytes,
                changed_ranges,
            })
        }
    }

    fn append_multibyte_substitutions(
        range: TextEditRange,
        region_bytes: &[u8],
        from_code: u32,
        to_bytes: &[u8],
        replacement_bytes: &mut Vec<u8>,
        changed_ranges: &mut Vec<TextEditRange>,
    ) {
        let mut byte_offset = 0;
        let mut char_offset = 0;
        while byte_offset < region_bytes.len() {
            let (code, len) =
                crate::emacs_core::emacs_char::string_char(&region_bytes[byte_offset..]);
            let clen = len.max(1);
            if code == from_code {
                debug_assert_eq!(
                    clen,
                    to_bytes.len(),
                    "subst-char-in-region: matched char byte length ({}) must equal replacement length ({})",
                    clen,
                    to_bytes.len()
                );
                replacement_bytes.extend_from_slice(to_bytes);
                changed_ranges.push(range.subrange_from_start_offsets(
                    EmacsByteLen::new(byte_offset),
                    CharLen::new(char_offset),
                    TextExtent::new(CharLen::new(1), EmacsByteLen::new(clen)),
                ));
            } else {
                replacement_bytes.extend_from_slice(&region_bytes[byte_offset..byte_offset + clen]);
            }
            byte_offset += clen;
            char_offset += 1;
        }
    }

    fn append_unibyte_substitutions(
        range: TextEditRange,
        region_bytes: &[u8],
        from_code: u32,
        to_bytes: &[u8],
        replacement_bytes: &mut Vec<u8>,
        changed_ranges: &mut Vec<TextEditRange>,
    ) -> Option<()> {
        if from_code > 0xFF || to_bytes.len() != 1 {
            return None;
        }
        let from_byte = from_code as u8;
        for (index, &byte) in region_bytes.iter().enumerate() {
            if byte == from_byte {
                replacement_bytes.push(to_bytes[0]);
                changed_ranges.push(range.subrange_from_start_offsets(
                    EmacsByteLen::new(index),
                    CharLen::new(index),
                    TextExtent::new(CharLen::new(1), EmacsByteLen::new(1)),
                ));
            } else {
                replacement_bytes.push(byte);
            }
        }
        Some(())
    }

    pub(in crate::buffer) fn replacement_bytes(&self) -> &[u8] {
        &self.replacement_bytes
    }

    pub(in crate::buffer) fn changed_ranges(&self) -> &[TextEditRange] {
        &self.changed_ranges
    }

    pub(in crate::buffer) fn first_to_last_changed_range(&self) -> TextEditRange {
        let first = self
            .changed_ranges
            .first()
            .expect("substitution plan should contain at least one changed range");
        let last = self
            .changed_ranges
            .last()
            .expect("substitution plan should contain at least one changed range");
        TextEditRange::from_start_end(first.start_anchor(), last.end_anchor())
    }

    pub(in crate::buffer) fn replacement_for_range(
        &self,
        range: TextEditRange,
        multibyte: bool,
    ) -> TextReplacement {
        TextReplacement::new(
            range,
            TextExtent::from_emacs_bytes(&self.replacement_bytes, multibyte),
        )
    }
}

/// Backend-neutral storage plan for GNU `transpose-regions`.
///
/// GNU transposes bytes over the full `[start1, end2)` span without changing
/// the span's total size.  Undo records and text properties still have special
/// character-length cases, so this plan only owns the storage replacement and
/// the measured same-size edit descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct TranspositionStoragePlan {
    replacement_bytes: Vec<u8>,
    replacement: TextReplacement,
    edit: MeasuredSameLenEdit,
}

impl TranspositionStoragePlan {
    pub(in crate::buffer) fn new(
        transposition: TextTransposition,
        first: &[u8],
        middle: &[u8],
        second: &[u8],
    ) -> Self {
        let span = transposition.span_edit_range();
        let mut replacement_bytes = Vec::with_capacity(span.byte_len().get());
        replacement_bytes.extend_from_slice(second);
        replacement_bytes.extend_from_slice(middle);
        replacement_bytes.extend_from_slice(first);
        debug_assert_eq!(
            replacement_bytes.len(),
            span.byte_len().get(),
            "transpose-regions storage replacement must preserve byte length"
        );
        let replacement = TextReplacement::new(span, span.extent());
        Self {
            replacement_bytes,
            replacement,
            edit: MeasuredSameLenEdit::covering(span),
        }
    }

    pub(in crate::buffer) fn replacement_bytes(&self) -> &[u8] {
        &self.replacement_bytes
    }

    pub(in crate::buffer) const fn replacement(&self) -> TextReplacement {
        self.replacement
    }

    pub(in crate::buffer) const fn edit(&self) -> MeasuredSameLenEdit {
        self.edit
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum BufferStateFieldUpdatePolicy {
    Update,
    Skip,
}

impl BufferStateFieldUpdatePolicy {
    pub(in crate::buffer) const fn from_shared_update(
        state_update: SharedBufferStateUpdate,
    ) -> Self {
        if state_update.update_state_fields() {
            Self::Update
        } else {
            Self::Skip
        }
    }

    pub(in crate::buffer) const fn update_state_fields(self) -> bool {
        matches!(self, Self::Update)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum AccessibleStartUpdatePolicy {
    Preserve,
    ShiftAfterEdit,
}

impl AccessibleStartUpdatePolicy {
    pub(in crate::buffer) const fn shift_after_edit(self) -> bool {
        matches!(self, Self::ShiftAfterEdit)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum InsertPointUpdatePolicy {
    AdvanceAtInsertion,
    StayBeforeInsertion,
}

impl InsertPointUpdatePolicy {
    pub(in crate::buffer) const fn advance_point_at_insertion(self) -> bool {
        matches!(self, Self::AdvanceAtInsertion)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) enum SharedTextSideDataPolicy {
    AdjustInThisBuffer,
    AlreadyAdjustedInSharedText,
}

impl SharedTextSideDataPolicy {
    pub(in crate::buffer) const fn adjust_in_this_buffer(self) -> bool {
        matches!(self, Self::AdjustInThisBuffer)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct InsertSideEffectPolicy {
    pub(in crate::buffer) state_fields: BufferStateFieldUpdatePolicy,
    pub(in crate::buffer) accessible_start: AccessibleStartUpdatePolicy,
    pub(in crate::buffer) point_at_insertion: InsertPointUpdatePolicy,
    pub(in crate::buffer) shared_side_data: SharedTextSideDataPolicy,
}

impl InsertSideEffectPolicy {
    pub(in crate::buffer) fn current_buffer() -> Self {
        Self {
            state_fields: BufferStateFieldUpdatePolicy::Update,
            accessible_start: AccessibleStartUpdatePolicy::Preserve,
            point_at_insertion: InsertPointUpdatePolicy::AdvanceAtInsertion,
            shared_side_data: SharedTextSideDataPolicy::AdjustInThisBuffer,
        }
    }

    pub(in crate::buffer) fn shared_buffer(state_update: SharedBufferStateUpdate) -> Self {
        Self {
            state_fields: BufferStateFieldUpdatePolicy::from_shared_update(state_update),
            accessible_start: AccessibleStartUpdatePolicy::ShiftAfterEdit,
            point_at_insertion: InsertPointUpdatePolicy::StayBeforeInsertion,
            shared_side_data: SharedTextSideDataPolicy::AlreadyAdjustedInSharedText,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct DeleteSideEffectPolicy {
    pub(in crate::buffer) state_fields: BufferStateFieldUpdatePolicy,
    pub(in crate::buffer) accessible_start: AccessibleStartUpdatePolicy,
    pub(in crate::buffer) shared_side_data: SharedTextSideDataPolicy,
}

impl DeleteSideEffectPolicy {
    pub(in crate::buffer) fn current_buffer() -> Self {
        Self {
            state_fields: BufferStateFieldUpdatePolicy::Update,
            accessible_start: AccessibleStartUpdatePolicy::Preserve,
            shared_side_data: SharedTextSideDataPolicy::AdjustInThisBuffer,
        }
    }

    pub(in crate::buffer) fn shared_buffer(state_update: SharedBufferStateUpdate) -> Self {
        Self {
            state_fields: BufferStateFieldUpdatePolicy::from_shared_update(state_update),
            accessible_start: AccessibleStartUpdatePolicy::ShiftAfterEdit,
            shared_side_data: SharedTextSideDataPolicy::AlreadyAdjustedInSharedText,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct ReplaceSideEffectPolicy {
    pub(in crate::buffer) state_fields: BufferStateFieldUpdatePolicy,
    pub(in crate::buffer) shared_side_data: SharedTextSideDataPolicy,
}

impl ReplaceSideEffectPolicy {
    pub(in crate::buffer) fn current_buffer() -> Self {
        Self {
            state_fields: BufferStateFieldUpdatePolicy::Update,
            shared_side_data: SharedTextSideDataPolicy::AdjustInThisBuffer,
        }
    }

    pub(in crate::buffer) fn shared_buffer(state_update: SharedBufferStateUpdate) -> Self {
        Self {
            state_fields: BufferStateFieldUpdatePolicy::from_shared_update(state_update),
            shared_side_data: SharedTextSideDataPolicy::AlreadyAdjustedInSharedText,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(
        pt_byte: usize,
        pt: usize,
        begv_byte: usize,
        begv: usize,
        zv_byte: usize,
        zv: usize,
    ) -> BufferEditState {
        BufferEditState::new(
            TextPositionAnchor::from_usize(pt, pt_byte),
            TextPositionAnchor::from_usize(begv, begv_byte),
            TextPositionAnchor::from_usize(zv, zv_byte),
        )
    }

    fn replace_state(old: BufferEditState) -> BufferEditState {
        replace_state_after_edit(
            old,
            TextReplacement::new(
                TextEditRange::from_usize(20, 36, 10, 18),
                crate::buffer::TextExtent::from_usize(3, 5),
            ),
        )
    }

    fn same_len_edit() -> MeasuredSameLenEdit {
        MeasuredSameLenEdit::new(
            TextEditRange::from_usize(0, 10, 0, 10),
            TextEditRange::from_usize(3, 10, 3, 10),
        )
    }

    fn insertion() -> TextInsertion {
        TextInsertion::from_usize(20, 10, 3, 5)
    }

    fn deleted_range() -> TextEditRange {
        TextEditRange::from_usize(20, 36, 10, 18)
    }

    fn measured_insert_edit() -> MeasuredInsertEdit {
        MeasuredInsertEdit::by_insertion_type(insertion(), InsertMarkerPlacement::AfterMarkers)
    }

    fn measured_delete_edit() -> MeasuredDeleteEdit {
        MeasuredDeleteEdit::new(deleted_range())
    }

    fn measured_replace_edit() -> MeasuredReplaceEdit {
        MeasuredReplaceEdit::new(TextReplacement::new(
            deleted_range(),
            TextExtent::from_usize(3, 5),
        ))
    }

    fn state_policy_for_shared_sibling(edit: SharedTextEditMetadata) -> SharedTextEditStatePolicy {
        edit.state_policy_for_shared_sibling(|| SharedBufferStateUpdate::RefreshFromStateMarkers)
    }

    #[test]
    fn shared_edit_metadata_derives_sibling_state_policy() {
        assert_eq!(
            state_policy_for_shared_sibling(SharedTextEditMetadata::Insert(measured_insert_edit())),
            SharedTextEditStatePolicy::StateFields(
                SharedBufferStateUpdate::RefreshFromStateMarkers
            )
        );
        assert_eq!(
            state_policy_for_shared_sibling(SharedTextEditMetadata::Delete(measured_delete_edit())),
            SharedTextEditStatePolicy::StateFields(
                SharedBufferStateUpdate::RefreshFromStateMarkers
            )
        );
        assert_eq!(
            state_policy_for_shared_sibling(SharedTextEditMetadata::Replace(
                measured_replace_edit()
            )),
            SharedTextEditStatePolicy::StateFields(
                SharedBufferStateUpdate::RefreshFromStateMarkers
            )
        );
        assert_eq!(
            state_policy_for_shared_sibling(SharedTextEditMetadata::Transposition {
                edit: same_len_edit(),
                transposition: TextTransposition::from_usize(2, 5, 1, 3, 8, 10, 5, 7),
                preserve_modified_state: true,
            }),
            SharedTextEditStatePolicy::StateFields(
                SharedBufferStateUpdate::RefreshFromStateMarkers
            )
        );
        assert_eq!(
            state_policy_for_shared_sibling(SharedTextEditMetadata::SameLen {
                edit: same_len_edit(),
                preserve_modified_state: true,
            }),
            SharedTextEditStatePolicy::NoStateFields
        );
    }

    #[test]
    fn insert_state_current_buffer_advances_point_at_insert_and_zv() {
        assert_eq!(
            insert_state_after_edit(
                state(20, 10, 0, 0, 60, 42),
                insertion(),
                InsertSideEffectPolicy::current_buffer(),
            ),
            state(25, 13, 0, 0, 65, 45)
        );
    }

    #[test]
    fn insert_state_shared_buffer_keeps_point_at_insert_and_shifts_begv_after_insert() {
        assert_eq!(
            insert_state_after_edit(
                state(20, 10, 28, 14, 60, 42),
                insertion(),
                InsertSideEffectPolicy::shared_buffer(SharedBufferStateUpdate::UpdateFields),
            ),
            state(20, 10, 33, 17, 65, 45)
        );
    }

    #[test]
    fn insert_state_shifts_zv_at_insert_position() {
        assert_eq!(
            insert_state_after_edit(
                state(0, 0, 0, 0, 20, 10),
                insertion(),
                InsertSideEffectPolicy::shared_buffer(SharedBufferStateUpdate::UpdateFields),
            ),
            state(0, 0, 0, 0, 25, 13)
        );
    }

    #[test]
    fn delete_state_current_buffer_maps_point_inside_range_to_deleted_start() {
        assert_eq!(
            delete_state_after_edit(
                state(28, 14, 0, 0, 60, 42),
                deleted_range(),
                DeleteSideEffectPolicy::current_buffer(),
            ),
            state(20, 10, 0, 0, 44, 34)
        );
    }

    #[test]
    fn delete_state_keeps_point_at_deleted_start() {
        assert_eq!(
            delete_state_after_edit(
                state(20, 10, 0, 0, 60, 42),
                deleted_range(),
                DeleteSideEffectPolicy::current_buffer(),
            ),
            state(20, 10, 0, 0, 44, 34)
        );
    }

    #[test]
    fn delete_state_shared_buffer_shifts_point_begv_and_zv() {
        assert_eq!(
            delete_state_after_edit(
                state(44, 24, 28, 14, 60, 42),
                deleted_range(),
                DeleteSideEffectPolicy::shared_buffer(SharedBufferStateUpdate::UpdateFields),
            ),
            state(28, 16, 20, 10, 44, 34)
        );
    }

    #[test]
    fn insert_and_delete_state_skip_update_when_policy_disables_state_fields() {
        let original = state(44, 24, 28, 14, 60, 42);

        assert_eq!(
            insert_state_after_edit(
                original,
                insertion(),
                InsertSideEffectPolicy::shared_buffer(
                    SharedBufferStateUpdate::RefreshFromStateMarkers,
                ),
            ),
            original
        );
        assert_eq!(
            delete_state_after_edit(
                original,
                deleted_range(),
                DeleteSideEffectPolicy::shared_buffer(
                    SharedBufferStateUpdate::RefreshFromStateMarkers,
                ),
            ),
            original
        );
    }

    #[test]
    fn replace_state_maps_point_inside_deleted_range_to_replacement_end() {
        assert_eq!(
            replace_state(state(28, 14, 0, 0, 60, 42)),
            state(25, 13, 0, 0, 49, 37)
        );
    }

    #[test]
    fn replace_state_keeps_point_at_deleted_start() {
        assert_eq!(
            replace_state(state(20, 10, 0, 0, 60, 42)),
            state(20, 10, 0, 0, 49, 37)
        );
    }

    #[test]
    fn replace_state_maps_point_at_deleted_end_to_replacement_end() {
        assert_eq!(
            replace_state(state(36, 18, 0, 0, 60, 42)),
            state(25, 13, 0, 0, 49, 37)
        );
    }

    #[test]
    fn replace_state_shifts_point_after_deleted_range_by_extent_delta() {
        assert_eq!(
            replace_state(state(44, 24, 0, 0, 60, 42)),
            state(33, 19, 0, 0, 49, 37)
        );
    }

    #[test]
    fn replace_state_clamps_begv_inside_deleted_range_to_deleted_start() {
        assert_eq!(
            replace_state(state(0, 0, 28, 14, 60, 42)),
            state(0, 0, 20, 10, 49, 37)
        );
    }

    #[test]
    fn replace_state_maps_begv_at_deleted_end_to_deleted_start() {
        assert_eq!(
            replace_state(state(0, 0, 36, 18, 60, 42)),
            state(0, 0, 20, 10, 49, 37)
        );
    }

    #[test]
    fn replace_state_maps_zv_inside_deleted_range_to_replacement_end() {
        assert_eq!(
            replace_state(state(0, 0, 0, 0, 28, 14)),
            state(0, 0, 0, 0, 25, 13)
        );
    }

    #[test]
    fn modification_tick_delta_is_logarithmic_and_never_zero() {
        assert_eq!(modification_tick_delta(CharLen::new(0)), 1);
        assert_eq!(modification_tick_delta(CharLen::new(1)), 1);
        assert_eq!(modification_tick_delta(CharLen::new(2)), 2);
        assert_eq!(modification_tick_delta(CharLen::new(3)), 2);
        assert_eq!(modification_tick_delta(CharLen::new(4)), 3);
        assert_eq!(modification_tick_delta(CharLen::new(8)), 4);
    }

    #[test]
    fn same_len_edit_keeps_storage_and_modified_ranges_separate() {
        let edit = same_len_edit();

        assert_eq!(
            edit.storage_range(),
            TextEditRange::from_usize(0, 10, 0, 10)
        );
        assert_eq!(
            edit.modified_range(),
            TextEditRange::from_usize(3, 10, 3, 10)
        );
        assert_eq!(edit.changed_chars(), CharLen::new(7));
    }

    #[test]
    fn same_len_substitution_plan_records_per_character_multibyte_ranges() {
        let range = TextEditRange::from_usize(0, "a日本日".len(), 0, 4);
        let plan = SameLenSubstitutionPlan::new(
            range,
            "a日本日".as_bytes(),
            true,
            '日' as u32,
            "本".as_bytes(),
        )
        .expect("matching chars should produce a substitution plan");

        assert_eq!(plan.replacement_bytes(), "a本本本".as_bytes());
        assert_eq!(
            plan.changed_ranges(),
            &[
                TextEditRange::from_usize(1, 4, 1, 2),
                TextEditRange::from_usize(7, 10, 3, 4),
            ]
        );
        assert_eq!(
            plan.first_to_last_changed_range(),
            TextEditRange::from_usize(1, 10, 1, 4)
        );
        assert_eq!(
            plan.replacement_for_range(range, true),
            TextReplacement::new(range, TextExtent::from_usize(4, "a本本本".len()))
        );
    }

    #[test]
    fn same_len_substitution_plan_records_unibyte_ranges_and_rejects_non_bytes() {
        let range = TextEditRange::from_usize(20, 25, 10, 15);
        let plan = SameLenSubstitutionPlan::new(range, b"ababa", false, b'a' as u32, b"z")
            .expect("matching unibyte chars should produce a substitution plan");

        assert_eq!(plan.replacement_bytes(), b"zbzbz");
        assert_eq!(
            plan.changed_ranges(),
            &[
                TextEditRange::from_usize(20, 21, 10, 11),
                TextEditRange::from_usize(22, 23, 12, 13),
                TextEditRange::from_usize(24, 25, 14, 15),
            ]
        );
        assert_eq!(
            plan.first_to_last_changed_range(),
            TextEditRange::from_usize(20, 25, 10, 15)
        );
        assert!(SameLenSubstitutionPlan::new(range, b"ababa", false, 0x100, b"z").is_none());
        assert!(SameLenSubstitutionPlan::new(range, b"ababa", false, b'a' as u32, b"zz").is_none());
    }

    #[test]
    fn same_len_substitution_plan_returns_none_without_matches() {
        let range = TextEditRange::from_usize(0, 5, 0, 5);

        assert!(SameLenSubstitutionPlan::new(range, b"abcde", true, b'z' as u32, b"q").is_none());
    }

    #[test]
    fn transposition_storage_plan_swaps_outer_regions_over_full_span() {
        let transposition = TextTransposition::from_usize(2, 5, 1, 3, 8, 10, 5, 7);
        let plan = TranspositionStoragePlan::new(transposition, b"abc", b"XYZ", b"de");
        let span = TextEditRange::from_usize(2, 10, 1, 7);

        assert_eq!(plan.replacement_bytes(), b"deXYZabc");
        assert_eq!(
            plan.replacement(),
            TextReplacement::new(span, span.extent())
        );
        assert_eq!(plan.edit(), MeasuredSameLenEdit::covering(span));
    }
}
