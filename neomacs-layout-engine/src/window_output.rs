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
use crate::display_current_row_output::DisplayRowCurrentRowOutput;
use crate::display_cursor::CursorVisualColumnResolutionRequest;
use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_rendered_row_output_install::{
    install_measured_window_display_row, install_rendered_display_row_fragment_assets,
};
use crate::display_row::{MeasuredDisplayRow, RenderedDisplayRowMedia};
#[cfg(test)]
use crate::display_row_builder::DisplayRowAppendProgress;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowLimit, DisplayRowYPositions};
use crate::display_row_special_glyphs::{
    TextWindowRightEdgeMarkers, install_text_window_right_edge_markers,
};
use crate::display_row_text_output::{TextOutputSpan, TextRowOutput};
use crate::display_row_walk_state::HitRowRangeTracker;
use crate::display_text_output_install::{
    DisplayOutputCursorArtifactInstallRequest, DisplayOutputRowStoredMetrics,
    DisplayOutputTextRowMetricsInstallRequest, DisplayOutputTextWindowBeginInstallRequest,
    TextWindowRowDecorationRequest, begin_text_output_row, begin_text_output_window,
    end_text_output_window, finalize_text_output_row, finish_text_output_row,
    install_output_resolved_face, install_text_output_cursor_artifact,
    install_text_output_cursor_effects, install_text_output_display_range,
    install_text_output_row_cursor, install_text_output_row_decoration,
    install_text_output_row_metrics, restore_text_output_retry_checkpoint,
    store_text_output_phys_cursor,
};
use crate::hit_test::HitRow;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor,
};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::Context;
use neovm_core::window::{
    DisplayPointSnapshot, DisplayRowSnapshot, WindowCursorKind, WindowCursorPos,
    WindowCursorSnapshot, WindowDisplaySnapshot,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct RowMetricsSnapshot {
    pub(crate) display_row_index: usize,
    pub(crate) row: usize,
    pub(crate) pixel_y: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug)]
struct CurrentRowProgress {
    display_row_index: Option<usize>,
    row: i64,
    y: i64,
    col: i64,
    x: i64,
    start_col: i64,
    start_x: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChromeRowOutput {
    pub(crate) row: i64,
    pub(crate) y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChromeRowProgress {
    pub(crate) output: ChromeRowOutput,
    pub(crate) progress: DisplayRowOutputProgress,
}

impl ChromeRowProgress {
    pub(crate) fn new(output: ChromeRowOutput, progress: DisplayRowOutputProgress) -> Self {
        Self { output, progress }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayTextRowMetrics {
    pub(crate) y: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayTextRowBegin {
    pub(crate) display_row_index: usize,
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
pub(crate) struct TextWindowOutputBegin {
    pub(crate) window_id: u64,
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    pub(crate) bounds: Rect,
    pub(crate) text_bounds: Rect,
    pub(crate) selected: bool,
}

impl From<TextWindowBegin> for TextWindowOutputBegin {
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

impl TextWindowOutputBegin {
    fn into_output_install_request(self) -> DisplayOutputTextWindowBeginInstallRequest {
        DisplayOutputTextWindowBeginInstallRequest::new(
            self.window_id,
            self.rows,
            self.cols,
            self.bounds,
            self.text_bounds,
            self.selected,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextWindowDisplayRange {
    pub(crate) window_id: u64,
    pub(crate) window_start: LispCharPos1,
    pub(crate) window_end: LispCharPos1,
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

pub(crate) struct TextWindowOutputTarget<'a> {
    output_builder: &'a mut DisplayOutputBuilder,
}

impl<'a> TextWindowOutputTarget<'a> {
    pub(crate) fn from_builder(output_builder: &'a mut DisplayOutputBuilder) -> Self {
        Self { output_builder }
    }

    pub(crate) fn reborrow(&mut self) -> TextWindowOutputTarget<'_> {
        TextWindowOutputTarget {
            output_builder: self.output_builder,
        }
    }

    pub(crate) fn builder(&mut self) -> &mut DisplayOutputBuilder {
        self.output_builder
    }

    pub(crate) fn current_row_output(&mut self) -> DisplayRowCurrentRowOutput<'_> {
        DisplayRowCurrentRowOutput::from_output_builder(self.builder())
    }

    pub(crate) fn install_resolved_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<crate::font_metrics::FontMetrics>,
    ) {
        install_output_resolved_face(self.builder(), face_id, face, metrics);
    }

    pub(crate) fn install_rendered_fragment_assets(
        &mut self,
        role: GlyphRowRole,
        display_row_index: usize,
        faces: &[Face],
        media: &[RenderedDisplayRowMedia],
    ) {
        install_rendered_display_row_fragment_assets(
            self.builder(),
            role,
            display_row_index,
            faces,
            media,
        );
    }

    pub(crate) fn install_measured_window_display_row(&mut self, measured: &MeasuredDisplayRow) {
        install_measured_window_display_row(self.builder(), measured);
    }
}

pub(crate) fn begin_text_window_output(
    mut output: TextWindowOutputTarget<'_>,
    request: TextWindowOutputBegin,
) {
    begin_text_output_window(output.builder(), request.into_output_install_request());
}

pub(crate) fn record_text_window_display_range(
    mut output: TextWindowOutputTarget<'_>,
    range: TextWindowDisplayRange,
) {
    install_text_output_display_range(
        output.builder(),
        range.window_id as i64,
        range.window_start.as_i64(),
        range.window_end.as_i64(),
    );
}

fn record_text_window_redisplay_positions(
    output: TextWindowOutputTarget<'_>,
    window_id: u64,
    positions: TextWindowRedisplayPositions,
) {
    record_text_window_display_range(output, positions.display_range(window_id));
}

fn install_text_window_row_decoration(
    output_builder: &mut DisplayOutputBuilder,
    request: TextWindowRowDecorationRequest,
) {
    install_text_output_row_decoration(output_builder, request);
}

pub(crate) fn install_text_window_row_decoration_request(
    mut output: TextWindowOutputTarget<'_>,
    request: TextWindowRowDecorationRequest,
) {
    install_text_window_row_decoration(output.builder(), request);
}

fn begin_display_text_row(
    output_builder: &mut DisplayOutputBuilder,
    begin: DisplayTextRowBegin,
) -> usize {
    begin_text_output_row(output_builder, begin.display_row_index)
}

fn finish_display_text_row(
    output_builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
    metrics: DisplayTextRowMetrics,
) -> DisplayTextRowFinish {
    let matrix_metrics = install_text_output_row_metrics(
        output_builder,
        display_text_row_metrics_request(display_row_index, metrics),
    );
    DisplayTextRowFinish {
        display_row_index,
        metrics: matrix_metrics,
    }
}

fn finalize_display_text_row(output_builder: &mut DisplayOutputBuilder, display_row_index: usize) {
    finalize_text_output_row(output_builder, display_row_index);
}

pub(crate) fn begin_text_window_row(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    begin: DisplayTextRowBegin,
) -> usize {
    let display_row_index = begin_display_text_row(output.builder(), begin);
    output_emitter.begin_display_text_row(
        evaluator,
        begin.display_row_index,
        begin.row,
        begin.col,
        begin.y,
        begin.x,
    );
    display_row_index
}

pub(crate) fn finish_text_window_row(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    metrics: DisplayTextRowMetrics,
) -> DisplayTextRowFinish {
    let display_row_index = output_emitter.current_display_text_row_index();
    let finish = finish_display_text_row(output.builder(), display_row_index, metrics);
    output_emitter.push_text_row(metrics.y, metrics.height, metrics.ascent);
    finish
}

pub(crate) fn finish_and_end_text_window_row(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    metrics: DisplayTextRowMetrics,
) -> DisplayTextRowFinish {
    let display_row_index = output_emitter.current_display_text_row_index();
    let matrix_metrics = finish_text_output_row(
        output.builder(),
        display_text_row_metrics_request(display_row_index, metrics),
    );
    output_emitter.push_text_row(metrics.y, metrics.height, metrics.ascent);
    DisplayTextRowFinish {
        display_row_index,
        metrics: matrix_metrics,
    }
}

pub(crate) fn transition_text_window_row(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    transition: DisplayTextRowGeometryTransition,
) -> DisplayTextRowTransition {
    finish_and_end_text_window_row(output.reborrow(), output_emitter, transition.finished_row);
    begin_text_window_row(
        output.reborrow(),
        output_emitter,
        evaluator,
        transition.begin_row,
    );
    DisplayTextRowTransition::BeganNextRow
}

pub(crate) fn transition_text_window_row_with_limit(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    transition: DisplayTextRowGeometryTransition,
    max_rows: usize,
) -> DisplayTextRowTransition {
    if transition.begin_row.row >= max_rows {
        finish_and_end_text_window_row(output.reborrow(), output_emitter, transition.finished_row);
        return DisplayTextRowTransition::ExhaustedRows;
    }
    transition_text_window_row(output.reborrow(), output_emitter, evaluator, transition)
}

pub(crate) fn begin_text_window_output_and_row(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    request: TextWindowBegin,
) {
    let first_row = request.first_row;
    begin_text_window_output(output.reborrow(), request.into());
    begin_text_window_row(output.reborrow(), output_emitter, evaluator, first_row);
}

pub(crate) fn finish_pending_text_window_row(
    output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    request: TextWindowPendingRowFinish<'_>,
) -> bool {
    let has_pending_row_output = output_emitter.current_row_has_output();
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
    finish_text_window_row(output, output_emitter, row_cursor.finish_current_row());
    true
}

pub(crate) fn render_window_chrome_rows(
    output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    request: WindowChromeRowsRenderRequest<'_, '_>,
    render_services: ChromeRowRenderServices<'_, '_>,
) {
    request.render(&mut WindowChromeRowsRenderState::new(
        output,
        output_emitter,
        evaluator,
        render_services,
    ));
}

pub(crate) fn close_text_window_output(mut output: TextWindowOutputTarget<'_>) {
    end_text_output_window(output.builder());
}

pub(crate) fn install_text_window_finished_rows(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &WindowOutputEmitter,
) {
    finish_output_rows(output.builder(), output_emitter);
}

pub(crate) fn capture_text_window_retry_checkpoint(
    mut output: TextWindowOutputTarget<'_>,
) -> TextWindowOutputRetryCheckpoint {
    let output_builder = output.builder();
    TextWindowOutputRetryCheckpoint {
        transition_hints_len: output_builder.transition_hints().len(),
        effect_hints_len: output_builder.effect_hints().len(),
    }
}

pub(crate) fn restore_text_window_retry_checkpoint(
    mut output: TextWindowOutputTarget<'_>,
    checkpoint: TextWindowOutputRetryCheckpoint,
) {
    restore_text_output_retry_checkpoint(
        output.builder(),
        checkpoint.transition_hints_len,
        checkpoint.effect_hints_len,
    );
}

fn install_display_text_row_metrics(
    output_builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
    metrics: DisplayTextRowMetrics,
) -> DisplayTextRowStoredMetrics {
    install_text_output_row_metrics(
        output_builder,
        display_text_row_metrics_request(display_row_index, metrics),
    )
}

fn display_text_row_metrics_request(
    display_row_index: usize,
    metrics: DisplayTextRowMetrics,
) -> DisplayOutputTextRowMetricsInstallRequest {
    DisplayOutputTextRowMetricsInstallRequest::new(
        display_row_index,
        metrics.y,
        metrics.height,
        metrics.ascent,
    )
}

fn finish_output_rows(
    output_builder: &mut DisplayOutputBuilder,
    output_emitter: &WindowOutputEmitter,
) {
    for metric in output_emitter.row_metrics() {
        let _ = install_display_text_row_metrics(
            output_builder,
            metric.display_row_index,
            DisplayTextRowMetrics {
                y: metric.pixel_y,
                height: metric.height,
                ascent: metric.ascent,
            },
        );
    }
    if let Some(metric) = output_emitter.row_metrics().last() {
        finalize_display_text_row(output_builder, metric.display_row_index);
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
    pub(crate) display_row_index: usize,
    pub(crate) metrics: DisplayTextRowStoredMetrics,
}

pub(crate) type DisplayTextRowStoredMetrics = DisplayOutputRowStoredMetrics;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextWindowOutputRetryCheckpoint {
    pub(crate) transition_hints_len: usize,
    pub(crate) effect_hints_len: usize,
}

pub(crate) fn install_text_window_cursor_effects(
    mut output: TextWindowOutputTarget<'_>,
    request: TextWindowCursorEffects,
) {
    install_text_output_cursor_effects(output.builder(), request.window_id, request.effects);
}

pub(crate) fn publish_text_window_decorative_cursor(
    mut output: TextWindowOutputTarget<'_>,
    cursor: TextWindowDecorativeCursor,
) {
    if let Some(effects) = cursor.effects {
        install_text_window_cursor_effects(
            output.reborrow(),
            TextWindowCursorEffects {
                window_id: cursor.window_id,
                effects,
            },
        );
    }
    install_text_window_cursor_artifact(
        output.builder(),
        TextWindowCursorArtifact {
            window_id: cursor.window_id,
            slot_id: cursor.slot_id,
            x: cursor.x,
            y: cursor.y,
            width: cursor.width,
            height: cursor.height,
            style: cursor.style,
            color: cursor.color,
        },
    );
}

fn install_text_window_cursor_artifact(
    output_builder: &mut DisplayOutputBuilder,
    cursor: TextWindowCursorArtifact,
) {
    install_text_output_cursor_artifact(
        output_builder,
        DisplayOutputCursorArtifactInstallRequest::new(
            cursor.window_id,
            cursor.slot_id,
            cursor.x,
            cursor.y,
            cursor.width,
            cursor.height,
            cursor.style,
            cursor.color,
        ),
    );
}

fn store_text_window_phys_cursor(output_builder: &mut DisplayOutputBuilder, cursor: PhysCursor) {
    store_text_output_phys_cursor(output_builder, cursor);
}

fn install_text_window_row_cursor(
    output_builder: &mut DisplayOutputBuilder,
    row: usize,
    row_col: u16,
    style: CursorStyle,
) {
    install_text_output_row_cursor(output_builder, row, row_col, style);
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextWindowCursorPublication {
    cursor_artifact: Option<TextWindowCursorArtifact>,
    row: usize,
    row_col: u16,
    style: CursorStyle,
    live_cursor: WindowCursorSnapshot,
    selected_phys_cursor: Option<PhysCursor>,
}

#[derive(Clone, Debug, PartialEq)]
struct TextWindowCursorArtifact {
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
    pub(crate) installed_cursor_artifact: bool,
    pub(crate) stored_phys_cursor: bool,
    pub(crate) row: usize,
    pub(crate) row_col: u16,
    pub(crate) live_cursor: WindowCursorSnapshot,
}

impl TextWindowCursorPublication {
    fn resolve(output_builder: &DisplayOutputBuilder, cursor: TextWindowCursor) -> Self {
        let cursor_artifact = (!cursor.selected).then_some(TextWindowCursorArtifact {
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
                .resolve_phys_cursor_placement(output_builder.cursor_visual_column_context())
            {
                placement.apply_to(&mut phys_cursor);
            }
            phys_cursor.col
        } else {
            cursor.col()
        };

        Self {
            cursor_artifact,
            row: cursor.row(),
            row_col,
            style: cursor.style,
            live_cursor: cursor.window_snapshot(),
            selected_phys_cursor: cursor.selected.then_some(phys_cursor),
        }
    }

    fn publish(
        self,
        mut output: TextWindowOutputTarget<'_>,
        output_emitter: &mut WindowOutputEmitter,
    ) -> TextWindowCursorPublicationOutcome {
        let installed_cursor_artifact = self.cursor_artifact.is_some();
        if let Some(cursor) = self.cursor_artifact {
            install_text_window_cursor_artifact(output.builder(), cursor);
        }
        install_text_window_row_cursor(output.builder(), self.row, self.row_col, self.style);
        output_emitter.set_phys_cursor(self.live_cursor.clone());
        if let Some(cursor) = self.selected_phys_cursor.clone() {
            store_text_window_phys_cursor(output.builder(), cursor);
        }
        TextWindowCursorPublicationOutcome {
            installed_cursor_artifact,
            stored_phys_cursor: self.selected_phys_cursor.is_some(),
            row: self.row,
            row_col: self.row_col,
            live_cursor: self.live_cursor,
        }
    }
}

pub(crate) fn publish_text_window_cursor(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &mut WindowOutputEmitter,
    cursor: TextWindowCursor,
) -> TextWindowCursorPublicationOutcome {
    let publication = TextWindowCursorPublication::resolve(output.builder(), cursor);
    publication.publish(output, output_emitter)
}

pub(crate) fn install_text_window_body_output(
    mut output: TextWindowOutputTarget<'_>,
    output_emitter: &WindowOutputEmitter,
    request: TextWindowBodyOutputInstall<'_>,
    render_services: Option<ChromeRowRenderServices<'_, '_>>,
) -> TextWindowRedisplayPositions {
    let redisplay_positions = TextWindowRedisplayPositions::from_output_rows(
        output_emitter,
        request.window_start,
        request.text_start_byte,
        request.byte_idx,
    );
    record_text_window_redisplay_positions(
        output.reborrow(),
        request.window_id,
        redisplay_positions,
    );
    install_text_window_finished_rows(output.reborrow(), output_emitter);
    if let Some(markers) = request.right_edge_markers {
        let render_services =
            render_services.expect("right-edge markers require chrome render services");
        install_text_window_right_edge_markers(output.builder(), render_services, markers);
    }
    redisplay_positions
}

pub(crate) trait DisplayProgressSink {
    #[cfg(test)]
    fn emit_text_progress(
        &mut self,
        evaluator: &mut Context,
        output: TextRowOutput,
        progress: &DisplayRowAppendProgress,
    );

    fn emit_chrome_progress(&mut self, evaluator: &mut Context, progress: ChromeRowProgress);
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
        self.emit_text_output_spans(
            evaluator,
            output,
            output.spans_for_source_slots(&progress.slots),
            progress.end,
        );
    }

    fn emit_chrome_progress(&mut self, evaluator: &mut Context, progress: ChromeRowProgress) {
        self.begin_chrome_row(evaluator, progress.output.row, progress.output.y);
        self.move_chrome_output_to(evaluator, progress.output.row, progress.progress);
        self.push_chrome_row_progress(progress.progress);
    }
}

impl WindowOutputEmitter {
    pub(crate) fn emit_text_output_spans(
        &mut self,
        evaluator: &mut Context,
        output: TextRowOutput,
        spans: Vec<TextOutputSpan>,
        end: DisplayRowPosition,
    ) {
        if spans.is_empty() {
            self.move_text_output_to(evaluator, output.row, end.col, output.row_y, end.x_px);
            return;
        }
        for span in spans {
            self.emit_text_output_span(evaluator, span);
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
        display_row_index: Option<usize>,
        row: i64,
        col: i64,
        y: i64,
        x: i64,
    ) {
        self.current_row_progress = Some(CurrentRowProgress {
            display_row_index,
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
        display_row_index: usize,
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
            Some(display_row_index),
            output_row,
            output_col,
            output_y,
            output_x,
        );
        let _ = self.with_live_update(evaluator, |update| {
            update.output_cursor_to_coords(output_row, output_col, output_y, output_x)
        });
    }

    pub(crate) fn current_display_text_row_index(&self) -> usize {
        self.current_row_progress
            .and_then(|progress| progress.display_row_index)
            .expect("text row must have display row progress before finishing")
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
            display_row_index: row_progress
                .display_row_index
                .expect("text row must have display row progress before recording metrics"),
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
