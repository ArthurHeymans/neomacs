//! Typed output row lifecycle requests.

use neomacs_display_protocol::frame_glyphs::{CursorStyle, GlyphRowRole};
use neomacs_display_protocol::glyph_matrix::GlyphRow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OutputRowBeginRequest {
    pub(crate) row: usize,
    pub(crate) role: GlyphRowRole,
    pub(crate) mode_line: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct OutputCompleteRowInstallRequest {
    pub(crate) row: usize,
    pub(crate) role: GlyphRowRole,
    pub(crate) mode_line: bool,
    pub(crate) glyph_row: GlyphRow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OutputRowMetricsRequest {
    /// Stored row Y, relative to the window matrix origin.
    pixel_y: f32,
    height_px: f32,
    ascent_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputCurrentRowDecorationRequest {
    MarkTruncatedLeft,
}

#[derive(Clone, Debug)]
pub(crate) enum OutputRowLifecycleRequest {
    Begin(OutputRowBeginRequest),
    Complete(OutputCompleteRowInstallRequest),
    Metrics {
        row: usize,
        metrics: OutputRowMetricsRequest,
    },
    Finalize {
        row: usize,
    },
    Cursor {
        row: usize,
        col: u16,
        style: CursorStyle,
    },
    CurrentDecoration(OutputCurrentRowDecorationRequest),
}

impl OutputRowBeginRequest {
    pub(crate) fn new(row: usize, role: GlyphRowRole, mode_line: bool) -> Self {
        Self {
            row,
            role,
            mode_line,
        }
    }

    pub(crate) fn apply_to_row(self, row: &mut GlyphRow) {
        row.role = self.role;
        row.enabled = true;
        row.mode_line = self.mode_line;
    }
}

impl OutputCompleteRowInstallRequest {
    pub(crate) fn new(
        row: usize,
        role: GlyphRowRole,
        mode_line: bool,
        glyph_row: GlyphRow,
    ) -> Self {
        Self {
            row,
            role,
            mode_line,
            glyph_row,
        }
    }
}

impl OutputRowMetricsRequest {
    pub(crate) fn new(pixel_y: f32, height_px: f32, ascent_px: f32) -> Self {
        Self {
            pixel_y,
            height_px,
            ascent_px,
        }
    }

    pub(crate) fn pixel_y(self) -> f32 {
        self.pixel_y
    }

    pub(crate) fn height_px(self) -> f32 {
        self.height_px.max(0.0)
    }

    pub(crate) fn ascent_px(self) -> f32 {
        self.ascent_px.max(0.0).min(self.height_px())
    }

    pub(crate) fn apply_to_row(self, row: &mut GlyphRow) {
        row.pixel_y = self.pixel_y();
        row.height_px = self.height_px();
        row.ascent_px = self.ascent_px();
    }
}

impl OutputRowLifecycleRequest {
    pub(crate) fn begin(row: usize, role: GlyphRowRole, mode_line: bool) -> Self {
        Self::Begin(OutputRowBeginRequest::new(row, role, mode_line))
    }

    pub(crate) fn complete(
        row: usize,
        role: GlyphRowRole,
        mode_line: bool,
        glyph_row: GlyphRow,
    ) -> Self {
        Self::Complete(OutputCompleteRowInstallRequest::new(
            row, role, mode_line, glyph_row,
        ))
    }

    pub(crate) fn metrics(row: usize, pixel_y: f32, height_px: f32, ascent_px: f32) -> Self {
        Self::Metrics {
            row,
            metrics: OutputRowMetricsRequest::new(pixel_y, height_px, ascent_px),
        }
    }

    pub(crate) fn finalize(row: usize) -> Self {
        Self::Finalize { row }
    }

    pub(crate) fn cursor(row: usize, col: u16, style: CursorStyle) -> Self {
        Self::Cursor { row, col, style }
    }

    pub(crate) fn current_decoration(decoration: OutputCurrentRowDecorationRequest) -> Self {
        Self::CurrentDecoration(decoration)
    }
}
