//! Buffer text visible-loop single-step rendering.

use crate::display_buffer_text_consumed_render::BufferTextWindowConsumedRenderState;
use crate::display_buffer_text_face_resolution::BufferCurrentFaceResolutionContext;
use crate::display_buffer_text_loop_context::{
    BufferTextWindowConsumedDisplayItemRenderRequest, BufferTextWindowLoopRequestContext,
};
use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_pre_source_render::{
    BufferTextWindowPreSourceOutcome, BufferTextWindowPreSourceRenderState,
};
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source_render::{
    BufferTextWindowSourceRenderOutcome, BufferTextWindowSourceRenderRequest,
};
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_row::DisplayRowActiveFaceState;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextWindowLoopStepOutcome {
    ContinueBufferWalk,
    StopBufferWalk,
}

pub(crate) struct BufferTextWindowLoopStepRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
}

impl BufferTextWindowLoopStepOutcome {
    pub(crate) fn should_stop_buffer_walk(self) -> bool {
        matches!(self, Self::StopBufferWalk)
    }
}

impl<'rows, 'emit, 'surface> BufferTextWindowLoopStepRenderState<'rows, 'emit, 'surface> {
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
    ) -> Self {
        Self {
            loop_context,
            state,
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
            self.state.invisible_text_checkpoint,
            self.state.progress.reborrow(),
            self.state.source_render.reborrow(),
            self.state.row_extend,
            self.state.box_face,
            self.state.line_numbers,
            self.state.row_geometry,
            self.state.row_flags,
            self.state.hit_rows,
            self.state.hit_row_range,
            self.state.prefix_request,
            self.state.hscroll_skip,
            self.state.word_wrap,
            self.state.trailing_whitespace,
            self.state.face_scan,
            self.state.row_y_positions,
            self.state.cursor_info,
            self.state.face_ids,
            self.state.append_surface,
            self.state.overlay_context,
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
            self.state.source_render.reborrow(),
            self.state.face_ids,
            self.state.append_surface,
            self.state.row_geometry,
            self.state.cursor_info,
            self.state.progress.reborrow(),
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
            self.state.append_state,
            self.state.progress.reborrow(),
            self.state.source_render.reborrow(),
            self.state.row_extend,
            self.state.box_face,
            self.state.line_numbers,
            self.state.row_geometry,
            self.state.row_flags,
            self.state.hit_rows,
            self.state.hit_row_range,
            self.state.prefix_request,
            self.state.hscroll_skip,
            self.state.word_wrap,
            self.state.trailing_whitespace,
            self.state.face_scan,
            self.state.row_y_positions,
            self.state.cursor_info,
            self.state.face_ids,
            self.state.append_surface,
            self.state.overlay_context,
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
