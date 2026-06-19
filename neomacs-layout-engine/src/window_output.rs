//! Live window-output emission helpers for Rust redisplay.
//!
//! This layer bridges Rust layout/status-line emission to GNU-like live window
//! output state. It advances live output through explicit output-cursor moves
//! while simultaneously recording immutable row snapshots for renderer
//! handoff.

use super::display_status_line::DisplayRowOutputProgress;
use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLength, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, DisplayTextRun, RenderFaceRef, SourceSpan,
};
#[cfg(test)]
use crate::display_row_builder::DisplayRowAppendProgress;
use crate::display_row_builder::{
    DisplayRowGlyphSlot, DisplayRowPosition, mark_display_row_truncated_left,
};
use crate::display_row_geometry::{
    DisplayRowFlags, DisplayRowGeometryState, DisplayRowLimit, DisplayRowYPositions,
};
use crate::display_row_walk_state::HitRowRangeTracker;
use crate::display_source::{DisplayItemSource, DisplaySourceContext, SyntheticTextItemSource};
use crate::hit_test::HitRow;
use crate::matrix_builder::{
    GlyphMatrixBuilder, MatrixCursorInstallRequest, MatrixFrameStateInstallRequest,
    MatrixIndexedRowMetricsRequest, MatrixRowBeginRequest, MatrixRowCursorRequest,
    MatrixRowLifecycleRequest, MatrixRowMetricsRequest, MatrixWindowBeginRequest,
    MatrixWindowLifecycleRequest,
};
use neomacs_display_protocol::effect_config::EffectsConfig;
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

const LINE_NUMBER_MARGIN_SOURCE_ID: u64 = 0x6c6e_756d;
const RIGHT_EDGE_MARKER_SOURCE_ID: u64 = 0x7265_6467;
const RIGHT_BORDER_SOURCE_ID: u64 = 0x7262_6f72;

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

pub(crate) struct TextWindowBodyOutputInstall {
    pub(crate) window_id: u64,
    pub(crate) window_start: i64,
    pub(crate) text_start_byte: usize,
    pub(crate) byte_idx: usize,
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
pub(crate) struct TextWindowLineNumberMargin<'a> {
    pub(crate) text: &'a str,
    pub(crate) cols: i32,
    pub(crate) face_id: u32,
    pub(crate) row_y: f32,
    pub(crate) row_height: f32,
    pub(crate) row_ascent: f32,
    pub(crate) char_width: f32,
}

pub(crate) enum TextWindowRowDecorationRequest {
    MarkCurrentTruncatedLeft,
}

impl TextWindowRowDecorationRequest {
    fn install(self, builder: &mut GlyphMatrixBuilder) {
        match self {
            Self::MarkCurrentTruncatedLeft => {
                builder.with_current_row_mut(|glyph_row| {
                    mark_display_row_truncated_left(glyph_row);
                });
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
        self.builder
            .install_row_lifecycle(MatrixRowLifecycleRequest::Begin(MatrixRowBeginRequest {
                row: begin.matrix_row,
                role: GlyphRowRole::Text,
            }));
        self.output_emitter
            .begin_text_row(self.evaluator, begin.row, begin.col, begin.y, begin.x);
    }

    pub(crate) fn finish(&mut self, metrics: TextMatrixRowMetrics) {
        self.builder
            .install_row_lifecycle(MatrixRowLifecycleRequest::CurrentMetrics(
                MatrixRowMetricsRequest {
                    pixel_y: metrics.y,
                    height_px: metrics.height,
                    ascent_px: metrics.ascent,
                },
            ));
        self.output_emitter
            .push_text_row(metrics.y, metrics.height, metrics.ascent);
    }

    pub(crate) fn finish_and_end(&mut self, metrics: TextMatrixRowMetrics) {
        self.finish(metrics);
        self.builder
            .install_row_lifecycle(MatrixRowLifecycleRequest::EndIncremental);
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
    let first_row = request.first_row;
    begin_text_window_matrix(builder, request.into());
    TextMatrixRowOutput::new(builder, output_emitter, evaluator).begin(first_row);
}

pub(crate) fn begin_text_window_matrix(
    builder: &mut GlyphMatrixBuilder,
    request: TextWindowMatrixBegin,
) {
    builder.install_window_lifecycle(MatrixWindowLifecycleRequest::Begin(
        MatrixWindowBeginRequest {
            window_id: request.window_id,
            nrows: request.rows,
            ncols: request.cols,
            pixel_bounds: request.bounds,
            text_pixel_bounds: request.text_bounds,
            selected: request.selected,
        },
    ));
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

pub(crate) fn record_text_window_redisplay_positions(
    builder: &mut GlyphMatrixBuilder,
    window_id: u64,
    positions: TextWindowRedisplayPositions,
) {
    record_text_window_display_range(builder, positions.display_range(window_id));
}

pub(crate) fn close_text_window_output(builder: &mut GlyphMatrixBuilder) {
    builder.install_window_lifecycle(MatrixWindowLifecycleRequest::End);
}

pub(crate) fn finish_text_matrix_row_output(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    metrics: TextMatrixRowMetrics,
) {
    TextMatrixRowOutput::new(builder, output_emitter, evaluator).finish(metrics);
}

pub(crate) fn finish_pending_text_window_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
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
    finish_text_matrix_row_output(
        builder,
        output_emitter,
        evaluator,
        row_cursor.finish_current_row(),
    );
    true
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

#[cfg(test)]
pub(crate) fn mark_current_text_row_truncated_left(builder: &mut GlyphMatrixBuilder) {
    install_text_window_row_decoration(
        builder,
        TextWindowRowDecorationRequest::MarkCurrentTruncatedLeft,
    );
}

fn line_number_margin_text_item(text: &str, face_id: u32, start_offset: usize) -> DisplayItem {
    let end_offset = start_offset.saturating_add(text.chars().count());
    DisplayItem::new(
        SourceSpan::synthetic(LINE_NUMBER_MARGIN_SOURCE_ID, start_offset, end_offset),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text.to_owned())),
    )
}

fn line_number_margin_stretch_item(
    cols: u16,
    face_id: u32,
    char_width: f32,
    start_offset: usize,
) -> DisplayItem {
    DisplayItem::new(
        SourceSpan::synthetic(
            LINE_NUMBER_MARGIN_SOURCE_ID,
            start_offset,
            start_offset.saturating_add(usize::from(cols)),
        ),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(
                f32::from(cols) * char_width.max(1.0),
            )),
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
                request.char_width,
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
            request.char_width,
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

pub(crate) fn right_border_text_source(
    text: impl Into<Box<str>>,
    face_id: u32,
    start_offset: usize,
) -> SyntheticTextItemSource {
    SyntheticTextItemSource::new(
        RIGHT_BORDER_SOURCE_ID,
        text,
        RenderFaceRef::FaceId(face_id),
        start_offset,
    )
}

pub(crate) struct RightEdgeMarkerItemSource {
    items: std::vec::IntoIter<DisplayItem>,
}

impl RightEdgeMarkerItemSource {
    pub(crate) fn new(padding_cols: usize, marker: char, face_id: u32) -> Self {
        let mut source_offset = 0usize;
        let mut items = Vec::with_capacity(usize::from(padding_cols > 0) + 1);
        if padding_cols > 0 {
            items.push(synthetic_window_marker_text_item(
                RIGHT_EDGE_MARKER_SOURCE_ID,
                " ".repeat(padding_cols),
                face_id,
                source_offset,
            ));
            source_offset = source_offset.saturating_add(padding_cols);
        }
        items.push(synthetic_window_marker_text_item(
            RIGHT_EDGE_MARKER_SOURCE_ID,
            marker.to_string(),
            face_id,
            source_offset,
        ));
        Self {
            items: items.into_iter(),
        }
    }
}

impl DisplayItemSource for RightEdgeMarkerItemSource {
    fn next_item(&mut self, _context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.items.next()
    }
}

fn synthetic_window_marker_text_item(
    source_id: u64,
    text: impl Into<Box<str>>,
    face_id: u32,
    start_offset: usize,
) -> DisplayItem {
    let text = text.into();
    let end_offset = start_offset.saturating_add(text.chars().count());
    DisplayItem::new(
        SourceSpan::synthetic(source_id, start_offset, end_offset),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text)),
    )
}

pub(crate) fn publish_text_window_cursor(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    cursor: TextWindowCursor,
) {
    if !cursor.selected {
        builder.install_cursor(MatrixCursorInstallRequest {
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
    builder.install_row_lifecycle(MatrixRowLifecycleRequest::CursorAt(
        MatrixRowCursorRequest {
            row: cursor.row(),
            col: cursor.col(),
            style: cursor.style,
        },
    ));
    output_emitter.set_phys_cursor(cursor.window_snapshot());
    if cursor.selected {
        if cursor.glyph_row_resolved {
            builder.set_glyph_row_resolved_phys_cursor(cursor.phys_cursor());
        } else {
            builder.set_phys_cursor(cursor.phys_cursor());
        }
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
    builder.install_cursor(MatrixCursorInstallRequest {
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

pub(crate) fn install_text_window_cursor_effects(
    builder: &mut GlyphMatrixBuilder,
    request: TextWindowCursorEffects,
) {
    builder.install_frame_state(MatrixFrameStateInstallRequest::CursorEffects {
        window_id: request.window_id,
        effects: request.effects,
    });
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
        builder.install_row_lifecycle(MatrixRowLifecycleRequest::RowMetrics(
            MatrixIndexedRowMetricsRequest {
                row: metric.row,
                metrics: MatrixRowMetricsRequest {
                    pixel_y: metric.pixel_y,
                    height_px: metric.height,
                    ascent_px: metric.ascent,
                },
            },
        ));
    }
    builder.install_row_lifecycle(MatrixRowLifecycleRequest::EndIncremental);
}

pub(crate) fn install_text_window_output(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &WindowOutputEmitter,
    _request: TextWindowOutputInstall,
) {
    finish_text_window_output_rows(builder, output_emitter);
}

pub(crate) fn install_text_window_body_output(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &WindowOutputEmitter,
    request: TextWindowBodyOutputInstall,
) -> TextWindowRedisplayPositions {
    let redisplay_positions = TextWindowRedisplayPositions::from_output_rows(
        output_emitter,
        request.window_start,
        request.text_start_byte,
        request.byte_idx,
    );
    record_text_window_redisplay_positions(builder, request.window_id, redisplay_positions);
    install_text_window_output(builder, output_emitter, TextWindowOutputInstall);
    redisplay_positions
}

pub(crate) fn install_text_window_row_decoration(
    builder: &mut GlyphMatrixBuilder,
    request: TextWindowRowDecorationRequest,
) {
    request.install(builder);
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
