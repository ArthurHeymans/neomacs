//! Buffer text visible-loop single-step rendering.

use crate::display_buffer_text_face_resolution::BufferCurrentFaceResolutionContext;
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_pre_source_render::{
    BufferTextWindowPreSourceOutcome, BufferTextWindowPreSourceRenderState,
};
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_buffer_text_source_render::BufferTextWindowSourceRenderRequest;
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_row::DisplayRowActiveFaceState;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

pub(crate) struct BufferTextWindowLoopStepRenderState<'rows, 'emit, 'surface> {
    loop_context: BufferTextWindowLoopRequestContext,
    state: BufferTextWindowLoopMutableState<'rows, 'emit, 'surface>,
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
    ) -> bool
    where
        'surface: 'request,
    {
        let pre_source_outcome =
            BufferTextWindowPreSourceRenderState::new(self.loop_context, self.state.reborrow())
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
                return true;
            }
            BufferTextWindowPreSourceOutcome::StopBufferWalk => {
                return false;
            }
        }

        let continue_buffer_walk = BufferTextWindowSourceRenderRequest::new(
            self.loop_context,
            text,
            params,
            active_face_state,
            self.state.reborrow(),
        )
        .render_next_and_apply(source_walk, face_resolution_context.clone(), buffer);

        if !continue_buffer_walk {
            return false;
        }
        true
    }
}
