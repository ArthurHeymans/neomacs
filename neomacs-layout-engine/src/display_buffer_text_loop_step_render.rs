//! Buffer text visible-loop single-step rendering.

use crate::display_buffer_text_consumed_render::BufferTextWindowConsumedRenderState;
use crate::display_buffer_text_face_resolution::BufferCurrentFaceResolutionContext;
use crate::display_buffer_text_item_append::BufferTextRowAppendState;
use crate::display_buffer_text_loop_context::{
    BufferTextWindowConsumedDisplayItemRenderRequest, BufferTextWindowLoopRequestContext,
};
use crate::display_buffer_text_pre_source_render::{
    BufferTextWindowPreSourceOutcome, BufferTextWindowPreSourceRenderState,
};
use crate::display_buffer_text_progress::BufferTextWindowProgressState;
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source_render::{
    BufferTextWindowSourceRenderOutcome, BufferTextWindowSourceRenderRequest,
};
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowLoopStepOutcome {
    ContinueBufferWalk,
    StopBufferWalk,
}

pub(crate) struct BufferTextWindowLoopStepRenderState<'rows, 'emit, 'surface> {
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
}

impl BufferTextWindowLoopStepOutcome {
    pub(crate) fn should_stop_buffer_walk(self) -> bool {
        matches!(self, Self::StopBufferWalk)
    }
}

impl<'rows, 'emit, 'surface> BufferTextWindowLoopStepRenderState<'rows, 'emit, 'surface> {
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
        }
    }

    pub(crate) fn render_next<'request, B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferTextWindowLoopStepOutcome
    where
        'surface: 'request,
    {
        let pre_source_outcome = BufferTextWindowPreSourceRenderState::new(
            self.loop_context,
            self.invisible_text_checkpoint,
            self.progress.reborrow(),
            self.source_render.reborrow(),
            self.row_extend,
            self.box_face,
            self.line_numbers,
            self.row_geometry,
            self.row_flags,
            self.hit_rows,
            self.hit_row_range,
            self.prefix_request,
            self.hscroll_skip,
            self.word_wrap,
            self.trailing_whitespace,
            self.face_scan,
            self.row_y_positions,
            self.cursor_info,
            self.face_ids,
            self.append_surface,
            self.overlay_context,
        )
        .render_for_context(
            source_walk,
            row_prelude_context,
            face_resolution_context.clone(),
            text,
            active_face_state,
            buffer,
        );
        match pre_source_outcome {
            BufferTextWindowPreSourceOutcome::ReadyForSourceItem => {}
            BufferTextWindowPreSourceOutcome::ContinueBufferWalk => {
                return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
            }
            BufferTextWindowPreSourceOutcome::StopBufferWalk => {
                return BufferTextWindowLoopStepOutcome::StopBufferWalk;
            }
        }

        let source_outcome = BufferTextWindowSourceRenderRequest::new(
            self.loop_context,
            text,
            params,
            active_face_state,
            self.source_render.reborrow(),
            self.face_ids,
            self.append_surface,
            self.row_geometry,
            self.cursor_info,
            self.progress.reborrow(),
        )
        .consume_next(source_walk, face_resolution_context.clone(), buffer);

        if source_outcome.should_continue_buffer_walk() {
            return BufferTextWindowLoopStepOutcome::ContinueBufferWalk;
        }
        if source_outcome.should_stop_buffer_walk() {
            return BufferTextWindowLoopStepOutcome::StopBufferWalk;
        }
        let BufferTextWindowSourceRenderOutcome::DisplayItem(source_item) = source_outcome else {
            unreachable!("source render stop/continue outcomes handled above");
        };
        let consumed_outcome = BufferTextWindowConsumedRenderState::new(
            self.loop_context,
            self.append_state,
            self.progress.reborrow(),
            self.source_render.reborrow(),
            self.row_extend,
            self.box_face,
            self.line_numbers,
            self.row_geometry,
            self.row_flags,
            self.hit_rows,
            self.hit_row_range,
            self.prefix_request,
            self.hscroll_skip,
            self.word_wrap,
            self.trailing_whitespace,
            self.face_scan,
            self.row_y_positions,
            self.cursor_info,
            self.face_ids,
            self.append_surface,
            self.overlay_context,
        )
        .render_for_context(
            source_walk,
            BufferTextWindowConsumedDisplayItemRenderRequest {
                layout_resolution_context: face_resolution_context
                    .source_item_layout_resolution_context(),
                source_item,
                text,
                active_face_state,
                params,
            },
            buffer,
        );
        if consumed_outcome.should_stop_buffer_walk() {
            BufferTextWindowLoopStepOutcome::StopBufferWalk
        } else {
            BufferTextWindowLoopStepOutcome::ContinueBufferWalk
        }
    }
}
