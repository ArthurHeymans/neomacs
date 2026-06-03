//! Window-marker integration.
//!
//! GNU Emacs stores window positions (`w->start`, `w->pointm`, `w->old_pointm`)
//! as `Lisp_Marker` objects registered on the owning buffer's intrusive marker
//! chain.  When text is inserted or deleted, the chain automatically adjusts
//! every marker's position, so window positions stay correct without explicit
//! per-window patching.
//!
//! neomacs mirrors this: each `Window::Leaf` carries three marker IDs
//! (`start_marker_id`, `point_marker_id`, `old_point_marker_id`) alongside
//! cached `usize` byte positions.  The markers are the source of truth; the
//! caches are refreshed by `sync_window_positions_from_markers` after every
//! text edit.

use crate::buffer::{Buffer, BufferId, BufferManager, InsertionType, TextPositionAnchor};
use crate::window::{Frame, FrameManager, Window, WindowId};

/// Window-start markers use `InsertionType::Before` so the marker stays
/// before text inserted at the window start position, matching GNU `w->start`.
const START_INSERTION_TYPE: InsertionType = InsertionType::Before;
/// Window-point markers use `InsertionType::Before` so the marker does not
/// advance past text inserted at point, matching GNU `w->pointm`.
const POINT_INSERTION_TYPE: InsertionType = InsertionType::Before;
/// Old-point markers use `InsertionType::Before`, matching GNU `w->old_pointm`.
const OLD_POINT_INSERTION_TYPE: InsertionType = InsertionType::Before;

fn lisp_position_to_restricted_marker_position(
    bm: &BufferManager,
    buffer_id: BufferId,
    lisp_position: usize,
) -> TextPositionAnchor {
    let Some(buffer) = bm.get(buffer_id) else {
        return TextPositionAnchor::from_usize(
            lisp_position.saturating_sub(1),
            lisp_position.saturating_sub(1),
        );
    };
    restricted_marker_position(buffer, lisp_position)
}

fn restricted_marker_position(buffer: &Buffer, lisp_position: usize) -> TextPositionAnchor {
    let char_pos = lisp_position
        .saturating_sub(1)
        .clamp(buffer.point_min_char(), buffer.point_max_char());
    let byte_pos = buffer.char_to_byte_clamped(char_pos);
    TextPositionAnchor::from_usize(char_pos, byte_pos)
}

fn marker_lisp_position(bm: &BufferManager, buffer_id: BufferId, marker_id: u64) -> Option<usize> {
    bm.marker_char_position(buffer_id, marker_id)
        .map(|char_pos| char_pos.saturating_add(1).max(1))
}

pub fn create_window_markers(bm: &mut BufferManager, window: &mut Window, buffer_id: BufferId) {
    let Window::Leaf {
        window_start,
        start_marker_id,
        point,
        point_marker_id,
        old_point,
        old_point_marker_id,
        ..
    } = window
    else {
        return;
    };

    let start = lisp_position_to_restricted_marker_position(bm, buffer_id, *window_start);
    let (start_mid, _) =
        bm.create_marker(buffer_id, start.emacs_byte_pos.get(), START_INSERTION_TYPE);
    *start_marker_id = Some(start_mid);

    let point = lisp_position_to_restricted_marker_position(bm, buffer_id, *point);
    let (pt_mid, _) = bm.create_marker(buffer_id, point.emacs_byte_pos.get(), POINT_INSERTION_TYPE);
    *point_marker_id = Some(pt_mid);

    let old_point = lisp_position_to_restricted_marker_position(bm, buffer_id, *old_point);
    let (op_mid, _) = bm.create_marker(
        buffer_id,
        old_point.emacs_byte_pos.get(),
        OLD_POINT_INSERTION_TYPE,
    );
    *old_point_marker_id = Some(op_mid);
}

pub fn unchain_window_markers(bm: &mut BufferManager, window: &mut Window) {
    let Window::Leaf {
        buffer_id,
        start_marker_id,
        point_marker_id,
        old_point_marker_id,
        ..
    } = window
    else {
        return;
    };

    let buf_id = *buffer_id;
    if let Some(mid) = start_marker_id.take() {
        if let Some(buf) = bm.get(buf_id) {
            buf.text.remove_marker(mid);
        }
    }
    if let Some(mid) = point_marker_id.take() {
        if let Some(buf) = bm.get(buf_id) {
            buf.text.remove_marker(mid);
        }
    }
    if let Some(mid) = old_point_marker_id.take() {
        if let Some(buf) = bm.get(buf_id) {
            buf.text.remove_marker(mid);
        }
    }
}

fn move_marker(
    bm: &mut BufferManager,
    buffer_id: BufferId,
    marker_id: Option<u64>,
    lisp_position: usize,
) {
    let Some(mid) = marker_id else { return };
    let position = lisp_position_to_restricted_marker_position(bm, buffer_id, lisp_position);
    if let Some(buf) = bm.get(buffer_id) {
        buf.text
            .move_marker_to_position(mid, position.emacs_byte_pos, position.char_pos);
    }
}

pub fn set_window_start_with_marker(
    bm: &mut BufferManager,
    window: &mut Window,
    lisp_position: usize,
) {
    let Window::Leaf {
        buffer_id,
        window_start,
        start_marker_id,
        ..
    } = window
    else {
        return;
    };
    *window_start = lisp_position;
    move_marker(bm, *buffer_id, *start_marker_id, lisp_position);
}

pub fn set_window_point_with_marker(
    bm: &mut BufferManager,
    window: &mut Window,
    lisp_position: usize,
) {
    let Window::Leaf {
        buffer_id,
        point,
        point_marker_id,
        ..
    } = window
    else {
        return;
    };
    *point = lisp_position;
    move_marker(bm, *buffer_id, *point_marker_id, lisp_position);
}

pub fn set_window_old_point_with_marker(
    bm: &mut BufferManager,
    window: &mut Window,
    lisp_position: usize,
) {
    let Window::Leaf {
        buffer_id,
        old_point,
        old_point_marker_id,
        ..
    } = window
    else {
        return;
    };
    *old_point = lisp_position;
    move_marker(bm, *buffer_id, *old_point_marker_id, lisp_position);
}

/// Refresh cached `usize` positions on every leaf window from its markers.
///
/// Call this after text edits (insert/delete) so that the `usize` caches
/// reflect the auto-adjusted marker positions.  Only windows whose buffer
/// matches `edited_buffer_id` need updating.
pub fn sync_window_positions_from_markers(
    frame: &mut Frame,
    bm: &BufferManager,
    edited_buffer_id: BufferId,
) {
    sync_subtree(&mut frame.root_window, bm, edited_buffer_id);
    if let Some(ref mut mini) = frame.minibuffer_leaf {
        sync_leaf(mini, bm, edited_buffer_id);
    }
}

fn sync_subtree(window: &mut Window, bm: &BufferManager, edited_buffer_id: BufferId) {
    match window {
        Window::Leaf { .. } => sync_leaf(window, bm, edited_buffer_id),
        Window::Internal { children, .. } => {
            for child in children {
                sync_subtree(child, bm, edited_buffer_id);
            }
        }
    }
}

fn sync_leaf(window: &mut Window, bm: &BufferManager, edited_buffer_id: BufferId) {
    let Window::Leaf {
        buffer_id,
        window_start,
        start_marker_id,
        point,
        point_marker_id,
        old_point,
        old_point_marker_id,
        ..
    } = window
    else {
        return;
    };

    if *buffer_id != edited_buffer_id {
        return;
    }

    if let Some(mid) = *start_marker_id {
        if let Some(pos) = marker_lisp_position(bm, *buffer_id, mid) {
            *window_start = pos;
        }
    }
    if let Some(mid) = *point_marker_id {
        if let Some(pos) = marker_lisp_position(bm, *buffer_id, mid) {
            *point = pos;
        }
    }
    if let Some(mid) = *old_point_marker_id {
        if let Some(pos) = marker_lisp_position(bm, *buffer_id, mid) {
            *old_point = pos;
        }
    }
}

/// Walk all frames and sync windows for the given buffer.
pub fn sync_all_frames_for_buffer(
    frames: &mut FrameManager,
    bm: &BufferManager,
    edited_buffer_id: BufferId,
) {
    for frame in frames.frames_mut() {
        sync_window_positions_from_markers(frame, bm, edited_buffer_id);
    }
}
