//! Live window-output emission helpers for Rust redisplay.
//!
//! This layer bridges Rust layout/status-line emission to GNU-like live window
//! output state. It advances live output through explicit output-cursor moves
//! while simultaneously recording immutable row snapshots for renderer
//! handoff.

use super::display_status_line::DisplayRowOutputProgress;
use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_item::DisplaySourcePosition;
#[cfg(test)]
use crate::display_row_builder::DisplayRowAppendProgress;
use crate::display_row_builder::{DisplayRowGlyphSlot, DisplayRowPosition};
use crate::display_row_geometry::{DisplayRowFlagKind, DisplayRowFlags};
use crate::matrix_builder::GlyphMatrixBuilder;
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor,
};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::LispCharPos1;
use neovm_core::emacs_core::Context;
use neovm_core::window::{
    DisplayPointSnapshot, DisplayRowSnapshot, WindowCursorKind, WindowCursorPos,
    WindowCursorSnapshot, WindowDisplaySnapshot,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct RowMetricsSnapshot {
    pub(crate) row: usize,
    pub(crate) pixel_y: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug)]
struct CurrentRowProgress {
    row: i64,
    y: i64,
    col: i64,
    x: i64,
    start_col: i64,
    start_x: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TextOutputSpan {
    buffer_pos: LispCharPos1,
    row: usize,
    row_y: f32,
    glyph_y: f32,
    height: f32,
    start: DisplayRowPosition,
    end: DisplayRowPosition,
}

impl TextOutputSpan {
    fn can_merge(self, next: Self) -> bool {
        self.buffer_pos == next.buffer_pos
            && self.row == next.row
            && self.row_y == next.row_y
            && self.glyph_y == next.glyph_y
            && self.height == next.height
            && self.end == next.start
    }

    fn merge(&mut self, next: Self) {
        self.end = next.end;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextRowOutput {
    pub(crate) row: usize,
    pub(crate) row_y: f32,
    pub(crate) glyph_y: f32,
    pub(crate) height: f32,
}

impl TextRowOutput {
    fn span_for_buffer_slot(self, slot: &DisplayRowGlyphSlot) -> Option<TextOutputSpan> {
        let DisplaySourcePosition::Buffer { char_pos, .. } = slot.source else {
            return None;
        };
        Some(TextOutputSpan {
            buffer_pos: layout_i64_char_pos_to_lisp_char_pos(char_pos.get() as i64),
            row: self.row,
            row_y: self.row_y,
            glyph_y: self.glyph_y,
            height: self.height,
            start: DisplayRowPosition {
                x_px: slot.x_px,
                col: slot.col,
            },
            end: DisplayRowPosition {
                x_px: slot.x_px + slot.width_px,
                col: slot.col + slot.width_cols,
            },
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChromeRowOutput {
    pub(crate) row: i64,
    pub(crate) y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextMatrixRowMetrics {
    pub(crate) y: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextMatrixRowBegin {
    pub(crate) matrix_row: usize,
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) y: f32,
    pub(crate) x: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowBegin {
    pub(crate) window_id: u64,
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    pub(crate) bounds: Rect,
    pub(crate) text_bounds: Rect,
    pub(crate) selected: bool,
    pub(crate) first_row: TextMatrixRowBegin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextWindowDisplayRange {
    pub(crate) window_id: u64,
    pub(crate) window_start: LispCharPos1,
    pub(crate) window_end: LispCharPos1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextWindowRightEdgeMarkerColumn {
    LastColumn,
    BeforeRightBorder,
}

impl TextWindowRightEdgeMarkerColumn {
    fn target_col(self, matrix_cols: usize) -> usize {
        match self {
            Self::LastColumn => matrix_cols.saturating_sub(1),
            Self::BeforeRightBorder => matrix_cols.saturating_sub(2),
        }
    }
}

pub(crate) struct TextWindowRightEdgeMarkers<'a> {
    pub(crate) text_matrix_row_base: usize,
    pub(crate) matrix_cols: usize,
    pub(crate) column: TextWindowRightEdgeMarkerColumn,
    pub(crate) row_flags: &'a DisplayRowFlags,
    pub(crate) face_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextWindowLineNumberMargin<'a> {
    pub(crate) text: &'a str,
    pub(crate) cols: i32,
    pub(crate) face_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowCursor {
    pub(crate) selected: bool,
    pub(crate) window_id: i64,
    pub(crate) charpos: usize,
    pub(crate) slot_id: DisplaySlotId,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
    pub(crate) style: CursorStyle,
    pub(crate) color: Color,
    pub(crate) cursor_fg: Color,
    pub(crate) text_area_left: f32,
    pub(crate) window_top: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextWindowDecorativeCursor {
    pub(crate) window_id: i64,
    pub(crate) slot_id: DisplaySlotId,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) style: CursorStyle,
    pub(crate) color: Color,
    pub(crate) effects: Option<EffectsConfig>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextWindowCursorEffects {
    pub(crate) window_id: i64,
    pub(crate) effects: EffectsConfig,
}

impl TextWindowCursor {
    fn row(self) -> usize {
        self.slot_id.row as usize
    }

    fn col(self) -> u16 {
        self.slot_id.col
    }

    fn window_snapshot(self) -> WindowCursorSnapshot {
        WindowCursorSnapshot {
            kind: window_cursor_kind(self.style),
            x: (self.x - self.text_area_left).round() as i64,
            y: (self.y - self.window_top).round() as i64,
            width: self.width.round() as i64,
            height: self.height.round() as i64,
            ascent: self.ascent.round() as i64,
            row: self.row() as i64,
            col: i64::from(self.col()),
        }
    }

    fn phys_cursor(self) -> PhysCursor {
        PhysCursor {
            window_id: self.window_id,
            charpos: self.charpos,
            row: self.row(),
            col: self.col(),
            slot_id: self.slot_id,
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            ascent: self.ascent,
            style: self.style,
            color: self.color,
            cursor_fg: self.cursor_fg,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextMatrixRowGeometryTransition {
    pub(crate) finished_row: TextMatrixRowMetrics,
    pub(crate) begin_row: TextMatrixRowBegin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TextMatrixRowTransition {
    BeganNextRow,
    ExhaustedRows,
}

impl TextMatrixRowTransition {
    pub(crate) fn is_exhausted(self) -> bool {
        matches!(self, Self::ExhaustedRows)
    }
}

fn window_cursor_kind(style: CursorStyle) -> WindowCursorKind {
    match style {
        CursorStyle::FilledBox => WindowCursorKind::FilledBox,
        CursorStyle::Hollow => WindowCursorKind::HollowBox,
        CursorStyle::Bar(_) => WindowCursorKind::Bar,
        CursorStyle::Hbar(_) => WindowCursorKind::Hbar,
    }
}

pub(crate) struct TextMatrixRowOutput<'a> {
    builder: &'a mut GlyphMatrixBuilder,
    output_emitter: &'a mut WindowOutputEmitter,
    evaluator: &'a mut Context,
}

impl<'a> TextMatrixRowOutput<'a> {
    pub(crate) fn new(
        builder: &'a mut GlyphMatrixBuilder,
        output_emitter: &'a mut WindowOutputEmitter,
        evaluator: &'a mut Context,
    ) -> Self {
        Self {
            builder,
            output_emitter,
            evaluator,
        }
    }

    pub(crate) fn begin(&mut self, begin: TextMatrixRowBegin) {
        self.builder.begin_row(begin.matrix_row, GlyphRowRole::Text);
        self.output_emitter
            .begin_text_row(self.evaluator, begin.row, begin.col, begin.y, begin.x);
    }

    pub(crate) fn finish(&mut self, metrics: TextMatrixRowMetrics) {
        self.builder
            .set_current_row_metrics(metrics.y, metrics.height, metrics.ascent);
        self.output_emitter
            .push_text_row(metrics.y, metrics.height, metrics.ascent);
    }

    pub(crate) fn finish_and_end(&mut self, metrics: TextMatrixRowMetrics) {
        self.finish(metrics);
        self.builder.end_row();
    }

    pub(crate) fn emit(&mut self, transition: TextMatrixRowGeometryTransition) {
        self.finish_and_end(transition.finished_row);
        self.begin(transition.begin_row);
    }

    pub(crate) fn emit_with_row_limit(
        &mut self,
        transition: TextMatrixRowGeometryTransition,
        max_rows: usize,
    ) -> TextMatrixRowTransition {
        if transition.begin_row.row >= max_rows {
            self.finish_and_end(transition.finished_row);
            return TextMatrixRowTransition::ExhaustedRows;
        }
        self.emit(transition);
        TextMatrixRowTransition::BeganNextRow
    }
}

pub(crate) fn begin_text_window_output(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    request: TextWindowBegin,
) {
    builder.begin_window_with_text_bounds(
        request.window_id,
        request.rows,
        request.cols,
        request.bounds,
        request.text_bounds,
        request.selected,
    );
    TextMatrixRowOutput::new(builder, output_emitter, evaluator).begin(request.first_row);
}

pub(crate) fn record_text_window_display_range(
    builder: &mut GlyphMatrixBuilder,
    range: TextWindowDisplayRange,
) {
    if let Some(info) = builder.window_infos_last_mut()
        && info.window_id == range.window_id as i64
    {
        info.window_start = range.window_start.as_i64();
        info.window_end = range.window_end.as_i64();
    }
}

pub(crate) fn close_text_window_output(builder: &mut GlyphMatrixBuilder) {
    builder.end_window();
}

pub(crate) fn finish_text_matrix_row_output(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    metrics: TextMatrixRowMetrics,
) {
    TextMatrixRowOutput::new(builder, output_emitter, evaluator).finish(metrics);
}

pub(crate) fn finish_and_end_text_matrix_row_output(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    metrics: TextMatrixRowMetrics,
) {
    TextMatrixRowOutput::new(builder, output_emitter, evaluator).finish_and_end(metrics);
}

pub(crate) fn emit_text_matrix_row_transition(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    transition: TextMatrixRowGeometryTransition,
) {
    TextMatrixRowOutput::new(builder, output_emitter, evaluator).emit(transition);
}

pub(crate) fn emit_text_matrix_row_transition_with_limit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    transition: TextMatrixRowGeometryTransition,
    max_rows: usize,
) -> TextMatrixRowTransition {
    TextMatrixRowOutput::new(builder, output_emitter, evaluator)
        .emit_with_row_limit(transition, max_rows)
}

pub(crate) fn mark_current_text_row_truncated_left(builder: &mut GlyphMatrixBuilder) {
    builder.with_current_row_mut(|glyph_row| {
        glyph_row.truncated_left = true;
    });
}

pub(crate) fn emit_text_window_line_number_margin(
    builder: &mut GlyphMatrixBuilder,
    request: TextWindowLineNumberMargin<'_>,
) {
    let padding = (request.cols - 1) - request.text.chars().count() as i32;
    if padding > 0 {
        builder.push_left_margin_stretch(padding as u16, request.face_id);
    }
    for ch in request.text.chars() {
        builder.push_left_margin_char(ch, request.face_id);
    }
    builder.push_left_margin_stretch(1, request.face_id);
}

pub(crate) fn publish_text_window_cursor(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    cursor: TextWindowCursor,
) {
    if !cursor.selected {
        builder.push_cursor(
            cursor.window_id,
            cursor.slot_id,
            cursor.x,
            cursor.y,
            cursor.width,
            cursor.height,
            cursor.style,
            cursor.color,
        );
    }
    builder.set_cursor_at_row(cursor.row(), cursor.col(), cursor.style);
    output_emitter.set_phys_cursor(cursor.window_snapshot());
    if cursor.selected {
        builder.set_phys_cursor(cursor.phys_cursor());
    }
}

pub(crate) fn publish_text_window_decorative_cursor(
    builder: &mut GlyphMatrixBuilder,
    cursor: TextWindowDecorativeCursor,
) {
    if let Some(effects) = cursor.effects {
        install_text_window_cursor_effects(
            builder,
            TextWindowCursorEffects {
                window_id: cursor.window_id,
                effects,
            },
        );
    }
    builder.push_cursor(
        cursor.window_id,
        cursor.slot_id,
        cursor.x,
        cursor.y,
        cursor.width,
        cursor.height,
        cursor.style,
        cursor.color,
    );
}

pub(crate) fn install_text_window_cursor_effects(
    builder: &mut GlyphMatrixBuilder,
    request: TextWindowCursorEffects,
) {
    builder.set_window_cursor_effects(request.window_id, request.effects);
}

pub(crate) fn current_text_window_cluster_tail(
    builder: &GlyphMatrixBuilder,
) -> Option<(char, bool)> {
    builder.last_text_cluster_tail()
}

pub(crate) fn finish_text_window_output_rows(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &WindowOutputEmitter,
) {
    for metric in output_emitter.row_metrics() {
        builder.set_row_metrics(metric.row, metric.pixel_y, metric.height, metric.ascent);
    }
    builder.end_row();
}

pub(crate) fn install_text_window_right_edge_markers(
    builder: &mut GlyphMatrixBuilder,
    request: TextWindowRightEdgeMarkers<'_>,
) {
    let target_col = request.column.target_col(request.matrix_cols);
    for row_idx in 0..request.row_flags.len() {
        let matrix_row = request.text_matrix_row_base + row_idx;
        if request
            .row_flags
            .is_set(row_idx, DisplayRowFlagKind::Truncated)
        {
            builder.overwrite_current_window_row_glyph_at_col(
                matrix_row,
                target_col,
                '$',
                request.face_id,
            );
        } else if request
            .row_flags
            .is_set(row_idx, DisplayRowFlagKind::Continued)
        {
            builder.overwrite_current_window_row_glyph_at_col(
                matrix_row,
                target_col,
                '\\',
                request.face_id,
            );
        }
    }
}

pub(crate) trait DisplayProgressSink {
    #[cfg(test)]
    fn emit_text_progress(
        &mut self,
        evaluator: &mut Context,
        output: TextRowOutput,
        progress: &DisplayRowAppendProgress,
    );

    fn begin_chrome_progress(&mut self, evaluator: &mut Context, output: ChromeRowOutput);

    fn emit_chrome_progress(
        &mut self,
        evaluator: &mut Context,
        output: ChromeRowOutput,
        progress: DisplayRowOutputProgress,
    );

    fn finish_chrome_progress(&mut self, progress: DisplayRowOutputProgress);
}

pub(crate) struct WindowOutputEmitter {
    frame_id: neovm_core::window::FrameId,
    window_id: neovm_core::window::WindowId,
    text_row_base: i64,
    text_x: f32,
    window_top: f32,
    logical_cursor: Option<WindowCursorPos>,
    phys_cursor: Option<WindowCursorSnapshot>,
    points: Vec<DisplayPointSnapshot>,
    rows: Vec<DisplayRowSnapshot>,
    row_metrics: Vec<RowMetricsSnapshot>,
    current_row_first_display_pos: Option<LispCharPos1>,
    current_row_last_display_pos: Option<LispCharPos1>,
    current_row_progress: Option<CurrentRowProgress>,
}

impl DisplayProgressSink for WindowOutputEmitter {
    #[cfg(test)]
    fn emit_text_progress(
        &mut self,
        evaluator: &mut Context,
        output: TextRowOutput,
        progress: &DisplayRowAppendProgress,
    ) {
        self.emit_text_source_slots(evaluator, output, &progress.slots, progress.end);
    }

    fn begin_chrome_progress(&mut self, evaluator: &mut Context, output: ChromeRowOutput) {
        self.begin_chrome_row(evaluator, output.row, output.y);
    }

    fn emit_chrome_progress(
        &mut self,
        evaluator: &mut Context,
        output: ChromeRowOutput,
        progress: DisplayRowOutputProgress,
    ) {
        self.move_chrome_output_to(evaluator, output.row, progress);
    }

    fn finish_chrome_progress(&mut self, progress: DisplayRowOutputProgress) {
        self.push_chrome_row_progress(progress);
    }
}

impl WindowOutputEmitter {
    pub(crate) fn emit_text_source_slots(
        &mut self,
        evaluator: &mut Context,
        output: TextRowOutput,
        slots: &[DisplayRowGlyphSlot],
        end: DisplayRowPosition,
    ) {
        let mut emitted = false;
        let mut pending_span: Option<TextOutputSpan> = None;
        for slot in slots {
            let Some(span) = output.span_for_buffer_slot(slot) else {
                continue;
            };
            emitted = true;
            if let Some(pending) = pending_span.as_mut()
                && pending.can_merge(span)
            {
                pending.merge(span);
                continue;
            }
            if let Some(pending) = pending_span.take() {
                self.emit_text_output_span(evaluator, pending);
            }
            pending_span = Some(span);
        }
        if let Some(pending) = pending_span.take() {
            self.emit_text_output_span(evaluator, pending);
        }
        if !emitted {
            self.move_text_output_to(evaluator, output.row, end.col, output.row_y, end.x_px);
        }
    }

    pub(crate) fn new(
        frame_id: neovm_core::window::FrameId,
        window_id: neovm_core::window::WindowId,
        text_row_base: usize,
        text_x: f32,
        window_top: f32,
    ) -> Self {
        Self {
            frame_id,
            window_id,
            text_row_base: text_row_base as i64,
            text_x,
            window_top,
            logical_cursor: None,
            phys_cursor: None,
            points: Vec::new(),
            rows: Vec::new(),
            row_metrics: Vec::new(),
            current_row_first_display_pos: None,
            current_row_last_display_pos: None,
            current_row_progress: None,
        }
    }

    pub(crate) fn display_point_len(&self) -> usize {
        self.points.len()
    }

    pub(crate) fn truncate_display_points(&mut self, len: usize) {
        self.points.truncate(len);
    }

    pub(crate) fn rows(&self) -> &[DisplayRowSnapshot] {
        &self.rows
    }

    pub(crate) fn point_for_buffer_pos(&self, pos: LispCharPos1) -> Option<&DisplayPointSnapshot> {
        self.points.iter().find(|point| point.buffer_pos == pos)
    }

    pub(crate) fn point_for_lisp_buffer_pos(
        &self,
        pos: LispCharPos1,
    ) -> Option<&DisplayPointSnapshot> {
        self.point_for_buffer_pos(pos)
    }

    pub(crate) fn row_metrics(&self) -> &[RowMetricsSnapshot] {
        &self.row_metrics
    }

    pub(crate) fn current_row_display_positions(
        &self,
    ) -> (Option<LispCharPos1>, Option<LispCharPos1>) {
        (
            self.current_row_first_display_pos,
            self.current_row_last_display_pos,
        )
    }

    pub(crate) fn restore_current_row_display_positions(
        &mut self,
        first: Option<LispCharPos1>,
        last: Option<LispCharPos1>,
    ) {
        self.current_row_first_display_pos = first;
        self.current_row_last_display_pos = last;
    }

    pub(crate) fn current_row_has_output(&self) -> bool {
        self.current_row_progress.as_ref().is_some_and(|progress| {
            progress.x != progress.start_x
                || progress.col != progress.start_col
                || self.current_row_first_display_pos.is_some()
                || self.current_row_last_display_pos.is_some()
        })
    }

    fn begin_current_row_progress(&mut self, row: i64, col: i64, y: i64, x: i64) {
        self.current_row_progress = Some(CurrentRowProgress {
            row,
            y,
            col,
            x,
            start_col: col,
            start_x: x,
        });
    }

    fn update_current_row_progress(&mut self, row: i64, col: i64, y: i64, x: i64) {
        match self.current_row_progress.as_mut() {
            Some(progress) if progress.row == row => {
                progress.y = y;
                progress.col = col;
                progress.x = x;
            }
            _ => self.begin_current_row_progress(row, col, y, x),
        }
    }

    fn with_live_update<T>(
        &self,
        evaluator: &mut Context,
        f: impl FnOnce(&mut neovm_core::window::WindowOutputUpdate<'_>) -> T,
    ) -> Option<T> {
        let frame = evaluator.frame_manager_mut().get_mut(self.frame_id)?;
        let mut update = frame.window_output_update(self.window_id)?;
        Some(f(&mut update))
    }

    pub(crate) fn note_display_buffer_pos(&mut self, buffer_pos: LispCharPos1) {
        if self.current_row_first_display_pos.is_none() {
            self.current_row_first_display_pos = Some(buffer_pos);
        }
        self.current_row_last_display_pos = Some(buffer_pos);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_display_point(
        &mut self,
        buffer_pos: LispCharPos1,
        glyph_x: f32,
        glyph_y: f32,
        width: f32,
        height: f32,
        row: i64,
        col: usize,
    ) {
        self.note_display_buffer_pos(buffer_pos);
        self.points.push(DisplayPointSnapshot {
            buffer_pos,
            x: (glyph_x - self.text_x).round() as i64,
            y: (glyph_y - self.window_top).round() as i64,
            width: width.max(0.0).round() as i64,
            height: height.max(1.0).round() as i64,
            row,
            col: col as i64,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_text_display_point(
        &mut self,
        buffer_pos: LispCharPos1,
        glyph_x: f32,
        glyph_y: f32,
        width: f32,
        height: f32,
        row: usize,
        col: usize,
    ) {
        self.push_display_point(
            buffer_pos,
            glyph_x,
            glyph_y,
            width,
            height,
            self.text_row_base + row as i64,
            col,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn emit_text_span(
        &mut self,
        evaluator: &mut Context,
        buffer_pos: LispCharPos1,
        row: usize,
        row_y: f32,
        glyph_x: f32,
        glyph_y: f32,
        width: f32,
        height: f32,
        start_col: usize,
        end_col: usize,
    ) {
        self.push_text_display_point(buffer_pos, glyph_x, glyph_y, width, height, row, start_col);
        self.move_text_output_to(evaluator, row, end_col, row_y, glyph_x + width.max(0.0));
    }

    fn emit_text_output_span(&mut self, evaluator: &mut Context, span: TextOutputSpan) {
        self.emit_text_span(
            evaluator,
            span.buffer_pos,
            span.row,
            span.row_y,
            span.start.x_px,
            span.glyph_y,
            span.end.x_px - span.start.x_px,
            span.height,
            span.start.col,
            span.end.col,
        );
    }

    pub(crate) fn begin_row_output(
        &mut self,
        evaluator: &mut Context,
        row: i64,
        col: i64,
        y: i64,
        x: i64,
    ) {
        self.begin_current_row_progress(row, col, y, x);
        let _ = self.with_live_update(evaluator, |update| {
            update.output_cursor_to_coords(row, col, y, x)
        });
    }

    pub(crate) fn begin_text_row(
        &mut self,
        evaluator: &mut Context,
        row: usize,
        col: usize,
        y: f32,
        x: f32,
    ) {
        self.begin_row_output(
            evaluator,
            self.text_row_base + row as i64,
            col as i64,
            (y - self.window_top).round() as i64,
            (x - self.text_x).round() as i64,
        );
    }

    fn begin_chrome_row(&mut self, evaluator: &mut Context, row: i64, y: f32) {
        self.begin_row_output(evaluator, row, 0, (y - self.window_top).round() as i64, 0);
    }

    pub(crate) fn begin_update(&self, evaluator: &mut Context) {
        let _ = self.with_live_update(evaluator, |update| update.begin_update());
    }

    pub(crate) fn move_output_to(
        &mut self,
        evaluator: &mut Context,
        row: i64,
        col: i64,
        y: i64,
        x: i64,
    ) {
        self.update_current_row_progress(row, col, y, x);
        let _ = self.with_live_update(evaluator, |update| {
            update.output_cursor_to_coords(row, col, y, x)
        });
    }

    pub(crate) fn move_text_output_to(
        &mut self,
        evaluator: &mut Context,
        row: usize,
        col: usize,
        y: f32,
        x: f32,
    ) {
        self.move_output_to(
            evaluator,
            self.text_row_base + row as i64,
            col as i64,
            (y - self.window_top).round() as i64,
            (x - self.text_x).round() as i64,
        );
    }

    fn move_chrome_output_to(
        &mut self,
        evaluator: &mut Context,
        row: i64,
        progress: DisplayRowOutputProgress,
    ) {
        self.move_output_to(
            evaluator,
            row,
            progress.end_col,
            (progress.y - self.window_top).round() as i64,
            progress.end_x.round() as i64,
        );
    }

    pub(crate) fn push_text_row(&mut self, row_y_start: f32, row_height: f32, row_ascent: f32) {
        let row_progress = self
            .current_row_progress
            .take()
            .expect("text row must have live output progress before finishing");
        self.rows.push(DisplayRowSnapshot {
            row: row_progress.row,
            y: row_progress.y,
            height: row_height.max(1.0).round() as i64,
            start_x: row_progress.start_x,
            start_col: row_progress.start_col,
            end_x: row_progress.x,
            end_col: row_progress.col,
            start_buffer_pos: self.current_row_first_display_pos.take(),
            end_buffer_pos: self.current_row_last_display_pos.take(),
        });
        self.row_metrics.push(RowMetricsSnapshot {
            row: row_progress.row.max(0) as usize,
            pixel_y: row_y_start,
            height: row_height.max(1.0),
            ascent: row_ascent.max(0.0).min(row_height.max(1.0)),
        });
    }

    fn push_chrome_row(&mut self, row: DisplayRowSnapshot) {
        self.rows.push(row);
    }

    fn push_chrome_row_progress(&mut self, progress: DisplayRowOutputProgress) {
        let row_progress = self
            .current_row_progress
            .take()
            .expect("chrome row must have live output progress before finishing");
        self.push_chrome_row(DisplayRowSnapshot {
            row: row_progress.row,
            y: row_progress.y,
            height: progress.height.round() as i64,
            start_x: row_progress.start_x,
            start_col: row_progress.start_col,
            end_x: row_progress.x,
            end_col: row_progress.col,
            start_buffer_pos: None,
            end_buffer_pos: None,
        });
    }

    pub(crate) fn set_logical_cursor(&mut self, cursor: WindowCursorPos) {
        self.logical_cursor = Some(cursor);
    }

    pub(crate) fn set_phys_cursor(&mut self, cursor: WindowCursorSnapshot) {
        self.phys_cursor = Some(cursor);
    }

    pub(crate) fn finish_snapshot(
        mut self,
        evaluator: &mut Context,
        text_area_left_offset: i64,
        mode_line_height: i64,
        header_line_height: i64,
        tab_line_height: i64,
    ) -> WindowDisplaySnapshot {
        let frame_id = self.frame_id;
        let window_id = self.window_id;
        let logical_cursor = self.logical_cursor.take();
        let phys_cursor = self.phys_cursor.take();
        self.points
            .sort_by_key(|point| (point.buffer_pos, point.row, point.col, point.x));
        self.rows.sort_by_key(|row| row.row);
        let snapshot = WindowDisplaySnapshot {
            window_id,
            text_area_left_offset,
            mode_line_height,
            header_line_height,
            tab_line_height,
            logical_cursor,
            phys_cursor: phys_cursor.clone(),
            points: self.points,
            rows: self.rows,
        };
        if let Some(frame) = evaluator.frame_manager_mut().get_mut(frame_id)
            && let Some(mut update) = frame.window_output_update(window_id)
        {
            update.finalize_live_update(logical_cursor, phys_cursor);
        }
        snapshot
    }
}

#[cfg(test)]
#[path = "window_output_test.rs"]
mod tests;
