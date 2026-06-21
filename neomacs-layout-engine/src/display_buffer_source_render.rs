//! Buffer text source consumption with replacement application.

use crate::display_buffer_display_property_render::{
    BufferDisplayPropertyTextReplacementRenderContext,
    BufferDisplayPropertyTextReplacementRenderOutcome,
    BufferDisplayPropertyTextReplacementRenderState,
};
use crate::display_buffer_source_char_render::BufferSourceCharRenderRequest;
use crate::display_buffer_source_consumption::BufferSourceConsumedItem;
use crate::display_buffer_source_face_resolution::BufferSourceFaceResolutionContext;
use crate::display_buffer_source_face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::display_buffer_source_item_append::BufferSourceRowAppendContext;
use crate::display_buffer_source_loop_context::BufferSourceLoopRequestContext;
use crate::display_buffer_source_loop_state::BufferSourceLoopMutableState;
use crate::display_buffer_source_row_lifecycle::{
    BufferSourceLineBreakRenderRequest, BufferSourceSelectiveDisplayTailRenderOutcome,
    BufferSourceSelectiveDisplayTailRenderRequest,
};
use crate::display_buffer_source_text_run::BufferSourceTextRunRenderRequest;
use crate::display_buffer_source_walk::BufferSourceWalk;
use crate::display_item::BufferDisplayPropertyReplacementItem;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowLimit, DisplayRowVisibilityLimit,
};
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_transition::DisplayRowTransitionContinuation;
use crate::display_source::DisplaySourceStepChar;
use crate::display_source::DisplaySourceStepItem;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neovm_core::buffer::BufferId;

#[derive(Clone, Copy)]
struct BufferSourceItemRenderContext<'a> {
    layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
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
}

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

pub(crate) struct BufferSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face> {
    loop_context: BufferSourceLoopRequestContext,
    text: &'request [u8],
    params: &'request WindowParams,
    active_face_state: &'face DisplayRowActiveFaceState,
    state: BufferSourceLoopMutableState<'rows, 'emit, 'surface>,
}

impl<'a> BufferSourceItemRenderContext<'a> {
    fn from_loop_context(
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
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
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
    ) -> Self {
        Self {
            layout_resolution_context,
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
        }
    }
}

fn render_source_item_and_apply<B: LayoutBufferView>(
    source_item: DisplaySourceStepItem,
    context: BufferSourceItemRenderContext<'_>,
    source_walk: &mut BufferSourceWalk<'_, B>,
    buffer: &B,
    state: BufferSourceLoopMutableState<'_, '_, '_>,
) -> BufferSourceItemRenderOutcome {
    debug_assert_ne!(source_item.source_step_char().ch(), '\n');
    render_prepared_source_item_and_apply(source_item, context, source_walk, buffer, state)
}

fn render_prepared_source_item_and_apply<B: LayoutBufferView>(
    mut source_item: DisplaySourceStepItem,
    context: BufferSourceItemRenderContext<'_>,
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
    let active_face_state = context
        .layout_resolution_context
        .resolve_source_item_layout_for_active_face(
            &mut source_render,
            face_ids,
            row_geometry,
            context.active_face_state,
            source_item.item_mut(),
        );
    let buffer_row_append_context = BufferSourceRowAppendContext::from_active_face_row(
        buffer,
        context.buffer_id,
        context.append_surface,
        &active_face_state,
        context.glyph_y_offset,
        context.char_h,
    );
    let append_position = progress.row_position();
    let append_geometry = *row_geometry;
    let text_run_request = BufferSourceTextRunRenderRequest::new(
        context.text_start_byte,
        context.overlay_context,
        context.point_charpos,
        context.append_surface.right_edge(),
        append_position,
        append_geometry,
    );

    if let Some((prefix, suffix)) = text_run_request.split_at_first_overlay(&source_item, buffer) {
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
        context.params.wrap_mode,
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
        context.text,
        context.text_start_byte,
        context.append_surface,
        context.overlay_context,
        context.params,
        context.point_charpos,
        context.row_visibility_limit,
        context.content_x,
        context.has_prefix,
        context.row_geometry_defaults,
        context.display_text_row_base,
        context.max_rows,
        context.row_limit,
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

impl<'rows, 'request, 'emit, 'surface, 'face>
    BufferSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face>
{
    pub(crate) fn new(
        loop_context: BufferSourceLoopRequestContext,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &'face DisplayRowActiveFaceState,
        state: BufferSourceLoopMutableState<'rows, 'emit, 'surface>,
    ) -> Self {
        Self {
            loop_context,
            text,
            params,
            active_face_state,
            state,
        }
    }

    pub(crate) fn render_next_and_apply<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        face_resolution_context: BufferSourceFaceResolutionContext<'request, B>,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let layout_resolution_context =
            face_resolution_context.source_item_layout_resolution_context();
        let Some(consumed_item) = source_walk.consume_source_item_for_render(
            &mut self.state.progress,
            face_resolution_context,
            self.state.face_ids,
            &mut self.state.source_render.reborrow(),
            self.state.row_geometry,
        ) else {
            return false;
        };

        match consumed_item {
            BufferSourceConsumedItem::DisplayPropertyReplacement(replacement) => self
                .consume_replacement(source_walk, layout_resolution_context, replacement, buffer),
            BufferSourceConsumedItem::Renderable(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
        }
    }

    fn consume_replacement<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        replacement: BufferDisplayPropertyReplacementItem,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let replacement_context = BufferDisplayPropertyTextReplacementRenderContext::new(
            replacement,
            self.loop_context.text_start_byte(),
            self.text,
            self.loop_context.content_x(),
            self.params,
            0.0,
            self.loop_context.char_height(),
            self.active_face_state,
            self.state.progress.row_progress().x(),
            self.state.progress.row_position(),
        );
        match replacement_context.render(
            buffer,
            BufferDisplayPropertyTextReplacementRenderState::new(
                self.state.source_render.reborrow(),
                self.state.face_ids,
                self.state.append_surface,
                self.state.row_geometry,
                self.active_face_state,
            ),
        ) {
            BufferDisplayPropertyTextReplacementRenderOutcome::Rendered(outcome) => {
                replacement_context.apply_rendered_outcome(
                    outcome,
                    &mut self.state.progress,
                    self.state.cursor_info,
                    self.state.row_geometry,
                    self.loop_context.point_charpos(),
                );
                true
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Stop => false,
        }
    }

    fn render_source_item<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: DisplaySourceStepItem,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let source_step_char = source_item.source_step_char();
        let selective_display_outcome =
            self.render_selective_display_tail_for_context(source_walk, source_step_char, buffer);
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
                self.state.progress.set_byte_idx(end_byte_idx);
            }
            if self
                .render_line_break_for_context(source_walk, source_step_char, buffer)
                .should_break()
            {
                return false;
            }
            return true;
        }

        let char_render_outcome = self.render_text_source_item_for_context(
            source_walk,
            layout_resolution_context,
            source_item,
            buffer,
        );
        if char_render_outcome.should_break() {
            return false;
        }
        true
    }

    fn render_selective_display_tail_for_context<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        source_step_char: DisplaySourceStepChar,
        buffer: &B,
    ) -> BufferSourceSelectiveDisplayTailRenderOutcome
    where
        'surface: 'request,
    {
        let request = self.loop_context.selective_display_tail_request(
            source_step_char,
            self.text,
            self.state.append_surface,
            self.active_face_state,
            0.0,
        );
        self.render_selective_display_tail(source_walk, request, buffer)
    }

    fn render_line_break_for_context<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        source_char: DisplaySourceStepChar,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation
    where
        'surface: 'request,
    {
        let request = self.loop_context.line_break_request(
            source_char,
            self.text,
            self.state.overlay_context,
            self.active_face_state,
        );
        self.render_line_break(source_walk, request, buffer)
    }

    fn render_text_source_item_for_context<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: DisplaySourceStepItem,
        buffer: &B,
    ) -> BufferSourceItemRenderOutcome
    where
        'surface: 'request,
    {
        render_source_item_and_apply(
            source_item,
            BufferSourceItemRenderContext::from_loop_context(
                layout_resolution_context,
                self.loop_context,
                self.text,
                self.state.append_surface,
                self.state.overlay_context,
                self.active_face_state,
                self.params,
            ),
            source_walk,
            buffer,
            self.state.reborrow(),
        )
    }

    fn render_selective_display_tail<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        request: BufferSourceSelectiveDisplayTailRenderRequest<'_>,
        buffer: &B,
    ) -> BufferSourceSelectiveDisplayTailRenderOutcome {
        request.render_if_needed_and_apply(source_walk, buffer, self.state.reborrow())
    }

    fn render_line_break<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        request: BufferSourceLineBreakRenderRequest<'_>,
        buffer: &B,
    ) -> DisplayRowTransitionContinuation {
        request.render_and_apply(source_walk, buffer, self.state.reborrow())
    }
}
