//! GNU-shaped buffer edit transaction policy.
//!
//! This module names the semantic side-effect policy used by the structural
//! edit pipeline.  The actual executor still lives in `insdel.rs`; keeping
//! these types separate is the first step toward a central insert/delete/
//! replace transaction boundary.

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
