use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_output_install_request::OutputFrameStateInstallRequest;
use crate::display_output_row_request::OutputRowLifecycleRequest;
use crate::display_output_window_request::OutputWindowLifecycleRequest;
use crate::display_row_face_state::resolved_display_row_face;
use crate::font_metrics::FontMetrics;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::glyph_matrix::GlyphRow;
use neomacs_display_protocol::types::FaceId;
use neomacs_display_protocol::types::Rect;

pub(crate) struct DisplayRowOutputInstall<'a> {
    display_row_index: usize,
    row: &'a GlyphRow,
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
    /// Buffer position (0-based) where this row's walk began. Stamped onto the
    /// row when it emitted no buffer glyphs, so empty lines and the EOB
    /// placeholder carry real `MATRIX_ROW_START/END_CHARPOS` values.
    row_start_charpos: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayOutputTextWindowBeginInstallRequest {
    window_id: u64,
    rows: usize,
    cols: usize,
    bounds: Rect,
    text_bounds: Rect,
    text_clip_bounds: Rect,
    selected: bool,
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
        }
    }

    pub(crate) fn install(self, builder: &mut DisplayOutputBuilder) {
        builder.install_output_row_lifecycle(
            OutputRowLifecycleRequest::complete_window_absolute_row(
                self.display_row_index,
                self.row,
                builder.current_window_pixel_bounds(),
            ),
        );
    }
}

impl DisplayOutputTextWindowBeginInstallRequest {
    pub(crate) fn new(
        window_id: u64,
        rows: usize,
        cols: usize,
        bounds: Rect,
        text_bounds: Rect,
        text_clip_bounds: Rect,
        selected: bool,
    ) -> Self {
        Self {
            window_id,
            rows,
            cols,
            bounds,
            text_bounds,
            text_clip_bounds,
            selected,
        }
    }

    pub(crate) fn install(self, builder: &mut DisplayOutputBuilder) {
        builder.install_output_window_lifecycle(OutputWindowLifecycleRequest::begin(
            self.window_id,
            self.rows,
            self.cols,
            self.bounds,
            self.text_bounds,
            self.text_clip_bounds,
            self.selected,
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
        row_start_charpos: i64,
    ) -> Self {
        Self {
            display_row_index,
            absolute_y,
            height_px,
            ascent_px,
            row_start_charpos,
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

    pub(crate) fn install(
        self,
        builder: &mut DisplayOutputBuilder,
    ) -> DisplayOutputRowStoredMetrics {
        let metrics = self.stored_metrics(builder);
        builder.install_output_row_lifecycle(OutputRowLifecycleRequest::metrics(
            self.display_row_index,
            metrics.pixel_y,
            metrics.height_px,
            metrics.ascent_px,
        ));
        // A buffer-text row that emitted no buffer glyphs still holds the unset
        // `start == end` bounds; give it the real position where its walk began
        // so empty lines carry their line's charpos and the trailing EOB
        // placeholder carries ZV, matching GNU's MATRIX_ROW_START/END_CHARPOS.
        builder.stamp_empty_text_row_buffer_bounds(self.display_row_index, self.row_start_charpos);
        metrics
    }
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
    face_id: FaceId,
    face: &ResolvedFace,
    metrics: Option<FontMetrics>,
) {
    let render_face = resolved_display_row_face(face_id, face, metrics);
    builder.install_output_frame_state(OutputFrameStateInstallRequest::face(
        render_face.face_id,
        render_face.render_face(),
    ));
}
