use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_output_install_request::{
    OutputCursorInstallRequest, OutputFrameArtifactInstallRequest, OutputFrameStateInstallRequest,
    OutputRetryCheckpointRestoreRequest, OutputTextWindowDisplayRangeInstallRequest,
};
use crate::display_output_row_request::{
    OutputCurrentRowDecorationRequest, OutputRowLifecycleRequest,
};
use crate::display_output_window_request::OutputWindowLifecycleRequest;
use crate::display_row::resolved_display_row_face;
use crate::font_metrics::FontMetrics;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, PhysCursor,
};
use neomacs_display_protocol::glyph_matrix::GlyphRow;
use neomacs_display_protocol::types::{Color, Rect};

pub(crate) struct DisplayRowOutputInstall<'a> {
    display_row_index: usize,
    row: &'a GlyphRow,
    pixel_y: f32,
    height_px: f32,
    ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayOutputRowStoredMetrics {
    pub(crate) pixel_y: f32,
    pub(crate) height_px: f32,
    pub(crate) ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayOutputTextRowMetricsInstallRequest {
    display_row_index: usize,
    absolute_y: f32,
    height_px: f32,
    ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayOutputTextWindowBeginInstallRequest {
    window_id: u64,
    rows: usize,
    cols: usize,
    bounds: Rect,
    text_bounds: Rect,
    selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayOutputCursorArtifactInstallRequest {
    window_id: i64,
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    style: CursorStyle,
    color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextWindowRowDecorationRequest {
    MarkCurrentTruncatedLeft,
}

impl<'a> DisplayRowOutputInstall<'a> {
    pub(crate) fn from_row(display_row_index: usize, row: &'a GlyphRow) -> Self {
        Self {
            display_row_index,
            row,
            pixel_y: row.pixel_y,
            height_px: row.height_px,
            ascent_px: row.ascent_px,
        }
    }

    pub(crate) fn install(self, builder: &mut DisplayOutputBuilder) {
        let pixel_bounds = builder.current_window_pixel_bounds();
        let mut row = self.row.clone();
        row.pixel_y = self.pixel_y - pixel_bounds.y;
        row.height_px = self.height_px;
        row.ascent_px = self.ascent_px;
        builder.install_output_row_lifecycle(OutputRowLifecycleRequest::complete(
            self.display_row_index,
            row.role,
            row.mode_line,
            row,
        ));
    }
}

impl DisplayOutputTextWindowBeginInstallRequest {
    pub(crate) fn new(
        window_id: u64,
        rows: usize,
        cols: usize,
        bounds: Rect,
        text_bounds: Rect,
        selected: bool,
    ) -> Self {
        Self {
            window_id,
            rows,
            cols,
            bounds,
            text_bounds,
            selected,
        }
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
        builder.install_output_window_lifecycle(OutputWindowLifecycleRequest::begin(
            self.window_id,
            self.rows,
            self.cols,
            self.bounds,
            self.text_bounds,
            self.selected,
        ));
    }
}

impl DisplayOutputCursorArtifactInstallRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        window_id: i64,
        slot_id: DisplaySlotId,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: CursorStyle,
        color: Color,
    ) -> Self {
        Self {
            window_id,
            slot_id,
            x,
            y,
            width,
            height,
            style,
            color,
        }
    }

    fn install(self, builder: &mut DisplayOutputBuilder) {
        builder.install_output_cursor(OutputCursorInstallRequest::new(
            self.window_id,
            self.slot_id,
            self.x,
            self.y,
            self.width,
            self.height,
            self.style,
            self.color,
        ));
    }
}

impl DisplayOutputRowStoredMetrics {
    fn from_absolute_window_y(
        builder: &DisplayOutputBuilder,
        request_y: f32,
        height_px: f32,
        ascent_px: f32,
    ) -> Self {
        let window_y = builder.current_window_pixel_bounds().y;
        Self {
            pixel_y: request_y - window_y,
            height_px,
            ascent_px,
        }
    }
}

impl DisplayOutputTextRowMetricsInstallRequest {
    pub(crate) fn new(
        display_row_index: usize,
        absolute_y: f32,
        height_px: f32,
        ascent_px: f32,
    ) -> Self {
        Self {
            display_row_index,
            absolute_y,
            height_px,
            ascent_px,
        }
    }

    pub(crate) fn display_row_index(self) -> usize {
        self.display_row_index
    }

    fn stored_metrics(self, builder: &DisplayOutputBuilder) -> DisplayOutputRowStoredMetrics {
        DisplayOutputRowStoredMetrics::from_absolute_window_y(
            builder,
            self.absolute_y,
            self.height_px,
            self.ascent_px,
        )
    }

    fn install(self, builder: &mut DisplayOutputBuilder) -> DisplayOutputRowStoredMetrics {
        let metrics = self.stored_metrics(builder);
        builder.install_output_row_lifecycle(OutputRowLifecycleRequest::metrics(
            self.display_row_index,
            metrics.pixel_y,
            metrics.height_px,
            metrics.ascent_px,
        ));
        metrics
    }
}

pub(crate) fn begin_text_output_window(
    builder: &mut DisplayOutputBuilder,
    request: DisplayOutputTextWindowBeginInstallRequest,
) {
    request.install(builder);
}

pub(crate) fn end_text_output_window(builder: &mut DisplayOutputBuilder) {
    builder.install_output_window_lifecycle(OutputWindowLifecycleRequest::end());
}

pub(crate) fn install_text_output_display_range(
    builder: &mut DisplayOutputBuilder,
    window_id: i64,
    window_start: i64,
    window_end: i64,
) {
    builder.install_window_metadata(OutputTextWindowDisplayRangeInstallRequest::new(
        window_id,
        window_start,
        window_end,
    ));
}

pub(crate) fn restore_text_output_retry_checkpoint(
    builder: &mut DisplayOutputBuilder,
    transition_hints_len: usize,
    effect_hints_len: usize,
) {
    builder.install_window_metadata(OutputRetryCheckpointRestoreRequest::new(
        transition_hints_len,
        effect_hints_len,
    ));
}

pub(crate) fn install_text_output_row_decoration(
    builder: &mut DisplayOutputBuilder,
    request: TextWindowRowDecorationRequest,
) {
    match request {
        TextWindowRowDecorationRequest::MarkCurrentTruncatedLeft => {
            builder.install_output_row_lifecycle(OutputRowLifecycleRequest::current_decoration(
                OutputCurrentRowDecorationRequest::MarkTruncatedLeft,
            ));
        }
    }
}

pub(crate) fn install_text_output_cursor_effects(
    builder: &mut DisplayOutputBuilder,
    window_id: i64,
    effects: EffectsConfig,
) {
    builder.install_output_frame_state(OutputFrameStateInstallRequest::cursor_effects(
        window_id, effects,
    ));
}

pub(crate) fn install_text_output_cursor_artifact(
    builder: &mut DisplayOutputBuilder,
    request: DisplayOutputCursorArtifactInstallRequest,
) {
    request.install(builder);
}

pub(crate) fn install_text_output_row_cursor(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
    col: u16,
    style: CursorStyle,
) {
    builder.install_output_row_lifecycle(OutputRowLifecycleRequest::cursor(
        display_row_index,
        col,
        style,
    ));
}

pub(crate) fn store_text_output_phys_cursor(
    builder: &mut DisplayOutputBuilder,
    cursor: PhysCursor,
) {
    builder.install_output_frame_artifact(OutputFrameArtifactInstallRequest::phys_cursor(cursor));
}

pub(crate) fn edit_current_text_output_row<R>(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
    f: impl FnOnce(&mut GlyphRow, usize) -> R,
) -> Option<R> {
    builder.edit_current_window_row_with_matrix_cols(display_row_index, f)
}

pub(crate) fn edit_last_text_output_rows(
    builder: &mut DisplayOutputBuilder,
    f: impl FnMut(&mut GlyphRow, usize),
) {
    builder.edit_last_window_rows_with_matrix_cols(f);
}

pub(crate) fn begin_text_output_row(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
) -> usize {
    builder.install_output_row_lifecycle(OutputRowLifecycleRequest::begin(
        display_row_index,
        GlyphRowRole::Text,
        false,
    ));
    display_row_index
}

pub(crate) fn install_text_output_row_metrics(
    builder: &mut DisplayOutputBuilder,
    request: DisplayOutputTextRowMetricsInstallRequest,
) -> DisplayOutputRowStoredMetrics {
    request.install(builder)
}

pub(crate) fn finish_text_output_row(
    builder: &mut DisplayOutputBuilder,
    request: DisplayOutputTextRowMetricsInstallRequest,
) -> DisplayOutputRowStoredMetrics {
    let metrics = install_text_output_row_metrics(builder, request);
    builder.install_output_row_lifecycle(OutputRowLifecycleRequest::finalize(
        request.display_row_index(),
    ));
    metrics
}

pub(crate) fn finalize_text_output_row(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
) {
    builder.install_output_row_lifecycle(OutputRowLifecycleRequest::finalize(display_row_index));
}

pub(crate) fn install_display_row(
    builder: &mut DisplayOutputBuilder,
    display_row_index: usize,
    row: &GlyphRow,
) {
    DisplayRowOutputInstall::from_row(display_row_index, row).install(builder);
}

pub(crate) fn install_output_resolved_face(
    builder: &mut DisplayOutputBuilder,
    face_id: u32,
    face: &ResolvedFace,
    metrics: Option<FontMetrics>,
) {
    let render_face = resolved_display_row_face(face_id, face, metrics);
    builder.install_output_frame_state(OutputFrameStateInstallRequest::face(
        render_face.face_id,
        render_face.render_face(),
    ));
}
