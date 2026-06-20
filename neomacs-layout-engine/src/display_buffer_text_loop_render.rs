//! Buffer text visible-loop rendering.

use crate::display_buffer_source_render::BufferSourceRenderRequest;
use crate::display_buffer_source_walk::*;
use crate::display_buffer_text_face_resolution::*;
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_loop_state::BufferTextWindowLoopMutableState;
use crate::display_buffer_text_row_lifecycle::{
    BufferHscrollSkipRenderRequest, BufferInvisibleTextRenderOutcome,
    BufferInvisibleTextRenderRequest,
};
use crate::display_buffer_text_row_prelude::BufferTextWindowRowPreludeRequestContext;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

impl<'rows, 'emit, 'surface> BufferTextWindowLoopMutableState<'rows, 'emit, 'surface> {
    pub(crate) fn render_visible_steps<'request, B: LayoutBufferView>(
        &mut self,
        loop_context: BufferTextWindowLoopRequestContext,
        source_walk: &mut BufferSourceWalk<'request, B>,
        row_prelude_context: BufferTextWindowRowPreludeRequestContext,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &mut DisplayRowActiveFaceState,
        buffer: &B,
    ) where
        'surface: 'request,
    {
        while *self.progress.byte_idx < text.len()
            && self
                .row_geometry
                .current_row_is_visible(loop_context.row_visibility_limit())
        {
            self.render_row_prelude(row_prelude_context, active_face_state, buffer);

            if self
                .render_invisible_text_for_context(
                    loop_context,
                    source_walk,
                    text,
                    active_face_state,
                    buffer,
                )
                .should_continue_buffer_walk()
            {
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

            self.render_face_checkpoint_for_context(
                face_resolution_context.clone(),
                active_face_state,
            );

            if !BufferSourceRenderRequest::new(
                loop_context,
                text,
                params,
                active_face_state,
                self.reborrow(),
            )
            .render_next_and_apply(
                source_walk,
                face_resolution_context.clone(),
                buffer,
            ) {
                break;
            }
        }
    }

    fn render_row_prelude<B: LayoutBufferView>(
        &mut self,
        context: BufferTextWindowRowPreludeRequestContext,
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

        context
            .line_prefix_request(
                self.append_surface,
                self.row_geometry,
                active_face_state,
                0.0,
                self.progress.row_position(),
            )
            .render_requested_with_source_state_and_apply(
                self.prefix_request,
                &mut self.source_render,
                buffer,
                self.progress.charpos(),
                self.face_ids,
                self.progress.row.x,
                self.progress.row.col,
            );
    }

    fn render_invisible_text_for_context<'request, B: LayoutBufferView>(
        &mut self,
        loop_context: BufferTextWindowLoopRequestContext,
        source_walk: &mut BufferSourceWalk<'_, B>,
        text: &'request [u8],
        active_face_state: &'request DisplayRowActiveFaceState,
        buffer: &B,
    ) -> BufferInvisibleTextRenderOutcome
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
        request: BufferInvisibleTextRenderRequest<'_>,
        buffer: &B,
    ) -> BufferInvisibleTextRenderOutcome {
        request.render_at_checkpoint_and_apply(source_walk, buffer, self.reborrow())
    }

    fn render_hscroll_skip_for_context<'request, B: LayoutBufferView>(
        &mut self,
        loop_context: BufferTextWindowLoopRequestContext,
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
        request: BufferHscrollSkipRenderRequest<'_>,
    ) -> DisplayRowTransitionContinuation {
        request.render_next_and_apply(source_walk, self.reborrow())
    }

    fn render_face_checkpoint_for_context<B: LayoutBufferView>(
        &mut self,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
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
            *self.progress.row.x,
            self.progress.charpos(),
        );
    }
}
