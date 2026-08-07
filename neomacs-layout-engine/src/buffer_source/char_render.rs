//! Single-character buffer source rendering.

use crate::buffer_source::item_append::BufferSourceRowAppendContext;
use crate::buffer_source::item_render::BufferSourceItemRenderOutcome;
use crate::buffer_source::loop_state::BufferSourceLoopMutableState;
use crate::buffer_source::overflow::{
    BufferSourceOverflowRenderContext, BufferSourceOverflowRenderRequest,
    BufferSourceSpecialOverflowRenderContext, BufferSourceSpecialOverflowRenderRequest,
};
use crate::buffer_source::walk::BufferSourceWalk;
use crate::display_cursor::capture_cursor_info;
use crate::display_row::append_context::DisplayRowAppendSurface;
use crate::display_row::face_state::DisplayRowActiveFaceState;
use crate::display_row::geometry::{
    DisplayRowGeometryDefaults, DisplayRowLimit, DisplayRowVisibilityLimit,
};
use crate::display_row::overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_source::DisplaySourceStepItem;
use crate::display_source_item_append::DisplaySourcePreparedCharAppend;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neomacs_display_protocol::types::Color;

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceCharRenderRequest<'a> {
    text: &'a [u8],
    text_start_byte: usize,
    append_surface: &'a DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    params: &'a WindowParams,
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

impl<'a> BufferSourceCharRenderRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'a [u8],
        text_start_byte: usize,
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        params: &'a WindowParams,
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
            text,
            text_start_byte,
            append_surface,
            overlay_context,
            params,
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
        self,
        mut source_item: DisplaySourceStepItem,
        source_walk: &mut BufferSourceWalk<'_, B>,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        append_context: &BufferSourceRowAppendContext<'_, '_, B>,
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

        // Element granularity: this path renders exactly ONE character, so take
        // the run's first character and leave the rest to the producer. The walk
        // position advances by that one character (the append does it), and the
        // next iteration reads the remainder straight from the cursor — the
        // producer's position IS the resume state. The remainder used to be
        // split into N single-character items and pushed back through a pending
        // queue for later iterations to pop.
        if let Some(first) = source_item
            .clone()
            .first_text_run_char(self.text_start_byte)
        {
            source_item = first;
        }

        let (source_step_char, source_end_charpos, source_end_byte_idx, source_item) =
            source_item.into_render_parts();
        let ch = source_step_char.ch();

        // Overlay strings are no longer emitted here. Since P4.6 the PRODUCER
        // surfaces them as a typed element at the anchor, with insertion
        // semantics, and the loop-level arm in render.rs appends them before the
        // buffer character at the same position — the GNU handle_stop order this
        // call used to implement by probing every character.
        let append_position = progress.row_position();
        let append_geometry = *row_geometry;
        source_step_char.record_word_wrap_candidate(word_wrap, &source_render);

        let buffer_source_char = source_step_char.source_char(self.params.nobreak_char_display);
        let prepared_append = append_context.prepare_source_item_for_current_text_row(
            append_geometry,
            source_walk.append_state(),
            &mut source_render,
            &buffer_source_char,
            self.text,
            source_step_char.start_byte_idx(),
            append_position,
            &source_item,
        );

        let prepared_append = match prepared_append {
            DisplaySourcePreparedCharAppend::Special(special_prepared_append) => {
                let special_overflow_outcome = BufferSourceSpecialOverflowRenderRequest::new(
                    &special_prepared_append,
                    BufferSourceSpecialOverflowRenderContext::new(
                        self.text,
                        self.text_start_byte,
                        progress.row_progress().x(),
                        self.append_surface.full_text_right_edge(),
                        self.params.wrap_mode,
                        self.row_visibility_limit,
                        self.content_x,
                        self.has_prefix,
                        self.row_geometry_defaults,
                        self.display_text_row_base,
                        self.max_rows,
                        self.row_limit,
                    ),
                )
                .render_if_needed_and_apply(
                    source_walk,
                    buffer,
                    BufferSourceLoopMutableState::new(
                        invisible_text_checkpoint,
                        progress.reborrow(),
                        source_render.reborrow(),
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
                );
                if special_overflow_outcome.should_break() {
                    return BufferSourceItemRenderOutcome::Stop;
                }
                if special_overflow_outcome.should_continue_buffer_walk() {
                    return BufferSourceItemRenderOutcome::ContinueBufferWalk;
                }

                if special_prepared_append
                    .append_to_text_row_and_apply(
                        append_context,
                        row_geometry,
                        self.params,
                        face_ids,
                        &mut source_render.reborrow(),
                        face_scan,
                        word_wrap,
                        &mut progress.reborrow(),
                    )
                    .should_break()
                {
                    return BufferSourceItemRenderOutcome::Stop;
                }
                if let Some(end_byte_idx) = source_end_byte_idx {
                    progress.set_byte_idx(end_byte_idx);
                }
                return BufferSourceItemRenderOutcome::ContinueBufferWalk;
            }
            DisplaySourcePreparedCharAppend::Text(prepared_append) => prepared_append,
        };

        prepared_append
            .update_cursor_info_for_main_char(cursor_info, source_step_char.start_byte_idx());
        let overflow_outcome = BufferSourceOverflowRenderRequest::new(
            &prepared_append,
            source_step_char,
            BufferSourceOverflowRenderContext::new(
                ch,
                self.append_surface.right_edge(),
                self.params.wrap_mode,
                *word_wrap,
                self.row_visibility_limit,
                self.content_x,
                self.has_prefix,
                self.row_geometry_defaults,
                self.display_text_row_base,
                self.max_rows,
                self.row_limit,
                active_face_state.metrics(),
                self.frame_background,
            ),
        )
        .render_if_needed_and_apply(
            source_walk,
            self.text,
            BufferSourceLoopMutableState::new(
                invisible_text_checkpoint,
                progress.reborrow(),
                source_render.reborrow(),
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
        );
        if overflow_outcome.should_break() {
            return BufferSourceItemRenderOutcome::Stop;
        }
        if overflow_outcome.should_continue_buffer_walk() {
            return BufferSourceItemRenderOutcome::ContinueBufferWalk;
        }

        let row_position = progress.row_position();
        prepared_append.capture_cursor_info_for_main_char_if_point(
            cursor_info,
            active_face_state,
            row_geometry,
            row_position.x_px(),
            source_step_char.start_byte_idx(),
            row_position.col(),
            ch == '\t',
            progress.charpos(),
            self.point_charpos,
        );
        if cursor_info.is_missing()
            && source_end_charpos.is_some_and(|end| {
                self.point_charpos > progress.charpos() && self.point_charpos < end
            })
        {
            capture_cursor_info(
                cursor_info,
                prepared_append.cursor_info_for_main_char(
                    active_face_state,
                    row_geometry.text_position(
                        row_position.x_px(),
                        source_step_char.start_byte_idx(),
                        row_position.col(),
                    ),
                    ch == '\t',
                ),
            );
        }

        if prepared_append
            .append_to_text_row_and_apply(
                append_context,
                &append_geometry,
                ch,
                &mut source_render.reborrow(),
                trailing_whitespace,
                word_wrap,
                &mut progress.reborrow(),
            )
            .should_break()
        {
            return BufferSourceItemRenderOutcome::Stop;
        }
        if let Some(end_charpos) = source_end_charpos {
            progress.max_charpos(end_charpos);
        }
        if let Some(end_byte_idx) = source_end_byte_idx {
            progress.set_byte_idx(end_byte_idx);
        }

        BufferSourceItemRenderOutcome::Rendered
    }
}
