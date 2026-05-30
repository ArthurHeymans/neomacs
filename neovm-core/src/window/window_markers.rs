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

use crate::buffer::{BufferId, BufferManager, InsertionType};
use crate::window::{Frame, FrameManager, Window, WindowId};

/// Window-start markers use `InsertionType::Before` so the marker stays
/// before text inserted at the window start position, matching GNU `w->start`.
const START_INSERTION_TYPE: InsertionType = InsertionType::Before;
/// Window-point markers use `InsertionType::Before` so the marker does not
/// advance past text inserted at point, matching GNU `w->pointm`.
const POINT_INSERTION_TYPE: InsertionType = InsertionType::Before;
/// Old-point markers use `InsertionType::Before`, matching GNU `w->old_pointm`.
const OLD_POINT_INSERTION_TYPE: InsertionType = InsertionType::Before;

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

    let (start_mid, _) = bm.create_marker(buffer_id, *window_start, START_INSERTION_TYPE);
    *start_marker_id = Some(start_mid);

    let (pt_mid, _) = bm.create_marker(buffer_id, *point, POINT_INSERTION_TYPE);
    *point_marker_id = Some(pt_mid);

    let (op_mid, _) = bm.create_marker(buffer_id, *old_point, OLD_POINT_INSERTION_TYPE);
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
    bytepos: usize,
) {
    let Some(mid) = marker_id else { return };
    if let Some(buf) = bm.get(buffer_id) {
        buf.text.move_marker_to(mid, bytepos, 0);
    }
}

pub fn set_window_start_with_marker(bm: &mut BufferManager, window: &mut Window, bytepos: usize) {
    let Window::Leaf {
        buffer_id,
        window_start,
        start_marker_id,
        ..
    } = window
    else {
        return;
    };
    *window_start = bytepos;
    move_marker(bm, *buffer_id, *start_marker_id, bytepos);
}

pub fn set_window_point_with_marker(bm: &mut BufferManager, window: &mut Window, bytepos: usize) {
    let Window::Leaf {
        buffer_id,
        point,
        point_marker_id,
        ..
    } = window
    else {
        return;
    };
    *point = bytepos;
    move_marker(bm, *buffer_id, *point_marker_id, bytepos);
}

pub fn set_window_old_point_with_marker(
    bm: &mut BufferManager,
    window: &mut Window,
    bytepos: usize,
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
    *old_point = bytepos;
    move_marker(bm, *buffer_id, *old_point_marker_id, bytepos);
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
        if let Some(pos) = bm.marker_position(*buffer_id, mid) {
            *window_start = pos;
        }
    }
    if let Some(mid) = *point_marker_id {
        if let Some(pos) = bm.marker_position(*buffer_id, mid) {
            *point = pos;
        }
    }
    if let Some(mid) = *old_point_marker_id {
        if let Some(pos) = bm.marker_position(*buffer_id, mid) {
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
