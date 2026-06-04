//! GNU-shaped buffer edit transaction policy.
//!
//! This module names the semantic side-effect policy used by the structural
//! edit pipeline.  The actual executor still lives in `insdel.rs`; keeping
//! these types separate is the first step toward a central insert/delete/
//! replace transaction boundary.

use crate::buffer::{BufferText, CharPos0, EmacsBytePos, TextEditRange, TextExtent};
use crate::heap_types::LispString;

#[inline]
pub(in crate::buffer) fn emacs_char_count(bytes: &[u8], multibyte: bool) -> usize {
    if multibyte {
        crate::emacs_core::emacs_char::chars_in_multibyte(bytes)
    } else {
        bytes.len()
    }
}

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
pub(in crate::buffer) fn char_pos_for_emacs_byte(text: &BufferText, byte_pos: usize) -> CharPos0 {
    text.emacs_byte_pos_to_char_pos(EmacsBytePos::new(byte_pos))
}

#[inline]
pub(in crate::buffer) fn emacs_byte_for_char_pos(
    text: &BufferText,
    char_pos: usize,
) -> EmacsBytePos {
    text.char_pos_to_emacs_byte_pos(CharPos0::new(char_pos))
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

    if !target_multibyte {
        // GNU: insert_from_gap for unibyte buffers sets nchars=nbytes,
        // storing each byte of the multibyte internal representation as
        // a separate character.  Do NOT mask character codes with 0xFF
        // because that would truncate non-ASCII characters.
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
pub(in crate::buffer) fn transpose_position(
    pos: usize,
    start1: usize,
    end1: usize,
    start2: usize,
    end2: usize,
) -> usize {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct BufferEditState {
    pub(in crate::buffer) pt_byte: usize,
    pub(in crate::buffer) pt: usize,
    pub(in crate::buffer) begv_byte: usize,
    pub(in crate::buffer) begv: usize,
    pub(in crate::buffer) zv_byte: usize,
    pub(in crate::buffer) zv: usize,
}

impl BufferEditState {
    pub(in crate::buffer) fn new(
        pt_byte: usize,
        pt: usize,
        begv_byte: usize,
        begv: usize,
        zv_byte: usize,
        zv: usize,
    ) -> Self {
        Self {
            pt_byte,
            pt,
            begv_byte,
            begv,
            zv_byte,
            zv,
        }
    }
}

pub(in crate::buffer) fn replace_state_after_edit(
    mut state: BufferEditState,
    old_range: TextEditRange,
    new_extent: TextExtent,
) -> BufferEditState {
    let start = old_range.byte_start_usize();
    let end = old_range.byte_end_usize();
    let start_char = old_range.char_start_usize();
    let end_char = old_range.char_end_usize();
    let old_byte_len = old_range.byte_len().get();
    let old_char_len = old_range.char_len().get();
    let new_byte_len = new_extent.emacs_bytes().get();
    let new_char_len = new_extent.chars().get();
    let old_pt_byte = state.pt_byte;
    let old_pt = state.pt;

    if start < old_pt_byte || old_pt_byte == end {
        let clamped = old_pt_byte.min(end);
        state.pt_byte = old_pt_byte + start + new_byte_len - clamped;
        let clamped_char = old_pt.min(end_char);
        state.pt = old_pt + start_char + new_char_len - clamped_char;
    } else if old_pt_byte > end {
        state.pt_byte = old_pt_byte + new_byte_len - old_byte_len;
        state.pt = old_pt + new_char_len - old_char_len;
    }

    if state.begv_byte > end {
        state.begv_byte = state.begv_byte + new_byte_len - old_byte_len;
        state.begv = state.begv + new_char_len - old_char_len;
    } else if state.begv_byte > start {
        state.begv_byte = start;
        state.begv = start_char;
    }

    if state.zv_byte >= end {
        state.zv_byte = state.zv_byte + new_byte_len - old_byte_len;
        state.zv = state.zv + new_char_len - old_char_len;
    } else if state.zv_byte > start {
        state.zv_byte = start + new_byte_len;
        state.zv = start_char + new_char_len;
    }

    state
}

/// GNU `modiff` increments logarithmically with edit size, and
/// `chars_modiff` is reset to the new `modiff` on each character change.
pub(in crate::buffer) fn modification_tick_delta(changed_chars: usize) -> i64 {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct InsertSideEffectPolicy {
    pub(in crate::buffer) update_state_fields: bool,
    pub(in crate::buffer) shift_begv: bool,
    pub(in crate::buffer) advance_point_at_insert: bool,
    pub(in crate::buffer) adjust_shared_markers: bool,
    pub(in crate::buffer) adjust_shared_text_props: bool,
    pub(in crate::buffer) overlay_before_markers: bool,
    pub(in crate::buffer) marker_adjustment: InsertMarkerAdjustment,
}

impl InsertSideEffectPolicy {
    pub(in crate::buffer) fn current_buffer(
        before_markers: bool,
        marker_adjustment: InsertMarkerAdjustment,
    ) -> Self {
        Self {
            update_state_fields: true,
            shift_begv: false,
            advance_point_at_insert: true,
            adjust_shared_markers: true,
            adjust_shared_text_props: true,
            overlay_before_markers: before_markers,
            marker_adjustment,
        }
    }

    pub(in crate::buffer) fn shared_buffer(
        update_state_fields: bool,
        overlay_before_markers: bool,
        marker_adjustment: InsertMarkerAdjustment,
    ) -> Self {
        Self {
            update_state_fields,
            shift_begv: true,
            advance_point_at_insert: false,
            adjust_shared_markers: false,
            adjust_shared_text_props: false,
            overlay_before_markers,
            marker_adjustment,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct DeleteSideEffectPolicy {
    pub(in crate::buffer) update_state_fields: bool,
    pub(in crate::buffer) shift_begv: bool,
    pub(in crate::buffer) adjust_shared_markers: bool,
    pub(in crate::buffer) adjust_shared_text_props: bool,
}

impl DeleteSideEffectPolicy {
    pub(in crate::buffer) fn current_buffer() -> Self {
        Self {
            update_state_fields: true,
            shift_begv: false,
            adjust_shared_markers: true,
            adjust_shared_text_props: true,
        }
    }

    pub(in crate::buffer) fn shared_buffer(update_state_fields: bool) -> Self {
        Self {
            update_state_fields,
            shift_begv: true,
            adjust_shared_markers: false,
            adjust_shared_text_props: false,
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
        BufferEditState::new(pt_byte, pt, begv_byte, begv, zv_byte, zv)
    }

    fn replace_state(old: BufferEditState) -> BufferEditState {
        replace_state_after_edit(
            old,
            TextEditRange::from_usize(20, 36, 10, 18),
            TextExtent::from_usize(3, 5),
        )
    }

    #[test]
    fn replace_state_maps_point_inside_deleted_range_to_replacement_end() {
        assert_eq!(
            replace_state(state(28, 14, 0, 0, 60, 42)),
            state(25, 13, 0, 0, 49, 37)
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
    fn replace_state_maps_zv_inside_deleted_range_to_replacement_end() {
        assert_eq!(
            replace_state(state(0, 0, 0, 0, 28, 14)),
            state(0, 0, 0, 0, 25, 13)
        );
    }

    #[test]
    fn modification_tick_delta_is_logarithmic_and_never_zero() {
        assert_eq!(modification_tick_delta(0), 1);
        assert_eq!(modification_tick_delta(1), 1);
        assert_eq!(modification_tick_delta(2), 2);
        assert_eq!(modification_tick_delta(3), 2);
        assert_eq!(modification_tick_delta(4), 3);
        assert_eq!(modification_tick_delta(8), 4);
    }
}
