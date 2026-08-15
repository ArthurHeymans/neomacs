// The public `buffer::buffer` path is established across the evaluator.
#[allow(clippy::module_inception)]
pub mod buffer;
pub(crate) mod buffer_text;
mod edit_transaction;
pub(crate) mod gap_buffer;
mod marker_data;
pub mod overlay;
mod overlay_bplus;
pub(crate) mod overlay_index;
mod overlay_order;
pub mod position;
pub mod shared;
pub mod text;
pub mod text_props;
pub mod undo;

pub use buffer::{
    Buffer, BufferId, BufferManager, BufferTextSnapshot, InsertionType, LabeledRestriction,
    LabeledRestrictionLabel, SavedRestrictionKind, SavedRestrictionState,
};
pub(crate) use buffer_text::BufferText;
pub use overlay::{Overlay, OverlayList};
pub use position::{
    AccessibleCharRange, AccessibleEmacsByteRange, CharLen, CharPos0, CharRange, DisplayColumn,
    EmacsByteLen, EmacsBytePos, EmacsByteRange, LispBytePos1, LispCharPos1, TextPositionAnchor,
};
pub use shared::SharedUndoState;
pub use text::{
    BufferTextBackendKind, TextChange, TextEditRange, TextExtent, TextInsertion, TextMetrics,
    TextReplacement, TextTransposition,
};
pub use text_props::{ObjectIntervalPlistRun, ObjectIntervalRun, TextPropertyTable};
pub use undo::{
    UndoRecording, truncate_undo_list, undo_list_boundary, undo_list_contains_boundary,
    undo_list_has_trailing_boundary, undo_list_is_disabled, undo_list_is_empty,
    undo_list_pop_group, undo_list_record_delete, undo_list_record_first_change,
    undo_list_record_insert, undo_list_record_point, undo_list_record_property_change,
};
