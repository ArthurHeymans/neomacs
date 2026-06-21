//! Buffer source row prelude requests.
//!
//! Converts local display policy into the line-number and prefix requests
//! emitted before ordinary buffer source items.

use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_face_state::DisplayRowActiveFaceState;
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_line_number_margin::BufferLineNumberMarginRenderRequest;
use crate::display_row_lisp_string::{
    BufferLinePrefixRenderContext, BufferLinePrefixRenderRequest, DisplayRowPrefixValues,
};
use crate::display_row_metrics::DisplayRowFallbackMetrics;

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
    ) -> BufferLinePrefixRenderRequest<'a> {
        BufferLinePrefixRenderRequest::new(
            BufferLinePrefixRenderContext::new(
                self.prefix_values,
                append_surface,
                row_geometry,
                active_face_state,
                glyph_y_offset,
                self.fallback_metrics,
            ),
            position,
        )
    }

    pub(crate) fn char_width(self) -> f32 {
        self.fallback_metrics.char_width()
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
