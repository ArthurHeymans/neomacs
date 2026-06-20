//! Buffer text visible-loop rendering.

use crate::display_buffer_text_face_resolution::*;
use crate::display_buffer_text_item_append::BufferTextRowAppendState;
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_loop_step_render::BufferTextWindowLoopStepRenderState;
use crate::display_buffer_text_progress::BufferTextWindowProgressState;
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source_walk::*;
use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryState, DisplayRowScopedValue, DisplayRowYPositions,
};
use crate::display_row_lisp_string::DisplayRowPrefixRequest;
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{
    BoxFaceRowState, FaceScanCheckpoint, HitRowRangeTracker, HorizontalScrollSkipState,
    InvisibleTextScanCheckpoint, LineNumberRenderState, TrailingWhitespaceRenderState,
    WordWrapRenderState,
};
use crate::hit_test::HitRow;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neomacs_display_protocol::types::Color;

pub(crate) struct BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
}

impl<'rows, 'emit, 'surface> BufferTextWindowLoopRenderState<'rows, 'emit, 'surface> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        append_state: &'emit mut BufferTextRowAppendState,
        invisible_text_checkpoint: &'emit mut InvisibleTextScanCheckpoint,
        progress: BufferTextWindowProgressState<'emit>,
        source_render: TextRowSourceRenderState<'emit>,
        row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
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
        face_ids: &'emit mut FrameFaceIdAllocator,
        append_surface: &'surface DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'surface>,
    ) -> Self {
        Self {
            loop_context,
            state: BufferTextWindowLoopMutableState::new(
                append_state,
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
            ),
        }
    }

    pub(crate) fn render_visible_steps<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) where
        'surface: 'request,
    {
        while *self.state.progress.byte_idx < text.len()
            && self
                .state
                .row_geometry
                .current_row_is_visible(self.loop_context.row_visibility_limit())
        {
            if !BufferTextWindowLoopStepRenderState::new(self.loop_context, self.state.reborrow())
                .render_next(
                    source_walk,
                    row_prelude_context,
                    face_resolution_context.clone(),
                    text,
                    params,
                    active_face_state,
                    buffer,
                )
            {
                break;
            }
        }
    }
}
