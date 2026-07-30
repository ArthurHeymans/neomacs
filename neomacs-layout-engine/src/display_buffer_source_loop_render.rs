//! Buffer text visible-loop rendering.

use crate::display_buffer_source_face_resolution::*;
use crate::display_buffer_source_loop_context::BufferSourceLoopRequestContext;
use crate::display_buffer_source_loop_state::BufferSourceLoopMutableState;
use crate::display_buffer_source_render::BufferSourceRenderRequest;
use crate::display_buffer_source_row_lifecycle::{
    BufferSourceHscrollSkipRenderContext, BufferSourceInvisibleTextRenderContext,
    BufferSourceInvisibleTextRenderOutcome,
};
use crate::display_buffer_source_row_prelude::BufferSourceRowPreludeRequestContext;
use crate::display_buffer_source_walk::*;
use crate::display_row_face_state::DisplayRowActiveFaceState;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

impl<'rows, 'emit, 'surface> BufferSourceLoopMutableState<'rows, 'emit, 'surface> {
    pub(crate) fn render_visible_steps<'request, B: LayoutBufferView>(
        &mut self,
        loop_context: BufferSourceLoopRequestContext,
        source_walk: &mut BufferSourceWalk<'request, B>,
        row_prelude_context: BufferSourceRowPreludeRequestContext,
        face_resolution_context: BufferSourceFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) where
        'surface: 'request,
    {
        while self.progress.byte_idx() < text.len()
            && self
                .row_geometry
                .current_row_is_visible(loop_context.row_visibility_limit())
        {
            self.render_row_prelude(row_prelude_context, params, active_face_state, buffer);

            let invisible_text_outcome = self.render_invisible_text_for_context(
                loop_context,
                source_walk,
                text,
                active_face_state,
                buffer,
            );
            if invisible_text_outcome.should_break() {
                break;
            }
            if invisible_text_outcome.should_continue_buffer_walk() {
                continue;
            }

            if self.hscroll_skip.should_skip() {
                if self
                    .render_hscroll_skip_for_context(
                        loop_context,
                        source_walk,
                        text,
                        active_face_state,
                    )
                    .should_break()
                {
                    break;
                }
                continue;
            }

            self.render_face_checkpoint_for_context(face_resolution_context, active_face_state);

            if !BufferSourceRenderRequest::new(
                loop_context,
                text,
                params,
                active_face_state,
                self.reborrow(),
            )
            .render_next_and_apply(source_walk, face_resolution_context, buffer)
            {
                break;
            }
        }
    }

    fn render_row_prelude<B: LayoutBufferView>(
        &mut self,
        context: BufferSourceRowPreludeRequestContext,
        params: &WindowParams,
        active_face_state: &DisplayRowActiveFaceState,
        buffer: &B,
    ) {
        context
            .line_number_margin_request()
            .render_pending_with_source_state(
                self.line_numbers,
                &mut self.source_render,
                self.face_ids,
                self.row_geometry,
                self.face_scan,
                context.char_width(),
            );

        let row_position = self.progress.row_position();
        let charpos = self.progress.charpos();
        let (x, col) = self.progress.row_progress_mut().coordinates_mut();
        context
            .line_prefix_request(
                self.append_surface,
                self.row_geometry,
                active_face_state,
                0.0,
                row_position,
                params,
            )
            .render_requested_with_source_state_and_apply(
                self.prefix_request,
                &mut self.source_render,
                buffer,
                charpos,
                self.face_ids,
                x,
                col,
            );
    }

    fn render_invisible_text_for_context<'request, B: LayoutBufferView>(
        &mut self,
        loop_context: BufferSourceLoopRequestContext,
        source_walk: &mut BufferSourceWalk<'_, B>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferSourceInvisibleTextRenderOutcome
    where
        'surface: 'request,
    {
        let request = loop_context.invisible_text_request(
            text,
            self.append_surface,
            self.overlay_context,
            active_face_state,
            0.0,
        );
        self.render_invisible_text_at_checkpoint(source_walk, request, buffer)
    }

    fn render_invisible_text_at_checkpoint<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'_, B>,
        request: BufferSourceInvisibleTextRenderContext<'_>,
        buffer: &B,
    ) -> BufferSourceInvisibleTextRenderOutcome {
        request.render_at_checkpoint_and_apply(source_walk, buffer, self.reborrow())
    }

    fn render_hscroll_skip_for_context<'request, B: LayoutBufferView>(
        &mut self,
        loop_context: BufferSourceLoopRequestContext,
        source_walk: &mut BufferSourceWalk<'_, B>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
    ) -> DisplayRowTransitionContinuation
    where
        'surface: 'request,
    {
        let request =
            loop_context.hscroll_skip_request(text, self.append_surface, active_face_state);
        self.render_hscroll_skip(source_walk, request)
    }

    fn render_hscroll_skip<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'_, B>,
        request: BufferSourceHscrollSkipRenderContext<'_>,
    ) -> DisplayRowTransitionContinuation {
        request.render_next_and_apply(source_walk, self.reborrow())
    }

    fn render_face_checkpoint_for_context<B: LayoutBufferView>(
        &mut self,
        face_resolution_context: BufferSourceFaceResolutionContext<'_, B>,
        active_face_state: &mut DisplayRowActiveFaceState,
    ) {
        face_resolution_context.resolve_at_checkpoint_with_source_state(
            &mut self.source_render.reborrow(),
            self.face_scan,
            self.face_ids,
            active_face_state,
            self.row_geometry,
            self.row_extend,
            self.box_face,
            self.progress.row_progress().x(),
            self.progress.charpos(),
        );
    }
}
