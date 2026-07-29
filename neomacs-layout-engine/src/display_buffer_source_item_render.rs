//! Buffer source item rendering orchestration.

use crate::display_buffer_source_char_render::BufferSourceCharRenderRequest;
use crate::display_buffer_source_face_resolution::{
    BufferSourceItemLayoutResolutionContext, DisplaySourceNobreakHint,
};
use crate::display_buffer_source_item_append::BufferSourceRowAppendContext;
use crate::display_buffer_source_loop_context::BufferSourceLoopRequestContext;
use crate::display_buffer_source_loop_state::BufferSourceLoopMutableState;
use crate::display_buffer_source_row_lifecycle::{
    BufferSourceLineBreakRenderRequest, BufferSourceSelectiveDisplayTailRenderOutcome,
    BufferSourceSelectiveDisplayTailRenderRequest,
};
use crate::display_buffer_source_text_run::BufferSourceTextRunRenderRequest;
use crate::display_buffer_source_walk::BufferSourceWalk;
use crate::display_face_ref::render_face_ref_id;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_face_state::DisplayRowActiveFaceState;
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowLimit, DisplayRowVisibilityLimit,
};
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::display_source::DisplaySourceStepChar;
use crate::display_source::DisplaySourceStepItem;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::BufferId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSourceItemRenderOutcome {
    Rendered,
    ContinueBufferWalk,
    Stop,
}

impl BufferSourceItemRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceItemRenderRequest<'a> {
    layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
    loop_context: BufferSourceLoopRequestContext,
    text: &'a [u8],
    text_start_byte: usize,
    buffer_id: BufferId,
    append_surface: &'a DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    active_face_state: &'a DisplayRowActiveFaceState,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    char_h: f32,
    point_charpos: i64,
    row_visibility_limit: DisplayRowVisibilityLimit,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
    frame_background: Color,
}

impl<'a> BufferSourceItemRenderRequest<'a> {
    pub(crate) fn from_loop_context(
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
        loop_context: BufferSourceLoopRequestContext,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
    ) -> Self {
        Self::new(
            layout_resolution_context,
            loop_context,
            text,
            loop_context.text_start_byte(),
            loop_context.buffer_id(),
            append_surface,
            overlay_context,
            active_face_state,
            params,
            0.0,
            loop_context.char_height(),
            loop_context.point_charpos(),
            loop_context.row_visibility_limit(),
            loop_context.content_x(),
            loop_context.has_prefix(),
            loop_context.row_geometry_defaults(),
            loop_context.display_text_row_base(),
            loop_context.max_rows(),
            loop_context.row_limit(),
            loop_context.frame_background(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
        loop_context: BufferSourceLoopRequestContext,
        text: &'a [u8],
        text_start_byte: usize,
        buffer_id: BufferId,
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        char_h: f32,
        point_charpos: i64,
        row_visibility_limit: DisplayRowVisibilityLimit,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
        frame_background: Color,
    ) -> Self {
        Self {
            layout_resolution_context,
            loop_context,
            text,
            text_start_byte,
            buffer_id,
            append_surface,
            overlay_context,
            active_face_state,
            params,
            glyph_y_offset,
            char_h,
            point_charpos,
            row_visibility_limit,
            content_x,
            has_prefix,
            row_geometry_defaults,
            display_text_row_base,
            max_rows,
            row_limit,
            frame_background,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        mut self,
        source_item: DisplaySourceStepItem,
        source_walk: &mut BufferSourceWalk<'_, B>,
        buffer: &B,
        state: BufferSourceLoopMutableState<'_, '_, '_>,
    ) -> bool {
        let source_step_char = source_item.source_step_char();
        let mut state = state;
        let selective_display_outcome = self.render_selective_display_tail_for_context(
            &mut state,
            source_walk,
            source_step_char,
            buffer,
        );
        if selective_display_outcome.should_break() {
            return false;
        }
        if selective_display_outcome.should_continue_buffer_walk() {
            return true;
        }

        let is_explicit_line_break = source_item.is_explicit_line_break();
        let end_byte_idx = source_item.source_end_byte_idx();
        if is_explicit_line_break {
            if let Some(end_byte_idx) = end_byte_idx {
                state.progress.set_byte_idx(end_byte_idx);
            }
            if self
                .render_line_break_for_context(&mut state, source_walk, source_step_char, buffer)
                .should_break()
            {
                return false;
            }
            return true;
        }

        let outcome = self.render_text_item_and_apply(source_item, source_walk, buffer, state);
        !outcome.should_break()
    }

    fn render_text_item_and_apply<B: LayoutBufferView>(
        self,
        source_item: DisplaySourceStepItem,
        source_walk: &mut BufferSourceWalk<'_, B>,
        buffer: &B,
        state: BufferSourceLoopMutableState<'_, '_, '_>,
    ) -> BufferSourceItemRenderOutcome {
        debug_assert_ne!(source_item.source_step_char().ch(), '\n');
        self.render_prepared_source_item_and_apply(source_item, source_walk, buffer, state)
    }

    fn render_selective_display_tail_for_context<B: LayoutBufferView>(
        &mut self,
        state: &mut BufferSourceLoopMutableState<'_, '_, '_>,
        source_walk: &mut BufferSourceWalk<'_, B>,
        source_step_char: DisplaySourceStepChar,
        buffer: &B,
    ) -> BufferSourceSelectiveDisplayTailRenderOutcome {
        let request = self.loop_context.selective_display_tail_request(
            source_step_char,
            self.text,
            state.append_surface,
            self.active_face_state,
            self.glyph_y_offset,
        );
        self.render_selective_display_tail(state, source_walk, request, buffer)
    }

    fn render_selective_display_tail<B: LayoutBufferView>(
        &mut self,
        state: &mut BufferSourceLoopMutableState<'_, '_, '_>,
        source_walk: &mut BufferSourceWalk<'_, B>,
        request: BufferSourceSelectiveDisplayTailRenderRequest<'_>,
        buffer: &B,
    ) -> BufferSourceSelectiveDisplayTailRenderOutcome {
        request.render_if_needed_and_apply(source_walk, buffer, state.reborrow())
    }

    fn render_line_break_for_context<B: LayoutBufferView>(
        &mut self,
        state: &mut BufferSourceLoopMutableState<'_, '_, '_>,
        source_walk: &mut BufferSourceWalk<'_, B>,
        source_char: DisplaySourceStepChar,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation {
        let request = self.loop_context.line_break_request(
            source_char,
            self.text,
            self.append_surface,
            state.overlay_context,
            self.active_face_state,
        );
        self.render_line_break(state, source_walk, request, buffer)
    }

    fn render_line_break<B: LayoutBufferView>(
        &mut self,
        state: &mut BufferSourceLoopMutableState<'_, '_, '_>,
        source_walk: &mut BufferSourceWalk<'_, B>,
        request: BufferSourceLineBreakRenderRequest<'_>,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation {
        request.render_and_apply(source_walk, buffer, state.reborrow())
    }

    fn render_prepared_source_item_and_apply<B: LayoutBufferView>(
        self,
        mut source_item: DisplaySourceStepItem,
        source_walk: &mut BufferSourceWalk<'_, B>,
        buffer: &B,
        state: BufferSourceLoopMutableState<'_, '_, '_>,
    ) -> BufferSourceItemRenderOutcome {
        let BufferSourceLoopMutableState {
            invisible_text_checkpoint,
            mut progress,
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
        } = state;
        let mut source_render = source_render;
        // The unsubstituted buffer char + active nobreak policy, used by the
        // nbsp / nobreak-hyphen highlight branch in face resolution. Captured
        // before the mutable `item_mut()` borrow below.
        let nobreak_hint = DisplaySourceNobreakHint::new(
            source_item.source_step_char().ch(),
            self.params.nobreak_char_display,
        );
        let active_face_state = self
            .layout_resolution_context
            .resolve_source_item_layout_for_active_face(
                &mut source_render,
                face_ids,
                row_geometry,
                self.active_face_state,
                source_item.item_mut(),
                nobreak_hint,
            );
        let item_face_id = render_face_ref_id(source_item.item().face, active_face_state.face_id());
        if item_face_id == active_face_state.face_id() {
            // Layout-only face transforms (height, escape-glyph and nobreak
            // highlighting) happen after source-property resolution. Preserve
            // their complete identity here before this item can be split and
            // its suffix queued for a later row/iteration.
            source_walk.remember_resolved_source_face_if_absent(
                active_face_state.face_id(),
                active_face_state.resolved_face(),
            );
        }
        let resolved_item_face = source_walk
            .resolved_source_face(item_face_id)
            .cloned()
            .map(|face| (item_face_id, face));
        let row_extend_fill = resolved_item_face
            .as_ref()
            .and_then(|(face_id, face)| face.extend.then(|| (Color::from_pixel(face.bg), *face_id)))
            .or_else(|| active_face_state.row_extend_fill());
        if let Some(fill) = row_extend_fill {
            row_extend.activate(row_geometry.current_row_marker(), fill);
        } else {
            row_extend.clear();
        }
        let mut buffer_row_append_context = BufferSourceRowAppendContext::from_active_face_row(
            buffer,
            self.buffer_id,
            self.append_surface,
            &active_face_state,
            self.glyph_y_offset,
            self.char_h,
            face_ids.clone(),
        );
        if let Some((face_id, face)) = resolved_item_face {
            buffer_row_append_context =
                buffer_row_append_context.with_resolved_item_face(face_id, face);
        }
        let append_position = progress.row_position();
        let append_geometry = *row_geometry;
        let text_run_request = BufferSourceTextRunRenderRequest::new(
            self.text_start_byte,
            self.overlay_context,
            self.point_charpos,
            self.append_surface.right_edge(),
            append_position,
            append_geometry,
        );

        if let Some((prefix, suffix)) =
            text_run_request.split_at_first_overlay(&source_item, buffer)
        {
            source_walk.prepend_pending_render_items(vec![suffix]);
            source_item = prefix;
        }

        if let Some(outcome) = text_run_request.render_if_fits_and_apply(
            source_item.clone(),
            buffer,
            &active_face_state,
            &buffer_row_append_context,
            cursor_info,
            trailing_whitespace,
            word_wrap,
            &mut source_render,
            &mut progress,
        ) {
            return outcome;
        }

        if let Some((prefix, suffix)) = text_run_request.split_prefix_to_fit(
            &source_item,
            buffer,
            self.params.wrap_mode,
            &buffer_row_append_context,
            &mut source_render,
        ) {
            source_walk.prepend_pending_render_items(vec![suffix]);
            return text_run_request.render_and_apply(
                prefix,
                &active_face_state,
                &buffer_row_append_context,
                cursor_info,
                trailing_whitespace,
                word_wrap,
                &mut source_render,
                &mut progress,
            );
        }

        BufferSourceCharRenderRequest::new(
            self.text,
            self.text_start_byte,
            self.append_surface,
            self.overlay_context,
            self.params,
            self.point_charpos,
            self.row_visibility_limit,
            self.content_x,
            self.has_prefix,
            self.row_geometry_defaults,
            self.display_text_row_base,
            self.max_rows,
            self.row_limit,
            self.frame_background,
        )
        .render_and_apply(
            source_item,
            source_walk,
            buffer,
            &active_face_state,
            &buffer_row_append_context,
            BufferSourceLoopMutableState::new(
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
        )
    }
}
