//! Buffer source row prelude requests.
//!
//! Converts local display policy into the line-number and prefix requests
//! emitted before ordinary buffer source items.

use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_face_state::DisplayRowActiveFaceState;
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_line_number_margin::BufferLineNumberMarginRenderRequest;
use crate::display_row_lisp_string::{BufferLinePrefixRenderRequest, DisplayRowPrefixValues};
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{FaceScanCheckpoint, LineNumberRenderState};
use crate::frame_face_arena::FrameFaceAttempt;
use crate::types::WindowParams;

/// The buffer-owned decoration emitted when a display-only source starts a
/// continuation row without advancing the logical buffer line.
///
/// GNU's display iterator runs `maybe_produce_line_number` for every glyph
/// row, including rows created by an overlay-string newline.  Keeping that
/// policy in a row-prelude request means overlay rendering does not need to
/// understand line-number modes, faces, widths, or margin glyph construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferSourceContinuationRowPreludeRequest {
    line_number_margin: BufferLineNumberMarginRenderRequest,
    char_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferSourceRowPreludeRequestContext {
    line_number_mode: u8,
    line_number_current_absolute: bool,
    line_number_offset: i64,
    line_number_major_tick: i32,
    line_number_cols: i32,
    prefix_values: DisplayRowPrefixValues,
    fallback_metrics: DisplayRowFallbackMetrics,
}

impl BufferSourceContinuationRowPreludeRequest {
    pub(crate) fn render_with_source_state(
        self,
        line_numbers: &mut LineNumberRenderState,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        row_geometry: &DisplayRowGeometryState,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        line_numbers.mark_continuation_row();
        self.line_number_margin.render_pending_with_source_state(
            line_numbers,
            source_render,
            face_ids,
            row_geometry,
            face_scan,
            self.char_width,
        );
    }
}

impl BufferSourceRowPreludeRequestContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        line_number_mode: u8,
        line_number_current_absolute: bool,
        line_number_offset: i64,
        line_number_major_tick: i32,
        line_number_cols: i32,
        prefix_values: DisplayRowPrefixValues,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            line_number_mode,
            line_number_current_absolute,
            line_number_offset,
            line_number_major_tick,
            line_number_cols,
            prefix_values,
            fallback_metrics,
        }
    }

    pub(crate) fn line_number_margin_request(self) -> BufferLineNumberMarginRenderRequest {
        BufferLineNumberMarginRenderRequest::new(
            self.line_number_mode,
            self.line_number_current_absolute,
            self.line_number_offset,
            self.line_number_major_tick,
            self.line_number_cols,
        )
    }

    pub(crate) fn line_prefix_request<'a>(
        self,
        append_surface: &'a DisplayRowAppendSurface,
        row_geometry: &'a DisplayRowGeometryState,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        position: DisplayRowPosition,
        params: &'a WindowParams,
    ) -> BufferLinePrefixRenderRequest<'a> {
        BufferLinePrefixRenderRequest::new(
            self.prefix_values,
            append_surface,
            row_geometry,
            active_face_state,
            glyph_y_offset,
            self.fallback_metrics,
            position,
            params,
        )
    }

    pub(crate) fn char_width(self) -> f32 {
        self.fallback_metrics.char_width()
    }

    pub(crate) fn continuation_row_prelude(self) -> BufferSourceContinuationRowPreludeRequest {
        BufferSourceContinuationRowPreludeRequest {
            line_number_margin: self.line_number_margin_request(),
            char_width: self.char_width(),
        }
    }

    #[cfg(test)]
    pub(crate) fn line_number_mode(self) -> u8 {
        self.line_number_mode
    }

    #[cfg(test)]
    pub(crate) fn prefix_values(self) -> DisplayRowPrefixValues {
        self.prefix_values
    }
}
