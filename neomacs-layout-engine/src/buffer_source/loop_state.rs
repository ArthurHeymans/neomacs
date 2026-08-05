//! Shared mutable state for buffer text visible-loop rendering.

use crate::display_cursor::CursorCaptureState;
use crate::display_row::append_context::DisplayRowAppendSurface;
use crate::display_row::geometry::{
    DisplayRowFlags, DisplayRowGeometryState, DisplayRowScopedValue, DisplayRowYPositions,
};
use crate::display_row::lisp_string::DisplayRowPrefixRequest;
use crate::display_row::overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row::source_render::TextRowSourceRenderState;
use crate::display_row::walk_state::{
    BoxFaceRowState, FaceScanCheckpoint, HitRowRangeTracker, HorizontalScrollSkipState,
    InvisibleTextScanCheckpoint, LineNumberRenderState, TrailingWhitespaceRenderState,
    WordWrapRenderState,
};
use crate::display_source_progress::DisplaySourceProgressState;
use crate::frame_face_arena::FrameFaceAttempt;
use crate::hit_test::HitRow;
use neomacs_display_protocol::types::Color;
use neomacs_display_protocol::types::FaceId;

pub(crate) struct BufferSourceLoopMutableState<'rows, 'emit, 'surface> {
    pub(crate) invisible_text_checkpoint: &'emit mut InvisibleTextScanCheckpoint,
    pub(crate) progress: DisplaySourceProgressState<'emit>,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, FaceId)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'rows mut DisplayRowYPositions,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) face_ids: &'emit mut FrameFaceAttempt,
    pub(crate) append_surface: &'surface DisplayRowAppendSurface,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
}

impl<'rows, 'emit, 'surface> BufferSourceLoopMutableState<'rows, 'emit, 'surface> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        invisible_text_checkpoint: &'emit mut InvisibleTextScanCheckpoint,
        progress: DisplaySourceProgressState<'emit>,
        source_render: TextRowSourceRenderState<'emit>,
        row_extend: &'emit mut DisplayRowScopedValue<(Color, FaceId)>,
        box_face: &'emit mut BoxFaceRowState,
        line_numbers: &'emit mut LineNumberRenderState,
        row_geometry: &'emit mut DisplayRowGeometryState,
        row_flags: &'emit mut DisplayRowFlags,
        hit_rows: &'emit mut Vec<HitRow>,
        hit_row_range: &'emit mut HitRowRangeTracker,
        prefix_request: &'emit mut DisplayRowPrefixRequest,
        hscroll_skip: &'emit mut HorizontalScrollSkipState,
        word_wrap: &'emit mut WordWrapRenderState,
        trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
        face_scan: &'emit mut FaceScanCheckpoint,
        row_y_positions: &'rows mut DisplayRowYPositions,
        cursor_info: &'emit mut CursorCaptureState,
        face_ids: &'emit mut FrameFaceAttempt,
        append_surface: &'surface DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
    ) -> Self {
        Self {
            invisible_text_checkpoint,
            progress,
            source_render,
            row_extend,
            box_face,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            face_scan,
            row_y_positions,
            cursor_info,
            face_ids,
            append_surface,
            overlay_context,
        }
    }

    pub(crate) fn reborrow(&mut self) -> BufferSourceLoopMutableState<'_, '_, 'surface> {
        BufferSourceLoopMutableState {
            invisible_text_checkpoint: self.invisible_text_checkpoint,
            progress: self.progress.reborrow(),
            source_render: self.source_render.reborrow(),
            row_extend: self.row_extend,
            box_face: self.box_face,
            line_numbers: self.line_numbers,
            row_geometry: self.row_geometry,
            row_flags: self.row_flags,
            hit_rows: self.hit_rows,
            hit_row_range: self.hit_row_range,
            prefix_request: self.prefix_request,
            hscroll_skip: self.hscroll_skip,
            word_wrap: self.word_wrap,
            trailing_whitespace: self.trailing_whitespace,
            face_scan: self.face_scan,
            row_y_positions: self.row_y_positions,
            cursor_info: self.cursor_info,
            face_ids: self.face_ids,
            append_surface: self.append_surface,
            overlay_context: self.overlay_context,
        }
    }
}
