//! Live window-output emission helpers for Rust redisplay.
//!
//! This layer bridges Rust layout/status-line emission to GNU-like live window
//! output state. It advances live output through explicit output-cursor moves
//! while simultaneously recording immutable row snapshots for renderer
//! handoff.

use super::display_status_line::{
    ChromeRowRenderServices, DisplayRowOutputProgress, WindowChromeRowsRenderRequest,
    WindowChromeRowsRenderState,
};
use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_cursor::CursorVisualColumnResolutionRequest;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLength, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_row::{MeasuredDisplayRow, RenderedDisplayRowMedia};
#[cfg(test)]
use crate::display_row_builder::DisplayRowAppendProgress;
use crate::display_row_builder::{DisplayRowGlyphSlot, DisplayRowPosition};
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryState, DisplayRowLimit, DisplayRowYPositions,
};
use crate::display_row_matrix_install::{
    DisplayRowArtifactInstallSurface, DisplayRowCurrentRowSurface, DisplayRowDecorationSurface,
    DisplayRowFaceInstallSurface, DisplayRowInstallSurface, DisplayRowLifecycleSurface,
    DisplayRowWindowContextSurface,
};
use crate::display_row_special_glyphs::{
    RightBorderRowsDecorator, RightEdgeMarkerRowDecorator,
    text_window_right_edge_marker_decorations,
};
use crate::display_row_walk_state::HitRowRangeTracker;
use crate::display_source::{DisplayItemSource, DisplaySourceContext};
use crate::hit_test::HitRow;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor,
};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::window::{
    DisplayPointSnapshot, DisplayRowSnapshot, WindowCursorKind, WindowCursorPos,
    WindowCursorSnapshot, WindowDisplaySnapshot,
};

const LINE_NUMBER_MARGIN_SOURCE_ID: u64 = 0x6c6e_756d;

#[derive(Clone, Copy, Debug)]
pub(crate) struct RowMetricsSnapshot {
    pub(crate) matrix_row: usize,
    pub(crate) row: usize,
    pub(crate) pixel_y: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug)]
struct CurrentRowProgress {
    matrix_row: Option<usize>,
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
pub(crate) struct DisplayTextRowMetrics {
    pub(crate) y: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayTextRowBegin {
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
    pub(crate) first_row: DisplayTextRowBegin,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowMatrixBegin {
    pub(crate) window_id: u64,
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    pub(crate) bounds: Rect,
    pub(crate) text_bounds: Rect,
    pub(crate) selected: bool,
}

impl From<TextWindowBegin> for TextWindowMatrixBegin {
    fn from(request: TextWindowBegin) -> Self {
        Self {
            window_id: request.window_id,
            rows: request.rows,
            cols: request.cols,
            bounds: request.bounds,
            text_bounds: request.text_bounds,
            selected: request.selected,
        }
    }
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
    pub(crate) fn target_col(self, matrix_cols: usize) -> usize {
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
    pub(crate) char_width: f32,
}

impl<'a> TextWindowRightEdgeMarkers<'a> {
    pub(crate) fn for_reserved_special_column(
        reserve_right_special_col: bool,
        reserve_right_border_col: bool,
        text_matrix_row_base: usize,
        matrix_cols: usize,
        row_flags: &'a DisplayRowFlags,
        face_id: u32,
        char_width: f32,
    ) -> Option<Self> {
        reserve_right_special_col.then_some(Self {
            text_matrix_row_base,
            matrix_cols,
            column: if reserve_right_border_col {
                TextWindowRightEdgeMarkerColumn::BeforeRightBorder
            } else {
                TextWindowRightEdgeMarkerColumn::LastColumn
            },
            row_flags,
            face_id,
            char_width,
        })
    }
}

pub(crate) struct TextWindowPendingRowFinish<'a> {
    pub(crate) row_geometry: &'a DisplayRowGeometryState,
    pub(crate) row_limit: DisplayRowLimit,
    pub(crate) row_y_positions: &'a DisplayRowYPositions,
    pub(crate) text_y: f32,
    pub(crate) char_height: f32,
    pub(crate) charpos: i64,
    pub(crate) hit_row_range: &'a mut HitRowRangeTracker,
    pub(crate) hit_rows: &'a mut Vec<HitRow>,
}

pub(crate) struct TextWindowOutputInstall;

pub(crate) struct TextWindowMatrixOutputSurface<'builder> {
    builder: &'builder mut GlyphMatrixBuilder,
}

impl<'builder> TextWindowMatrixOutputSurface<'builder> {
    pub(crate) fn from_builder(builder: &'builder mut GlyphMatrixBuilder) -> Self {
        Self { builder }
    }

    pub(crate) fn begin_text_window_matrix(&mut self, request: TextWindowMatrixBegin) {
        self.builder.window_installer().begin(
            request.window_id,
            request.rows,
            request.cols,
            request.bounds,
            request.text_bounds,
            request.selected,
        );
    }

    pub(crate) fn record_display_range(&mut self, range: TextWindowDisplayRange) {
        if let Some(info) = self.builder.window_infos_last_mut()
            && info.window_id == range.window_id as i64
        {
            info.window_start = range.window_start.as_i64();
            info.window_end = range.window_end.as_i64();
        }
    }

    fn record_redisplay_positions(
        &mut self,
        window_id: u64,
        positions: TextWindowRedisplayPositions,
    ) {
        self.record_display_range(positions.display_range(window_id));
    }

    pub(crate) fn install_row_decoration(&mut self, request: TextWindowRowDecorationRequest) {
        let mut row_lifecycle = DisplayRowLifecycleSurface::from_builder(self.builder);
        request.install(&mut row_lifecycle);
    }

    fn begin_display_text_row(&mut self, begin: DisplayTextRowBegin) -> usize {
        DisplayRowLifecycleSurface::from_builder(self.builder).begin_row(
            begin.matrix_row,
            GlyphRowRole::Text,
            false,
        );
        begin.matrix_row
    }

    fn finish_text_matrix_row(
        &mut self,
        matrix_row: usize,
        metrics: DisplayTextRowMetrics,
    ) -> DisplayTextRowFinish {
        let matrix_metrics = self.display_text_row_metrics(metrics);
        DisplayRowLifecycleSurface::from_builder(self.builder).set_metrics(
            matrix_row,
            matrix_metrics.pixel_y,
            matrix_metrics.height_px,
            matrix_metrics.ascent_px,
        );
        DisplayTextRowFinish {
            matrix_row,
            metrics: matrix_metrics,
        }
    }

    fn finalize_text_matrix_row(&mut self, matrix_row: usize) {
        DisplayRowLifecycleSurface::from_builder(self.builder).finalize_row(matrix_row);
    }

    pub(crate) fn close_text_window_output(&mut self) {
        self.builder.window_installer().end();
    }

    pub(crate) fn capture_retry_checkpoint(&self) -> TextWindowOutputRetryCheckpoint {
        TextWindowOutputRetryCheckpoint {
            transition_hints_len: self.builder.transition_hints().len(),
            effect_hints_len: self.builder.effect_hints().len(),
        }
    }

    pub(crate) fn restore_retry_checkpoint(&mut self, checkpoint: TextWindowOutputRetryCheckpoint) {
        self.builder
            .truncate_transition_hints(checkpoint.transition_hints_len);
        self.builder
            .truncate_effect_hints(checkpoint.effect_hints_len);
    }

    fn display_text_row_metrics(
        &self,
        metrics: DisplayTextRowMetrics,
    ) -> DisplayTextRowStoredMetrics {
        let window_y = DisplayRowWindowContextSurface::from_builder(self.builder)
            .current_window_pixel_bounds()
            .y;
        DisplayTextRowStoredMetrics {
            pixel_y: metrics.y - window_y,
            height_px: metrics.height,
            ascent_px: metrics.ascent,
        }
    }

    fn finish_output_rows(&mut self, output_emitter: &WindowOutputEmitter) {
        let window_y = DisplayRowWindowContextSurface::from_builder(self.builder)
            .current_window_pixel_bounds()
            .y;
        for metric in output_emitter.row_metrics() {
            DisplayRowLifecycleSurface::from_builder(self.builder).set_metrics(
                metric.matrix_row,
                metric.pixel_y - window_y,
                metric.height,
                metric.ascent,
            );
        }
        if let Some(metric) = output_emitter.row_metrics().last() {
            DisplayRowLifecycleSurface::from_builder(self.builder).finalize_row(metric.matrix_row);
        }
    }

    fn install_right_edge_markers(
        &mut self,
        mut render_services: ChromeRowRenderServices<'_, '_>,
        request: TextWindowRightEdgeMarkers<'_>,
    ) {
        let base_face = render_services.face_resolver().default_face().clone();
        let mut decorations = DisplayRowDecorationSurface::from_builder(self.builder);
        for decoration in text_window_right_edge_marker_decorations(&request) {
            decorations.decorate_current_window_row(
                decoration.matrix_row,
                RightEdgeMarkerRowDecorator::new(
                    decoration,
                    request.face_id,
                    &base_face,
                    request.char_width,
                    &mut render_services,
                ),
            );
        }
    }

    fn install_last_window_right_border(
        &mut self,
        mut render_services: ChromeRowRenderServices<'_, '_>,
        request: TextWindowRightBorder,
        base_face: &ResolvedFace,
    ) {
        DisplayRowDecorationSurface::from_builder(self.builder).decorate_last_window_rows(
            RightBorderRowsDecorator::new(request, base_face, &mut render_services),
        );
    }
}

pub(crate) struct TextWindowBodyOutputInstall<'a> {
    pub(crate) window_id: u64,
    pub(crate) window_start: i64,
    pub(crate) text_start_byte: usize,
    pub(crate) byte_idx: usize,
    pub(crate) right_edge_markers: Option<TextWindowRightEdgeMarkers<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextWindowRedisplayPositions {
    pub(crate) window_start: LispCharPos1,
    pub(crate) window_end: LispCharPos1,
    pub(crate) window_end_byte: EmacsBytePos,
    pub(crate) window_end_vpos: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowRightBorder {
    pub(crate) ch: char,
    pub(crate) face_id: u32,
    pub(crate) char_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowTerminalRightBorder {
    pub(crate) ch: char,
    pub(crate) face_name: &'static str,
    pub(crate) char_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextWindowLineNumberMargin<'a> {
    pub(crate) text: &'a str,
    pub(crate) cols: i32,
    pub(crate) face_id: u32,
}

pub(crate) enum TextWindowRowDecorationRequest {
    MarkCurrentTruncatedLeft,
}

impl TextWindowRowDecorationRequest {
    fn install(self, row_lifecycle: &mut DisplayRowLifecycleSurface<'_>) {
        match self {
            Self::MarkCurrentTruncatedLeft => {
                row_lifecycle.mark_current_truncated_left();
            }
        }
    }
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
    pub(crate) glyph_row_resolved: bool,
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
pub(crate) struct DisplayTextRowGeometryTransition {
    pub(crate) finished_row: DisplayTextRowMetrics,
    pub(crate) begin_row: DisplayTextRowBegin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayTextRowTransition {
    BeganNextRow,
    ExhaustedRows,
}

impl DisplayTextRowTransition {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayTextRowFinish {
    pub(crate) matrix_row: usize,
    pub(crate) metrics: DisplayTextRowStoredMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayTextRowStoredMetrics {
    pub(crate) pixel_y: f32,
    pub(crate) height_px: f32,
    pub(crate) ascent_px: f32,
}

pub(crate) struct TextWindowLiveCurrentRowEvaluatorState<'a> {
    pub(crate) row_surface: DisplayRowCurrentRowSurface<'a>,
    pub(crate) evaluator: &'a mut Context,
}

pub(crate) struct TextWindowLiveCurrentRowHostState<'a> {
    pub(crate) row_surface: DisplayRowCurrentRowSurface<'a>,
    pub(crate) display_host: Option<&'a dyn DisplayHost>,
}

pub(crate) struct TextWindowBeginOutputSurface<'a> {
    builder: &'a mut GlyphMatrixBuilder,
    evaluator: &'a mut Context,
}

pub(crate) struct TextWindowFinishOutputSurface<'a> {
    builder: &'a mut GlyphMatrixBuilder,
    output_emitter: WindowOutputEmitter,
    evaluator: &'a mut Context,
}

pub(crate) struct TextWindowLiveOutputSurface<'a> {
    builder: &'a mut GlyphMatrixBuilder,
    output_emitter: &'a mut WindowOutputEmitter,
    evaluator: &'a mut Context,
}

impl<'a> TextWindowBeginOutputSurface<'a> {
    pub(crate) fn from_builder(
        builder: &'a mut GlyphMatrixBuilder,
        evaluator: &'a mut Context,
    ) -> Self {
        Self { builder, evaluator }
    }

    pub(crate) fn begin_update(&mut self, output_emitter: &mut WindowOutputEmitter) {
        output_emitter.begin_update(self.evaluator);
    }

    pub(crate) fn begin_text_window_output(
        &mut self,
        output_emitter: &mut WindowOutputEmitter,
        begin: TextWindowBegin,
    ) {
        TextWindowRowOutputSurface::from_parts(self.builder, output_emitter)
            .begin_text_window_output(self.evaluator, begin);
    }
}

impl<'a> TextWindowFinishOutputSurface<'a> {
    pub(crate) fn from_builder(
        builder: &'a mut GlyphMatrixBuilder,
        output_emitter: WindowOutputEmitter,
        evaluator: &'a mut Context,
    ) -> Self {
        Self {
            builder,
            output_emitter,
            evaluator,
        }
    }

    pub(crate) fn finish_snapshot(
        self,
        text_area_left_offset: i64,
        mode_line_height: i64,
        header_line_height: i64,
        tab_line_height: i64,
    ) -> WindowDisplaySnapshot {
        TextWindowMatrixOutputSurface::from_builder(self.builder).close_text_window_output();
        self.output_emitter.finish_snapshot(
            self.evaluator,
            text_area_left_offset,
            mode_line_height,
            header_line_height,
            tab_line_height,
        )
    }
}

impl<'a> TextWindowLiveOutputSurface<'a> {
    pub(crate) fn from_builder(
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

    pub(crate) fn reborrow(&mut self) -> TextWindowLiveOutputSurface<'_> {
        TextWindowLiveOutputSurface {
            builder: self.builder,
            output_emitter: self.output_emitter,
            evaluator: self.evaluator,
        }
    }

    pub(crate) fn with_text_window_output<R>(
        self,
        f: impl FnOnce(&mut TextWindowRowOutputSurface<'_, '_>, &mut Context) -> R,
    ) -> R {
        let mut output = TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter);
        f(&mut output, self.evaluator)
    }

    #[cfg(test)]
    pub(crate) fn begin_text_row(self, begin: DisplayTextRowBegin) -> usize {
        TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter)
            .begin_text_row(self.evaluator, begin)
    }

    #[cfg(test)]
    pub(crate) fn finish_text_row(self, metrics: DisplayTextRowMetrics) {
        TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter)
            .finish_text_row(metrics);
    }

    pub(crate) fn finish_and_end_text_row(self, metrics: DisplayTextRowMetrics) {
        TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter)
            .finish_and_end_text_row(metrics);
    }

    pub(crate) fn transition_text_row(self, transition: DisplayTextRowGeometryTransition) {
        TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter)
            .transition_text_row(self.evaluator, transition);
    }

    pub(crate) fn transition_text_row_with_limit(
        self,
        transition: DisplayTextRowGeometryTransition,
        max_rows: usize,
    ) -> DisplayTextRowTransition {
        TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter)
            .transition_text_row_with_limit(self.evaluator, transition, max_rows)
    }

    pub(crate) fn install_row_decoration(self, request: TextWindowRowDecorationRequest) {
        TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter)
            .install_row_decoration(request);
    }

    pub(crate) fn install_resolved_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<crate::font_metrics::FontMetrics>,
    ) {
        DisplayRowFaceInstallSurface::from_builder(self.builder)
            .install_resolved_face(face_id, face, metrics);
    }

    pub(crate) fn install_rendered_fragment_assets(
        &mut self,
        role: GlyphRowRole,
        matrix_row: usize,
        faces: &[neomacs_display_protocol::face::Face],
        media: &[RenderedDisplayRowMedia],
    ) {
        DisplayRowInstallSurface::from_builder(self.builder)
            .install_fragment_assets(role, matrix_row, faces, media);
    }

    pub(crate) fn emit_text_source_slots(
        &mut self,
        output: TextRowOutput,
        source_slots: &[DisplayRowGlyphSlot],
        end: DisplayRowPosition,
    ) {
        self.output_emitter
            .emit_text_source_slots(self.evaluator, output, source_slots, end);
    }

    pub(crate) fn install_body_output(
        &mut self,
        request: TextWindowBodyOutputInstall<'_>,
        render_services: Option<ChromeRowRenderServices<'_, '_>>,
    ) -> TextWindowRedisplayPositions {
        TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter)
            .install_body_output(request, render_services)
    }

    pub(crate) fn render_chrome_rows(
        &mut self,
        request: WindowChromeRowsRenderRequest<'_, '_>,
        render_services: ChromeRowRenderServices<'_, '_>,
    ) {
        TextWindowRowOutputSurface::from_parts(self.builder, self.output_emitter)
            .render_chrome_rows(self.evaluator, request, render_services);
    }

    pub(crate) fn current_row_evaluator_state(
        &mut self,
    ) -> TextWindowLiveCurrentRowEvaluatorState<'_> {
        TextWindowLiveCurrentRowEvaluatorState {
            row_surface: DisplayRowCurrentRowSurface::from_builder(self.builder),
            evaluator: self.evaluator,
        }
    }

    pub(crate) fn current_row_host_state(&mut self) -> TextWindowLiveCurrentRowHostState<'_> {
        TextWindowLiveCurrentRowHostState {
            row_surface: DisplayRowCurrentRowSurface::from_builder(self.builder),
            display_host: self.evaluator.display_host.as_deref(),
        }
    }

    pub(crate) fn display_host(&self) -> Option<&dyn DisplayHost> {
        self.evaluator.display_host.as_deref()
    }

    pub(crate) fn with_evaluator<R>(&mut self, f: impl FnOnce(&mut Context) -> R) -> R {
        f(self.evaluator)
    }

    pub(crate) fn output_emitter(&mut self) -> &mut WindowOutputEmitter {
        self.output_emitter
    }

    pub(crate) fn output_rows(&self) -> &[DisplayRowSnapshot] {
        self.output_emitter.rows()
    }

    pub(crate) fn output_rows_len(&self) -> usize {
        self.output_emitter.rows().len()
    }
}

impl TextWindowRedisplayPositions {
    pub(crate) fn from_output_rows(
        output_emitter: &WindowOutputEmitter,
        window_start: i64,
        text_start_byte: usize,
        byte_idx: usize,
    ) -> Self {
        let window_start = layout_i64_char_pos_to_lisp_char_pos(window_start);
        let window_end = output_emitter
            .rows()
            .iter()
            .rev()
            .find_map(|row| row.end_buffer_pos)
            .map(|pos| layout_i64_char_pos_to_lisp_char_pos(pos.as_i64()))
            .unwrap_or_else(|| LispCharPos1::from_one_based_usize(1));
        let window_end_vpos = output_emitter
            .rows()
            .last()
            .map(|row| row.row.max(0) as usize)
            .unwrap_or(0);

        Self {
            window_start,
            window_end,
            window_end_byte: EmacsBytePos::new(text_start_byte.saturating_add(byte_idx)),
            window_end_vpos,
        }
    }

    pub(crate) fn display_range(self, window_id: u64) -> TextWindowDisplayRange {
        TextWindowDisplayRange {
            window_id,
            window_start: self.window_start,
            window_end: self.window_end,
        }
    }
}

fn line_number_margin_text_item(text: &str, face_id: u32, start_offset: usize) -> DisplayItem {
    let end_offset = start_offset.saturating_add(text.chars().count());
    DisplayItem::new(
        SourceSpan::synthetic(LINE_NUMBER_MARGIN_SOURCE_ID, start_offset, end_offset),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text.to_owned())),
    )
}

fn line_number_margin_stretch_item(cols: u16, face_id: u32, start_offset: usize) -> DisplayItem {
    DisplayItem::new(
        SourceSpan::synthetic(
            LINE_NUMBER_MARGIN_SOURCE_ID,
            start_offset,
            start_offset.saturating_add(usize::from(cols)),
        ),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Columns(cols)),
            height: None,
            ascent: None,
        }),
    )
}

pub(crate) struct LineNumberMarginItemSource {
    items: std::vec::IntoIter<DisplayItem>,
}

impl LineNumberMarginItemSource {
    pub(crate) fn new(request: &TextWindowLineNumberMargin<'_>) -> Self {
        let mut items = Vec::new();
        let mut source_offset = 0usize;
        let padding = (request.cols - 1) - request.text.chars().count() as i32;
        if padding > 0 {
            let cols = padding.min(i32::from(u16::MAX)) as u16;
            items.push(line_number_margin_stretch_item(
                cols,
                request.face_id,
                source_offset,
            ));
            source_offset = source_offset.saturating_add(usize::from(cols));
        }
        if !request.text.is_empty() {
            items.push(line_number_margin_text_item(
                request.text,
                request.face_id,
                source_offset,
            ));
            source_offset = source_offset.saturating_add(request.text.chars().count());
        }
        items.push(line_number_margin_stretch_item(
            1,
            request.face_id,
            source_offset,
        ));
        Self {
            items: items.into_iter(),
        }
    }
}

impl DisplayItemSource for LineNumberMarginItemSource {
    fn next_item(&mut self, _context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.items.next()
    }
}

pub(crate) struct TextWindowRowOutputSurface<'builder, 'output> {
    builder: &'builder mut GlyphMatrixBuilder,
    output_emitter: &'output mut WindowOutputEmitter,
}

pub(crate) struct TextWindowArtifactOutputSurface<'builder> {
    builder: &'builder mut GlyphMatrixBuilder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextWindowOutputRetryCheckpoint {
    transition_hints_len: usize,
    effect_hints_len: usize,
}

impl<'builder, 'output> TextWindowRowOutputSurface<'builder, 'output> {
    pub(crate) fn from_parts(
        builder: &'builder mut GlyphMatrixBuilder,
        output_emitter: &'output mut WindowOutputEmitter,
    ) -> Self {
        Self {
            builder,
            output_emitter,
        }
    }

    fn output_emitter(&self) -> &WindowOutputEmitter {
        self.output_emitter
    }

    fn output_emitter_mut(&mut self) -> &mut WindowOutputEmitter {
        self.output_emitter
    }

    pub(crate) fn set_logical_cursor(&mut self, cursor: WindowCursorPos) {
        self.output_emitter_mut().set_logical_cursor(cursor);
    }

    pub(crate) fn row_metrics(&self) -> &[RowMetricsSnapshot] {
        self.output_emitter().row_metrics()
    }

    pub(crate) fn point_for_lisp_buffer_pos(
        &self,
        pos: LispCharPos1,
    ) -> Option<&DisplayPointSnapshot> {
        self.output_emitter().point_for_lisp_buffer_pos(pos)
    }

    pub(crate) fn begin_text_row(
        &mut self,
        evaluator: &mut Context,
        begin: DisplayTextRowBegin,
    ) -> usize {
        let matrix_row =
            TextWindowMatrixOutputSurface::from_builder(self.builder).begin_display_text_row(begin);
        self.output_emitter.begin_display_text_row(
            evaluator,
            begin.matrix_row,
            begin.row,
            begin.col,
            begin.y,
            begin.x,
        );
        matrix_row
    }

    pub(crate) fn finish_text_row(
        &mut self,
        metrics: DisplayTextRowMetrics,
    ) -> DisplayTextRowFinish {
        let matrix_row = self.output_emitter.current_text_matrix_row();
        let finish = TextWindowMatrixOutputSurface::from_builder(self.builder)
            .finish_text_matrix_row(matrix_row, metrics);
        self.output_emitter
            .push_text_row(metrics.y, metrics.height, metrics.ascent);
        finish
    }

    pub(crate) fn finish_and_end_text_row(
        &mut self,
        metrics: DisplayTextRowMetrics,
    ) -> DisplayTextRowFinish {
        let matrix_row = self.output_emitter.current_text_matrix_row();
        let finish = self.finish_text_row(metrics);
        TextWindowMatrixOutputSurface::from_builder(self.builder)
            .finalize_text_matrix_row(matrix_row);
        finish
    }

    pub(crate) fn transition_text_row(
        &mut self,
        evaluator: &mut Context,
        transition: DisplayTextRowGeometryTransition,
    ) -> DisplayTextRowTransition {
        self.finish_and_end_text_row(transition.finished_row);
        self.begin_text_row(evaluator, transition.begin_row);
        DisplayTextRowTransition::BeganNextRow
    }

    pub(crate) fn transition_text_row_with_limit(
        &mut self,
        evaluator: &mut Context,
        transition: DisplayTextRowGeometryTransition,
        max_rows: usize,
    ) -> DisplayTextRowTransition {
        if transition.begin_row.row >= max_rows {
            self.finish_and_end_text_row(transition.finished_row);
            return DisplayTextRowTransition::ExhaustedRows;
        }
        self.transition_text_row(evaluator, transition)
    }

    pub(crate) fn install_row_decoration(&mut self, request: TextWindowRowDecorationRequest) {
        TextWindowMatrixOutputSurface::from_builder(self.builder).install_row_decoration(request);
    }

    pub(crate) fn publish_cursor(
        &mut self,
        cursor: TextWindowCursor,
    ) -> TextWindowCursorPublicationOutcome {
        let window_context = DisplayRowWindowContextSurface::from_builder(self.builder);
        let publication = TextWindowCursorPublication::resolve(&window_context, cursor);
        publication.publish(self)
    }

    pub(crate) fn publish_decorative_cursor(&mut self, cursor: TextWindowDecorativeCursor) {
        TextWindowArtifactOutputSurface::from_builder(self.builder)
            .publish_decorative_cursor(cursor);
    }

    fn install_matrix_cursor(&mut self, cursor: TextWindowMatrixCursor) {
        TextWindowArtifactOutputSurface::from_builder(self.builder).install_matrix_cursor(cursor);
    }

    fn set_matrix_cursor(&mut self, row: usize, col: u16, style: CursorStyle) {
        DisplayRowLifecycleSurface::from_builder(self.builder).set_cursor(row, col, style);
    }

    fn store_phys_cursor(&mut self, cursor: PhysCursor) {
        TextWindowArtifactOutputSurface::from_builder(self.builder).store_phys_cursor(cursor);
    }

    fn set_phys_cursor(&mut self, cursor: WindowCursorSnapshot) {
        self.output_emitter_mut().set_phys_cursor(cursor);
    }

    pub(crate) fn install_body_output(
        &mut self,
        request: TextWindowBodyOutputInstall<'_>,
        render_services: Option<ChromeRowRenderServices<'_, '_>>,
    ) -> TextWindowRedisplayPositions {
        TextWindowOutputInstaller::new(self.builder, self.output_emitter)
            .install_body_output(request, render_services)
    }

    pub(crate) fn begin_chrome_progress(
        &mut self,
        evaluator: &mut Context,
        output: ChromeRowOutput,
    ) {
        self.output_emitter_mut()
            .begin_chrome_progress(evaluator, output);
    }

    pub(crate) fn emit_chrome_progress(
        &mut self,
        evaluator: &mut Context,
        output: ChromeRowOutput,
        progress: DisplayRowOutputProgress,
    ) {
        self.output_emitter_mut()
            .emit_chrome_progress(evaluator, output, progress);
    }

    pub(crate) fn finish_chrome_progress(&mut self, progress: DisplayRowOutputProgress) {
        self.output_emitter_mut().finish_chrome_progress(progress);
    }

    pub(crate) fn install_measured_window_chrome_row(&mut self, measured: &MeasuredDisplayRow) {
        DisplayRowInstallSurface::from_builder(self.builder).install_measured(measured);
    }

    pub(crate) fn render_chrome_rows(
        &mut self,
        evaluator: &mut Context,
        request: WindowChromeRowsRenderRequest<'_, '_>,
        render_services: ChromeRowRenderServices<'_, '_>,
    ) {
        request.render(&mut WindowChromeRowsRenderState::new(
            self,
            evaluator,
            render_services,
        ));
    }

    pub(crate) fn begin_text_window_output(
        &mut self,
        evaluator: &mut Context,
        request: TextWindowBegin,
    ) {
        let first_row = request.first_row;
        TextWindowMatrixOutputSurface::from_builder(self.builder)
            .begin_text_window_matrix(request.into());
        self.begin_text_row(evaluator, first_row);
    }

    pub(crate) fn finish_pending_row(
        &mut self,
        _evaluator: &mut Context,
        request: TextWindowPendingRowFinish<'_>,
    ) -> bool {
        let has_pending_row_output = self.output_emitter.current_row_has_output();
        if !request.row_geometry.is_within_row_limit(request.row_limit)
            || !request
                .hit_row_range
                .should_finish_current_row(request.charpos, has_pending_row_output)
        {
            return false;
        }

        let row_y_start = request.row_geometry.current_row_y(
            request.row_y_positions,
            request.text_y,
            request.char_height,
        );
        let row_cursor = request.row_geometry.with_row_y(row_y_start).cursor();
        request
            .hit_rows
            .push(row_cursor.hit_row(request.hit_row_range.start(), request.charpos));
        self.finish_text_row(row_cursor.finish_current_row());
        true
    }
}

impl<'builder> TextWindowArtifactOutputSurface<'builder> {
    pub(crate) fn from_builder(builder: &'builder mut GlyphMatrixBuilder) -> Self {
        Self { builder }
    }

    pub(crate) fn install_cursor_effects(&mut self, request: TextWindowCursorEffects) {
        DisplayRowArtifactInstallSurface::from_builder(self.builder)
            .set_cursor_effects(request.window_id, request.effects);
    }

    pub(crate) fn publish_decorative_cursor(&mut self, cursor: TextWindowDecorativeCursor) {
        if let Some(effects) = cursor.effects {
            self.install_cursor_effects(TextWindowCursorEffects {
                window_id: cursor.window_id,
                effects,
            });
        }
        self.install_matrix_cursor(TextWindowMatrixCursor {
            window_id: cursor.window_id,
            slot_id: cursor.slot_id,
            x: cursor.x,
            y: cursor.y,
            width: cursor.width,
            height: cursor.height,
            style: cursor.style,
            color: cursor.color,
        });
    }

    fn install_matrix_cursor(&mut self, cursor: TextWindowMatrixCursor) {
        DisplayRowArtifactInstallSurface::from_builder(self.builder).add_cursor(
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

    fn store_phys_cursor(&mut self, cursor: PhysCursor) {
        DisplayRowArtifactInstallSurface::from_builder(self.builder).store_phys_cursor(cursor);
    }

    pub(crate) fn install_terminal_right_border(
        &mut self,
        request: TextWindowTerminalRightBorder,
        mut render_services: ChromeRowRenderServices<'_, '_>,
    ) -> u32 {
        let border_face = render_services
            .face_resolver()
            .resolve_named_face(request.face_name);
        // GNU draws every realized face id from the single per-frame face cache
        // counter (`face_cache->used`, xfaces.c `lookup_face`). Allocate the
        // border's id from the frame-scoped allocator (reconciled into
        // `frame_face_id_counter` by the decoration render, engine.rs) rather than
        // a separate `FaceResolver` counter that could collide with it.
        let border_face_id = render_services.face_ids().allocate();
        DisplayRowFaceInstallSurface::from_builder(self.builder).install_resolved_face(
            border_face_id,
            &border_face,
            None,
        );
        TextWindowMatrixOutputSurface::from_builder(self.builder).install_last_window_right_border(
            render_services.reborrow(),
            TextWindowRightBorder {
                ch: request.ch,
                face_id: border_face_id,
                char_width: request.char_width,
            },
            &border_face,
        );
        border_face_id
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextWindowCursorPublication {
    matrix_cursor: Option<TextWindowMatrixCursor>,
    row: usize,
    row_col: u16,
    style: CursorStyle,
    live_cursor: WindowCursorSnapshot,
    selected_phys_cursor: Option<PhysCursor>,
}

#[derive(Clone, Debug, PartialEq)]
struct TextWindowMatrixCursor {
    window_id: i64,
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: CursorStyle,
    color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextWindowCursorPublicationOutcome {
    pub(crate) installed_matrix_cursor: bool,
    pub(crate) stored_phys_cursor: bool,
    pub(crate) row: usize,
    pub(crate) row_col: u16,
    pub(crate) live_cursor: WindowCursorSnapshot,
}

impl TextWindowCursorPublication {
    fn resolve(
        window_context: &DisplayRowWindowContextSurface<'_>,
        cursor: TextWindowCursor,
    ) -> Self {
        let matrix_cursor = (!cursor.selected).then_some(TextWindowMatrixCursor {
            window_id: cursor.window_id,
            slot_id: cursor.slot_id,
            x: cursor.x,
            y: cursor.y,
            width: cursor.width,
            height: cursor.height,
            style: cursor.style,
            color: cursor.color,
        });
        let mut phys_cursor = cursor.phys_cursor();
        let row_col = if cursor.selected && !cursor.glyph_row_resolved {
            if let Some(placement) = CursorVisualColumnResolutionRequest::from_cursor(&phys_cursor)
                .resolve_phys_cursor_placement(window_context.cursor_visual_column_context())
            {
                placement.apply_to(&mut phys_cursor);
            }
            phys_cursor.col
        } else {
            cursor.col()
        };

        Self {
            matrix_cursor,
            row: cursor.row(),
            row_col,
            style: cursor.style,
            live_cursor: cursor.window_snapshot(),
            selected_phys_cursor: cursor.selected.then_some(phys_cursor),
        }
    }

    fn publish(
        self,
        output: &mut TextWindowRowOutputSurface<'_, '_>,
    ) -> TextWindowCursorPublicationOutcome {
        let installed_matrix_cursor = self.matrix_cursor.is_some();
        if let Some(cursor) = self.matrix_cursor {
            output.install_matrix_cursor(cursor);
        }
        output.set_matrix_cursor(self.row, self.row_col, self.style);
        output.set_phys_cursor(self.live_cursor.clone());
        if let Some(cursor) = self.selected_phys_cursor.clone() {
            output.store_phys_cursor(cursor);
        }
        TextWindowCursorPublicationOutcome {
            installed_matrix_cursor,
            stored_phys_cursor: self.selected_phys_cursor.is_some(),
            row: self.row,
            row_col: self.row_col,
            live_cursor: self.live_cursor,
        }
    }
}

struct TextWindowOutputInstaller<'builder, 'output> {
    matrix_output: TextWindowMatrixOutputSurface<'builder>,
    output_emitter: &'output WindowOutputEmitter,
}

impl<'builder, 'output> TextWindowOutputInstaller<'builder, 'output> {
    fn new(
        builder: &'builder mut GlyphMatrixBuilder,
        output_emitter: &'output WindowOutputEmitter,
    ) -> Self {
        Self {
            matrix_output: TextWindowMatrixOutputSurface::from_builder(builder),
            output_emitter,
        }
    }

    fn install_output(&mut self, _request: TextWindowOutputInstall) {
        self.matrix_output.finish_output_rows(self.output_emitter);
    }

    fn install_body_output(
        &mut self,
        request: TextWindowBodyOutputInstall<'_>,
        render_services: Option<ChromeRowRenderServices<'_, '_>>,
    ) -> TextWindowRedisplayPositions {
        let redisplay_positions = TextWindowRedisplayPositions::from_output_rows(
            self.output_emitter,
            request.window_start,
            request.text_start_byte,
            request.byte_idx,
        );
        self.matrix_output
            .record_redisplay_positions(request.window_id, redisplay_positions);
        self.install_output(TextWindowOutputInstall);
        if let Some(markers) = request.right_edge_markers {
            let render_services =
                render_services.expect("right-edge markers require chrome render services");
            self.matrix_output
                .install_right_edge_markers(render_services, markers);
        }
        redisplay_positions
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

    fn begin_current_row_progress(
        &mut self,
        matrix_row: Option<usize>,
        row: i64,
        col: i64,
        y: i64,
        x: i64,
    ) {
        self.current_row_progress = Some(CurrentRowProgress {
            matrix_row,
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
            _ => self.begin_current_row_progress(None, row, col, y, x),
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
        self.begin_current_row_progress(None, row, col, y, x);
        let _ = self.with_live_update(evaluator, |update| {
            update.output_cursor_to_coords(row, col, y, x)
        });
    }

    pub(crate) fn begin_display_text_row(
        &mut self,
        evaluator: &mut Context,
        matrix_row: usize,
        row: usize,
        col: usize,
        y: f32,
        x: f32,
    ) {
        let output_row = self.text_row_base + row as i64;
        let output_col = col as i64;
        let output_y = (y - self.window_top).round() as i64;
        let output_x = (x - self.text_x).round() as i64;
        self.begin_current_row_progress(
            Some(matrix_row),
            output_row,
            output_col,
            output_y,
            output_x,
        );
        let _ = self.with_live_update(evaluator, |update| {
            update.output_cursor_to_coords(output_row, output_col, output_y, output_x)
        });
    }

    pub(crate) fn current_text_matrix_row(&self) -> usize {
        self.current_row_progress
            .and_then(|progress| progress.matrix_row)
            .expect("text row must have matrix row progress before finishing")
    }

    #[cfg(test)]
    pub(crate) fn begin_text_row(
        &mut self,
        evaluator: &mut Context,
        row: usize,
        col: usize,
        y: f32,
        x: f32,
    ) {
        self.begin_display_text_row(evaluator, self.text_row_base as usize + row, row, col, y, x);
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
            matrix_row: row_progress
                .matrix_row
                .expect("text row must have matrix row progress before recording metrics"),
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
