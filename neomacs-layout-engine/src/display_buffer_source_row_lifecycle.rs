//! Buffer source special-row lifecycle rendering.
//!
//! This module owns the buffer source row lifecycle actions that sit between
//! source walking and generic row/source append rendering: hscroll skip,
//! selective display, invisible text, line breaks, and end-of-buffer tails.

use crate::display_buffer_source_loop_state::BufferSourceLoopMutableState;
use crate::display_buffer_source_walk::BufferSourceWalk;
use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_face_state::DisplayRowActiveFaceState;
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowGeometryState, DisplayRowHitRange, DisplayRowLimit,
    DisplayRowScopedValue, DisplayRowYPositions,
};
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::display_row_source_append::{
    BufferSyntheticTextRenderContext, SyntheticTextAppendRequest, SyntheticTextMarker,
};
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_transition::{
    DisplayRowLineBreakTransitionPlan, DisplayRowTextWindowEmitContext,
    DisplayRowTransitionContinuation, DisplayRowTransitionRenderState,
};
use crate::display_row_walk_state::{
    BoxFaceRowState, HitRowRangeTracker, HorizontalScrollSkipState, InvisibleTextScanCheckpoint,
    LineNumberRenderState, TrailingWhitespaceRenderState, sync_position_after_row_transition,
};
use crate::display_source::DisplaySourceStepChar;
use crate::display_source::DisplaySourceTextPosition;
use crate::display_source_progress::{DisplaySourceProgressState, DisplaySourceRowProgressState};
use crate::hit_test::HitRow;
use crate::neovm_bridge::{LayoutBufferView, RustTextPropAccess};
use crate::unicode::is_wide_char;
use crate::window_output::{DisplayTextRowTransition, WindowOutputEmitter};
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::{EmacsBytePos, LispCharPos1};

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceEndOfBufferTailRenderContext<'a> {
    byte_idx: usize,
    charpos: i64,
    accessible_end: i64,
    point_charpos: i64,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    active_face_state: &'a DisplayRowActiveFaceState,
}

pub(crate) struct BufferSourceEndOfBufferTailRenderOutcome {
    point_is_visible_eob: bool,
}

impl BufferSourceEndOfBufferTailRenderOutcome {
    pub(crate) fn point_is_visible_eob(self) -> bool {
        self.point_is_visible_eob
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSourceHscrollSkipAction {
    LineBreak {
        ch_start_byte_idx: usize,
        charpos: i64,
    },
    Text {
        ch_start_byte_idx: usize,
        charpos: i64,
        show_left_truncation: bool,
    },
}

impl BufferSourceHscrollSkipAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn ch_start_byte_idx(self) -> usize {
        match self {
            Self::LineBreak {
                ch_start_byte_idx, ..
            }
            | Self::Text {
                ch_start_byte_idx, ..
            } => ch_start_byte_idx,
        }
    }

    pub(crate) fn charpos(self) -> i64 {
        match self {
            Self::LineBreak { charpos, .. } | Self::Text { charpos, .. } => charpos,
        }
    }

    pub(crate) fn should_show_left_truncation(self) -> bool {
        matches!(
            self,
            Self::Text {
                show_left_truncation: true,
                ..
            }
        )
    }

    pub(crate) fn apply_line_break_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        output_emitter: &mut WindowOutputEmitter,
        x: &mut f32,
        content_x: f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            output_emitter.note_display_buffer_pos(LispCharPos1::new(self.charpos()));
            row_extend.clear();
        }
    }

    pub(crate) fn line_break_hit_range(
        self,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> Option<DisplayRowHitRange> {
        if !self.is_line_break() {
            return None;
        }
        let hit_range = hit_row_range.range_to(self.charpos());
        hit_row_range.advance_to(self.charpos());
        Some(hit_range)
    }

    pub(crate) fn capture_line_break_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
        char_h: f32,
    ) {
        if !target.is_missing() || point_charpos != self.charpos() {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::line_break_from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.ch_start_byte_idx(), col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
                char_h,
            ),
        );
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: DisplayTextRowTransition,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
        char_h: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.capture_line_break_cursor_if_point(
            target,
            active_face_state,
            row_geometry,
            point_charpos,
            x,
            col,
            char_h,
        );
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn capture_text_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || point_charpos != self.charpos() {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.ch_start_byte_idx(), col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn append_left_truncation_marker_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        source_render: &mut TextRowSourceRenderState<'_>,
        mut row_progress: DisplaySourceRowProgressState<'_>,
        content_x: f32,
    ) {
        if !self.should_show_left_truncation() {
            return;
        }
        append_hscroll_truncation_marker_to_text_row(
            render_context,
            row_geometry,
            source_render,
            &mut row_progress,
            content_x,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSourceEndOfBufferCursorAction {
    byte_idx: usize,
    charpos: i64,
    accessible_end: i64,
    point_charpos: i64,
}

impl BufferSourceEndOfBufferCursorAction {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
    ) -> Self {
        Self {
            byte_idx,
            charpos,
            accessible_end,
            point_charpos,
        }
    }

    fn point_is_visible_eob(self) -> bool {
        self.point_charpos == self.accessible_end && self.charpos == self.accessible_end
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing()
            || (self.charpos != self.point_charpos && !self.point_is_visible_eob())
        {
            return;
        }
        if self.point_is_visible_eob() {
            tracing::debug!(
                "layout_window_rust: capturing EOB cursor at x={:.1} y={:.1} point={} point-max={}",
                x,
                row_geometry.glyph_y(0.0),
                self.point_charpos,
                self.accessible_end
            );
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSourceEndOfBufferTailAction {
    cursor: BufferSourceEndOfBufferCursorAction,
}

impl BufferSourceEndOfBufferTailAction {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
    ) -> Self {
        Self {
            cursor: BufferSourceEndOfBufferCursorAction::new(
                byte_idx,
                charpos,
                accessible_end,
                point_charpos,
            ),
        }
    }

    pub(crate) fn point_is_visible_eob(self) -> bool {
        self.cursor.point_is_visible_eob()
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        self.cursor
            .capture_cursor_if_point(target, active_face_state, row_geometry, x, col);
    }
}

impl<'a> BufferSourceEndOfBufferTailRenderContext<'a> {
    pub(crate) fn new(
        byte_idx: usize,
        charpos: i64,
        accessible_end: i64,
        point_charpos: i64,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> Self {
        Self {
            byte_idx,
            charpos,
            accessible_end,
            point_charpos,
            overlay_context,
            active_face_state,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        source_render: TextRowSourceRenderState<'_>,
        row_progress: DisplaySourceRowProgressState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        cursor_info: &mut CursorCaptureState,
        hit_rows: &mut Vec<HitRow>,
        hit_row_range: &mut HitRowRangeTracker,
        row_y_positions: &mut DisplayRowYPositions,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> BufferSourceEndOfBufferTailRenderOutcome {
        let mut row_progress = row_progress;
        let mut source_render = source_render;

        let tail = BufferSourceEndOfBufferTailAction::new(
            self.byte_idx,
            self.charpos,
            self.accessible_end,
            self.point_charpos,
        );
        let point_is_visible_eob = tail.point_is_visible_eob();
        tail.capture_cursor_if_point(
            cursor_info,
            self.active_face_state,
            row_geometry,
            row_progress.x(),
            row_progress.col(),
        );

        if self.overlay_context.should_render(row_geometry) {
            let (x, col) = row_progress.coordinates_mut();
            self.overlay_context.render_at_text_row(
                buffer,
                self.charpos,
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
        }

        BufferSourceEndOfBufferTailRenderOutcome {
            point_is_visible_eob,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceHscrollSkipRenderContext<'a> {
    text: &'a [u8],
    tab_width: i32,
    content_x: f32,
    append_surface: &'a DisplayRowAppendSurface,
    active_face_state: &'a DisplayRowActiveFaceState,
    metrics: DisplayRowFallbackMetrics,
    point_charpos: i64,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

pub(crate) fn consume_hscroll_skip_from_position(
    text: &[u8],
    position: &mut DisplaySourceTextPosition,
    hscroll_skip: &mut HorizontalScrollSkipState,
    tab_width: i32,
) -> Option<BufferSourceHscrollSkipAction> {
    let source_char = position.consume_step_char(text)?;
    Some(consume_source_char_for_hscroll(
        source_char,
        hscroll_skip,
        tab_width,
    ))
}

fn consume_source_char_for_hscroll(
    source_char: DisplaySourceStepChar,
    hscroll_skip: &mut HorizontalScrollSkipState,
    tab_width: i32,
) -> BufferSourceHscrollSkipAction {
    let end_charpos = source_char.start_charpos() + 1;
    if source_char.ch() == '\n' {
        return BufferSourceHscrollSkipAction::LineBreak {
            ch_start_byte_idx: source_char.start_byte_idx(),
            charpos: end_charpos,
        };
    }

    hscroll_skip.consume_columns(hscroll_skip_column_width(
        source_char,
        tab_width,
        hscroll_skip.consumed_columns(),
    ));
    BufferSourceHscrollSkipAction::Text {
        ch_start_byte_idx: source_char.start_byte_idx(),
        charpos: end_charpos,
        show_left_truncation: !hscroll_skip.should_skip()
            && hscroll_skip.should_show_left_truncation(),
    }
}

fn hscroll_skip_column_width(
    source_char: DisplaySourceStepChar,
    tab_width: i32,
    consumed_columns: i32,
) -> i32 {
    if source_char.ch() == '\t' {
        let tab_width = tab_width.max(1);
        return ((consumed_columns / tab_width + 1) * tab_width) - consumed_columns;
    }

    if is_wide_char(source_char.ch()) { 2 } else { 1 }
}

impl<'a> BufferSourceHscrollSkipRenderContext<'a> {
    pub(crate) fn render_next_and_apply<B: LayoutBufferView>(
        self,
        source_walk: &mut BufferSourceWalk<'_, B>,
        state: BufferSourceLoopMutableState<'_, '_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferSourceLoopMutableState {
            progress,
            hscroll_skip,
            row_extend,
            source_render,
            prefix_request,
            line_numbers,
            word_wrap,
            trailing_whitespace,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            cursor_info,
            row_y_positions,
            ..
        } = state;
        let mut progress = progress;
        let mut source_render = source_render;
        let context = self;

        let Some(hscroll_action) = source_walk
            .consume_hscroll_skip(
                context.text,
                progress.source_position(),
                hscroll_skip,
                context.tab_width,
            )
            .apply_to_progress(&mut progress)
        else {
            return DisplayRowTransitionContinuation::Exhausted;
        };

        if hscroll_action.is_line_break() {
            hscroll_action.apply_line_break_before_row_transition(
                row_extend,
                source_render.output_emitter(),
                progress.row_progress_mut().x_mut(),
                context.content_x,
            );
            let row_position = progress.row_position();
            let line_break_transition = DisplayRowLineBreakTransitionPlan::hscroll_line_break();
            let hit_range = hscroll_action
                .line_break_hit_range(hit_row_range)
                .expect("hscroll line break hit range");
            let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                context.row_geometry_defaults,
                context.display_text_row_base,
                row_y_positions,
                context.max_rows,
                row_geometry,
                row_flags,
                context.row_limit,
                hit_rows,
                &mut source_render,
            )
            .emit_line_break_then_row_start(
                line_break_transition,
                hit_range,
                row_position,
                0.0,
                DisplayRowTransitionRenderState::new(
                    prefix_request,
                    context.has_prefix,
                    line_numbers,
                    hscroll_skip,
                    word_wrap,
                    trailing_whitespace,
                ),
                progress.row_progress_mut().coordinates_mut().1,
            );
            return hscroll_action.apply_after_line_break_row_transition(
                row_transition,
                cursor_info,
                context.active_face_state,
                row_geometry,
                context.point_charpos,
                row_position.x_px(),
                row_position.col(),
                context.metrics.row_height(),
            );
        }

        hscroll_action.append_left_truncation_marker_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                0.0,
                context.metrics,
            ),
            row_geometry,
            &mut source_render.reborrow(),
            progress.row_progress_mut().reborrow(),
            context.content_x,
        );
        let row_position = progress.row_position();
        hscroll_action.capture_text_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            context.point_charpos,
            row_position.x_px(),
            row_position.col(),
        );
        DisplayRowTransitionContinuation::Continue
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'a [u8],
        tab_width: i32,
        content_x: f32,
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
        metrics: DisplayRowFallbackMetrics,
        point_charpos: i64,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            text,
            tab_width,
            content_x,
            append_surface,
            active_face_state,
            metrics,
            point_charpos,
            has_prefix,
            row_geometry_defaults,
            display_text_row_base,
            max_rows,
            row_limit,
        }
    }
}

pub(crate) struct BufferSourceSelectiveDisplayTailRenderRequest<'a> {
    source_char: DisplaySourceStepChar,
    context: BufferSourceSelectiveDisplayTailRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceSelectiveDisplayTailRenderContext<'a> {
    text: &'a [u8],
    text_start_byte: usize,
    selective_display: i32,
    tab_width: i32,
    append_surface: &'a DisplayRowAppendSurface,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    metrics: DisplayRowFallbackMetrics,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceInvisibleTextRenderContext<'a> {
    text: &'a [u8],
    accessible_end: i64,
    point_charpos: i64,
    append_surface: &'a DisplayRowAppendSurface,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    metrics: DisplayRowFallbackMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSourceSelectiveDisplayTailRenderOutcome {
    NotHidden,
    ContinueBufferWalk,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSourceInvisibleTextRenderOutcome {
    Visible,
    ContinueBufferWalk,
}

impl BufferSourceSelectiveDisplayTailRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferSourceInvisibleTextRenderOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl<'a> BufferSourceSelectiveDisplayTailRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'a [u8],
        text_start_byte: usize,
        selective_display: i32,
        tab_width: i32,
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        metrics: DisplayRowFallbackMetrics,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            text,
            text_start_byte,
            selective_display,
            tab_width,
            append_surface,
            active_face_state,
            glyph_y_offset,
            metrics,
            content_x,
            has_prefix,
            row_geometry_defaults,
            display_text_row_base,
            max_rows,
            row_limit,
        }
    }
}

impl<'a> BufferSourceInvisibleTextRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'a [u8],
        accessible_end: i64,
        point_charpos: i64,
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            text,
            accessible_end,
            point_charpos,
            append_surface,
            overlay_context,
            active_face_state,
            glyph_y_offset,
            metrics,
        }
    }
}

impl<'a> BufferSourceSelectiveDisplayTailRenderRequest<'a> {
    pub(crate) fn new(
        source_char: DisplaySourceStepChar,
        context: BufferSourceSelectiveDisplayTailRenderContext<'a>,
    ) -> Self {
        Self {
            source_char,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        source_walk: &mut BufferSourceWalk<'_, B>,
        buffer: &B,
        state: BufferSourceLoopMutableState<'_, '_, '_>,
    ) -> BufferSourceSelectiveDisplayTailRenderOutcome {
        let context = self.context;
        let selective_display = BufferSourceSelectiveDisplayContext::new(
            context.text,
            context.selective_display,
            context.tab_width,
        );
        let Some(marker) = selective_display.carriage_return_tail_marker(self.source_char.ch())
        else {
            return BufferSourceSelectiveDisplayTailRenderOutcome::NotHidden;
        };

        let BufferSourceLoopMutableState {
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
            row_y_positions,
            ..
        } = state;
        let mut source_render = source_render;

        marker.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                context.glyph_y_offset,
                context.metrics,
            ),
            row_geometry,
            &mut source_render.reborrow(),
            progress.row_progress_mut().reborrow(),
        );

        let tail_action = source_walk
            .consume_selective_display_tail(selective_display, progress.source_position())
            .apply_to_progress(&mut progress);
        if !tail_action.is_line_break() {
            return BufferSourceSelectiveDisplayTailRenderOutcome::ContinueBufferWalk;
        }

        tail_action.apply_hidden_line_break_row_state(
            row_geometry,
            row_extend,
            box_face,
            context.content_x,
            progress.row_progress_mut().x_mut(),
        );
        let row_position = progress.row_position();
        let line_break_transition = DisplayRowLineBreakTransitionPlan::hidden_line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            context.row_geometry_defaults,
            context.display_text_row_base,
            row_y_positions,
            context.max_rows,
            row_geometry,
            row_flags,
            context.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(progress.charpos()),
            row_position,
            0.0,
            DisplayRowTransitionRenderState::new(
                prefix_request,
                context.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            progress.row_progress_mut().coordinates_mut().1,
        );
        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                context.text_start_byte + progress.source_position().byte_idx(),
            ))
            .get() as i64;
        let mut synced_source_position = progress.source_position();
        let continuation = tail_action.apply_after_hidden_line_break_transition(
            row_transition,
            synced_charpos,
            &mut synced_source_position,
            hit_row_range,
        );
        source_walk
            .source_position_update(synced_source_position)
            .apply_to_progress(&mut progress);
        if continuation.should_break() {
            return BufferSourceSelectiveDisplayTailRenderOutcome::Stop;
        }

        BufferSourceSelectiveDisplayTailRenderOutcome::ContinueBufferWalk
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSourceInvisibleTextScanAction {
    Unchecked,
    Visible { next_visible: i64 },
    Hidden(BufferSourceInvisibleTextSkip),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSourceInvisibleTextSkip {
    start_byte_idx: usize,
    start_charpos: i64,
    skip_to: i64,
    next_visible: i64,
    point_in_hidden_region: bool,
    ellipsis: bool,
}

impl BufferSourceInvisibleTextSkip {
    pub(crate) fn new(
        start_byte_idx: usize,
        start_charpos: i64,
        skip_to: i64,
        next_visible: i64,
        point_in_hidden_region: bool,
        ellipsis: bool,
    ) -> Self {
        Self {
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            ellipsis,
        }
    }

    #[cfg(test)]
    pub(crate) fn start_byte_idx(self) -> usize {
        self.start_byte_idx
    }

    #[cfg(test)]
    pub(crate) fn start_charpos(self) -> i64 {
        self.start_charpos
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    #[cfg(test)]
    pub(crate) fn next_visible(self) -> i64 {
        self.next_visible
    }

    #[cfg(test)]
    pub(crate) fn point_in_hidden_region(self) -> bool {
        self.point_in_hidden_region
    }

    #[cfg(test)]
    pub(crate) fn ellipsis(self) -> bool {
        self.ellipsis
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !self.point_in_hidden_region {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.start_byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<SyntheticTextAppendRequest> {
        self.ellipsis.then(|| {
            SyntheticTextAppendRequest::active_marker(
                position,
                SyntheticTextMarker::InvisibleEllipsis,
            )
        })
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        cursor_info: &mut CursorCaptureState,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_progress: &mut DisplaySourceRowProgressState<'_>,
    ) {
        let position = row_progress.row_position();
        self.capture_cursor_if_point(
            cursor_info,
            render_context.active_face(),
            row_geometry,
            position.x_px(),
            position.col(),
        );

        let Some(request) = self.ellipsis_append_request(position) else {
            return;
        };
        append_synthetic_request_to_text_row(
            render_context,
            row_geometry,
            source_render,
            row_progress,
            request,
        );
    }
}

impl<'a> BufferSourceInvisibleTextRenderContext<'a> {
    pub(crate) fn render_at_checkpoint_and_apply<B: LayoutBufferView>(
        self,
        source_walk: &mut BufferSourceWalk<'_, B>,
        buffer: &B,
        state: BufferSourceLoopMutableState<'_, '_, '_>,
    ) -> BufferSourceInvisibleTextRenderOutcome {
        let BufferSourceLoopMutableState {
            invisible_text_checkpoint,
            mut progress,
            source_render,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
            ..
        } = state;
        let mut source_render = source_render;
        let context = self;

        let action = source_walk
            .consume_invisible_checkpoint(
                buffer,
                BufferSourceInvisibleTextScanContext::new(
                    context.text,
                    context.accessible_end,
                    context.point_charpos,
                    cursor_info.is_missing(),
                ),
                invisible_text_checkpoint,
                progress.source_position(),
            )
            .apply_to_progress(&mut progress);
        let BufferSourceInvisibleTextScanAction::Hidden(hidden_text) = action else {
            return BufferSourceInvisibleTextRenderOutcome::Visible;
        };

        let mut row_progress = progress.row_progress_mut().reborrow();
        hidden_text.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                context.glyph_y_offset,
                context.metrics,
            ),
            row_geometry,
            cursor_info,
            &mut source_render.reborrow(),
            &mut row_progress,
        );

        let overlay_charpos = progress.charpos();
        let (x, col) = progress.row_progress_mut().coordinates_mut();
        context.overlay_context.render_at_text_row(
            buffer,
            overlay_charpos,
            source_render.reborrow(),
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        );
        BufferSourceInvisibleTextRenderOutcome::ContinueBufferWalk
    }
}

pub(crate) struct BufferSourceInvisibleTextScanContext<'a> {
    text: &'a [u8],
    accessible_end: i64,
    point_charpos: i64,
    cursor_missing: bool,
}

impl<'a> BufferSourceInvisibleTextScanContext<'a> {
    pub(crate) fn new(
        text: &'a [u8],
        accessible_end: i64,
        point_charpos: i64,
        cursor_missing: bool,
    ) -> Self {
        Self {
            text,
            accessible_end,
            point_charpos,
            cursor_missing,
        }
    }

    pub(crate) fn consume_at_checkpoint<B: LayoutBufferView>(
        &self,
        buffer: &B,
        checkpoints: &mut InvisibleTextScanCheckpoint,
        position: &mut DisplaySourceTextPosition,
    ) -> BufferSourceInvisibleTextScanAction {
        if !checkpoints.should_check(position.charpos()) {
            return BufferSourceInvisibleTextScanAction::Unchecked;
        }

        let start_byte_idx = position.byte_idx();
        let start_charpos = position.charpos();
        let text_props = RustTextPropAccess::new(buffer);
        let (invisible, next_visible) = text_props.check_invisible(start_charpos);
        checkpoints.record_next_visible(next_visible);

        if !invisible.hidden {
            return BufferSourceInvisibleTextScanAction::Visible { next_visible };
        }

        let skip_to = next_visible.min(self.accessible_end);
        let point_in_hidden_region = self.cursor_missing
            && self.point_charpos >= start_charpos
            && self.point_charpos < skip_to;
        position.skip_chars_until(self.text, skip_to);

        BufferSourceInvisibleTextScanAction::Hidden(BufferSourceInvisibleTextSkip::new(
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            invisible.ellipsis,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSourceSelectiveDisplayLineTailAction {
    Exhausted,
    LineBreak { charpos: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSourceSelectiveDisplayLineTailMarker;

impl BufferSourceSelectiveDisplayLineTailMarker {
    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::active_marker(position, SyntheticTextMarker::SelectiveEllipsis)
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        source_render: &mut TextRowSourceRenderState<'_>,
        mut row_progress: DisplaySourceRowProgressState<'_>,
    ) {
        let request = self.ellipsis_append_request(row_progress.row_position());
        append_synthetic_request_to_text_row(
            render_context,
            row_geometry,
            source_render,
            &mut row_progress,
            request,
        );
    }
}

impl BufferSourceSelectiveDisplayLineTailAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn apply_hidden_line_break_row_state(
        self,
        row_geometry: &DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
        x: &mut f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            row_extend.clear();
            box_face.continue_on_row(row_geometry.next_row_marker(), content_x);
        }
    }

    pub(crate) fn apply_after_hidden_line_break_transition(
        self,
        row_transition: DisplayTextRowTransition,
        synced_charpos: i64,
        position: &mut DisplaySourceTextPosition,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        sync_position_after_row_transition(synced_charpos, position, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> Option<i64> {
        match self {
            Self::LineBreak { charpos } => Some(charpos),
            Self::Exhausted => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSourceSelectiveDisplayHiddenLines {
    hidden_line_count: usize,
}

impl BufferSourceSelectiveDisplayHiddenLines {
    fn new(hidden_line_count: usize) -> Self {
        Self { hidden_line_count }
    }

    #[cfg(test)]
    pub(crate) fn hidden_line_count(self) -> usize {
        self.hidden_line_count
    }

    pub(crate) fn apply_to_line_numbers(self, line_numbers: &mut LineNumberRenderState) {
        for _ in 0..self.hidden_line_count {
            line_numbers.advance_hidden_line();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSourceSelectiveDisplayContext<'a> {
    text: &'a [u8],
    selective_display: i32,
    tab_width: i32,
}

impl<'a> BufferSourceSelectiveDisplayContext<'a> {
    pub(crate) fn new(text: &'a [u8], selective_display: i32, tab_width: i32) -> Self {
        Self {
            text,
            selective_display,
            tab_width: tab_width.max(1),
        }
    }

    pub(crate) fn hides_carriage_return_tail(self, ch: char) -> bool {
        self.selective_display > 0 && ch == '\r'
    }

    pub(crate) fn carriage_return_tail_marker(
        self,
        ch: char,
    ) -> Option<BufferSourceSelectiveDisplayLineTailMarker> {
        self.hides_carriage_return_tail(ch)
            .then_some(BufferSourceSelectiveDisplayLineTailMarker)
    }

    pub(crate) fn hides_indented_lines_after_line_break(self, byte_idx: usize) -> bool {
        self.selective_display > 0
            && self.selective_display < i32::MAX
            && byte_idx < self.text.len()
    }

    pub(crate) fn skip_rest_of_line_after_carriage_return(
        self,
        position: &mut DisplaySourceTextPosition,
    ) -> BufferSourceSelectiveDisplayLineTailAction {
        position.advance_charpos_by_one();
        if position.consume_until_line_break(self.text) {
            return BufferSourceSelectiveDisplayLineTailAction::LineBreak {
                charpos: position.charpos(),
            };
        }

        BufferSourceSelectiveDisplayLineTailAction::Exhausted
    }

    pub(crate) fn skip_hidden_indented_lines_after_line_break(
        self,
        position: &mut DisplaySourceTextPosition,
    ) -> BufferSourceSelectiveDisplayHiddenLines {
        let mut hidden_line_count = 0;
        while position.byte_idx() < self.text.len() {
            let Some(indent) = self.indentation_columns_at(position.byte_idx()) else {
                break;
            };
            if indent <= self.selective_display {
                break;
            }

            if self.skip_line(position) {
                hidden_line_count += 1;
            }
        }

        BufferSourceSelectiveDisplayHiddenLines::new(hidden_line_count)
    }

    pub(crate) fn apply_hidden_indented_lines_after_line_break(
        self,
        position: &mut DisplaySourceTextPosition,
        line_numbers: &mut LineNumberRenderState,
    ) -> BufferSourceSelectiveDisplayHiddenLines {
        if !self.hides_indented_lines_after_line_break(position.byte_idx()) {
            return BufferSourceSelectiveDisplayHiddenLines::new(0);
        }
        let hidden_lines = self.skip_hidden_indented_lines_after_line_break(position);
        hidden_lines.apply_to_line_numbers(line_numbers);
        hidden_lines
    }

    fn indentation_columns_at(self, mut byte_idx: usize) -> Option<i32> {
        if byte_idx >= self.text.len() {
            return None;
        }

        let mut indent = 0i32;
        while byte_idx < self.text.len() {
            match self.text[byte_idx] {
                b' ' => {
                    indent += 1;
                    byte_idx += 1;
                }
                b'\t' => {
                    indent = ((indent / self.tab_width) + 1) * self.tab_width;
                    byte_idx += 1;
                }
                _ => break,
            }
        }
        Some(indent)
    }

    fn skip_line(self, position: &mut DisplaySourceTextPosition) -> bool {
        position.consume_until_line_break(self.text)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferSourceLineBreakSourceAction {
    ch_start_byte_idx: usize,
    charpos: i64,
    next_charpos: i64,
    line_spacing: f32,
}

pub(crate) struct BufferSourceLineBreakRenderRequest<'a> {
    source_char: DisplaySourceStepChar,
    context: BufferSourceLineBreakRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceLineBreakRenderContext<'a> {
    text: &'a [u8],
    text_start_byte: usize,
    selective_display: i32,
    tab_width: i32,
    active_face_state: &'a DisplayRowActiveFaceState,
    point_charpos: i64,
    char_h: f32,
    extra_line_spacing: f32,
    content_x: f32,
    has_prefix: bool,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
    append_surface: &'a DisplayRowAppendSurface,
    frame_background: Color,
    overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
}

impl<'a> BufferSourceLineBreakRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'a [u8],
        text_start_byte: usize,
        selective_display: i32,
        tab_width: i32,
        active_face_state: &'a DisplayRowActiveFaceState,
        point_charpos: i64,
        char_h: f32,
        extra_line_spacing: f32,
        content_x: f32,
        has_prefix: bool,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
        append_surface: &'a DisplayRowAppendSurface,
        frame_background: Color,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    ) -> Self {
        Self {
            text,
            text_start_byte,
            selective_display,
            tab_width,
            active_face_state,
            point_charpos,
            char_h,
            extra_line_spacing,
            content_x,
            has_prefix,
            row_geometry_defaults,
            display_text_row_base,
            max_rows,
            row_limit,
            append_surface,
            frame_background,
            overlay_context,
        }
    }
}

impl<'a> BufferSourceLineBreakRenderRequest<'a> {
    pub(crate) fn new(
        source_char: DisplaySourceStepChar,
        context: BufferSourceLineBreakRenderContext<'a>,
    ) -> Self {
        debug_assert_eq!(source_char.ch(), '\n');
        Self {
            source_char,
            context,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        source_walk: &mut BufferSourceWalk<'_, B>,
        buffer: &B,
        state: BufferSourceLoopMutableState<'_, '_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferSourceLoopMutableState {
            mut progress,
            cursor_info,
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render,
            prefix_request,
            line_numbers,
            hscroll_skip,
            word_wrap,
            row_flags,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
            ..
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let line_break_action = BufferSourceLineBreakSourceAction::for_source_step_newline(
            buffer,
            self.source_char,
            context.char_h,
            context.extra_line_spacing,
        );
        {
            let overlay_charpos = progress.charpos();
            let (x, col) = progress.row_progress_mut().coordinates_mut();
            context.overlay_context.render_at_text_row(
                buffer,
                overlay_charpos,
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
        }
        let row_position = progress.row_position();
        line_break_action.capture_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            context.point_charpos,
            row_position.x_px(),
            row_position.col(),
        );
        {
            let metrics = context.active_face_state.metrics();
            // R2L (reversed_p) is read from the GlyphRow inside the mutation,
            // which is the authoritative source; pass `false` here. R2L rows are
            // a documented limitation (the fill is suppressed for them).
            source_render.extend_face_to_end_of_line(
                row_extend,
                row_geometry,
                progress.row_progress().x(),
                context.append_surface.right_edge(),
                context.frame_background,
                false,
                metrics.row_height(),
                metrics.ascent(),
                metrics.char_width(),
            );
        }
        line_break_action.apply_before_row_transition(
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render.output_emitter(),
            context.content_x,
            &mut progress,
        );

        let line_break_transition = DisplayRowLineBreakTransitionPlan::line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            context.row_geometry_defaults,
            context.display_text_row_base,
            row_y_positions,
            context.max_rows,
            row_geometry,
            row_flags,
            context.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(progress.charpos()),
            progress.row_position(),
            line_break_action.line_spacing(),
            DisplayRowTransitionRenderState::new(
                prefix_request,
                context.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            progress.row_progress_mut().col_mut(),
        );

        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                context.text_start_byte + progress.source_position().byte_idx(),
            ))
            .get() as i64;
        let mut synced_source_position = progress.source_position();
        let continuation = line_break_action.apply_after_line_break_row_transition(
            row_transition,
            synced_charpos,
            &mut synced_source_position,
            hit_row_range,
            row_geometry,
            box_face,
            context.content_x,
        );
        source_walk
            .source_position_update(synced_source_position)
            .apply_to_progress(&mut progress);
        if continuation.should_break() {
            return continuation;
        }

        source_walk
            .consume_hidden_indented_lines_after_line_break(
                BufferSourceSelectiveDisplayContext::new(
                    context.text,
                    context.selective_display,
                    context.tab_width,
                ),
                progress.source_position(),
                line_numbers,
            )
            .apply_to_progress(&mut progress);
        DisplayRowTransitionContinuation::Continue
    }
}

impl BufferSourceLineBreakSourceAction {
    pub(crate) fn for_newline<B: LayoutBufferView>(
        buffer: &B,
        charpos: i64,
        ch_start_byte_idx: usize,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        let text_prop_spacing = RustTextPropAccess::new(buffer).check_line_spacing(charpos, char_h);
        let line_spacing = if text_prop_spacing > 0.0 {
            text_prop_spacing
        } else if extra_line_spacing > 0.0 {
            extra_line_spacing
        } else {
            0.0
        };
        Self {
            ch_start_byte_idx,
            charpos,
            next_charpos: charpos + 1,
            line_spacing,
        }
    }

    pub(crate) fn for_source_step_newline<B: LayoutBufferView>(
        buffer: &B,
        source_char: DisplaySourceStepChar,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        Self::for_newline(
            buffer,
            source_char.start_charpos(),
            source_char.start_byte_idx(),
            char_h,
            extra_line_spacing,
        )
    }

    pub(crate) fn point_matches(self, point_charpos: i64) -> bool {
        point_charpos == self.charpos
    }

    pub(crate) fn next_charpos(self) -> i64 {
        self.next_charpos
    }

    pub(crate) fn line_spacing(self) -> f32 {
        self.line_spacing
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        output_emitter: &mut WindowOutputEmitter,
        content_x: f32,
        progress: &mut DisplaySourceProgressState<'_>,
    ) {
        trailing_whitespace.reset_after_row_transition();
        row_extend.clear();
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
        progress.set_charpos(self.next_charpos());
        *progress.row_progress_mut().x_mut() = content_x;
        output_emitter.note_display_buffer_pos(LispCharPos1::new(progress.charpos()));
    }

    pub(crate) fn apply_after_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) {
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: DisplayTextRowTransition,
        synced_charpos: i64,
        position: &mut DisplaySourceTextPosition,
        hit_row_range: &mut HitRowRangeTracker,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        sync_position_after_row_transition(synced_charpos, position, hit_row_range);
        self.apply_after_row_transition(row_geometry, box_face, content_x);
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) -> CapturedCursorInfo {
        CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                row_geometry.text_position(x, self.ch_start_byte_idx, col),
                CapturedCursorSlotWidth::FaceChar,
                false,
            ),
        )
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || !self.point_matches(point_charpos) {
            return;
        }
        capture_cursor_info(
            target,
            self.cursor_info(active_face_state, row_geometry, x, col),
        );
    }
}

pub(crate) fn append_synthetic_request_to_text_row<'ctx>(
    render_context: BufferSyntheticTextRenderContext<'ctx>,
    row_geometry: &'ctx DisplayRowGeometryState,
    source_render: &mut TextRowSourceRenderState<'_>,
    row_progress: &mut DisplaySourceRowProgressState<'_>,
    request: SyntheticTextAppendRequest,
) {
    let Some(progress) =
        render_context.render_request_to_text_row(source_render, row_geometry, request)
    else {
        return;
    };
    row_progress.apply_position(progress.end());
}

pub(crate) fn append_hscroll_truncation_marker_to_text_row<'ctx>(
    render_context: BufferSyntheticTextRenderContext<'ctx>,
    row_geometry: &'ctx DisplayRowGeometryState,
    source_render: &mut TextRowSourceRenderState<'_>,
    row_progress: &mut DisplaySourceRowProgressState<'_>,
    content_x: f32,
) {
    let request =
        render_context.hscroll_truncation_request(source_render.default_face(), content_x);
    append_synthetic_request_to_text_row(
        render_context,
        row_geometry,
        source_render,
        row_progress,
        request,
    );
    source_render.mark_current_text_row_truncated_left();
}
