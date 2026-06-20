//! Output row grid storage for text/window rows.
//!
//! This module owns the low-level `GlyphMatrix` row storage used by the
//! output builder. Callers install already-built rows and metadata; the grid
//! is responsible only for placing, finalizing, and exporting matrix rows.

use crate::display_cursor::CursorVisualColumnRows;
use crate::display_output_builder::{OutputRowBeginRequest, OutputRowMetricsRequest};
use crate::display_row_finalizer::GlyphRowFinalizationContext;
use neomacs_display_protocol::frame_glyphs::PhysCursor;
use neomacs_display_protocol::glyph_matrix::{GlyphMatrix, GlyphRow, WindowMatrixEntry};
use neomacs_display_protocol::types::Rect;

pub(crate) struct OutputWindowRowGrid {
    matrix: GlyphMatrix,
}

pub(crate) struct OutputWindowGridEntry {
    window_id: u64,
    grid: OutputWindowRowGrid,
    pixel_bounds: Rect,
    text_pixel_bounds: Rect,
    selected: bool,
}

impl OutputWindowRowGrid {
    pub(crate) fn new(nrows: usize, ncols: usize) -> Self {
        Self {
            matrix: GlyphMatrix::new(nrows, ncols),
        }
    }

    fn into_matrix(self) -> GlyphMatrix {
        self.matrix
    }

    fn ensure_hashes(&mut self) {
        self.matrix.ensure_hashes();
    }

    pub(crate) fn enabled_row_count(&self) -> usize {
        self.matrix.rows.iter().filter(|row| row.enabled).count()
    }

    pub(crate) fn cursor_rows(&self) -> CursorVisualColumnRows<'_> {
        CursorVisualColumnRows::new(&self.matrix.rows, self.matrix.ncols)
    }

    pub(crate) fn row(&self, row: usize) -> Option<&GlyphRow> {
        self.matrix.rows.get(row)
    }

    pub(crate) fn row_mut(&mut self, row: usize) -> Option<&mut GlyphRow> {
        self.matrix.rows.get_mut(row)
    }

    pub(crate) fn edit_row_with_matrix_cols<R>(
        &mut self,
        row: usize,
        f: impl FnOnce(&mut GlyphRow, usize) -> R,
    ) -> Option<R> {
        let ncols = self.matrix.ncols;
        let row = self.row_mut(row)?;
        Some(f(row, ncols))
    }

    fn edit_rows_with_matrix_cols(&mut self, mut f: impl FnMut(&mut GlyphRow, usize)) {
        let ncols = self.matrix.ncols;
        for row in &mut self.matrix.rows {
            f(row, ncols);
        }
    }

    pub(crate) fn write_row_metrics(&mut self, row: usize, metrics: OutputRowMetricsRequest) {
        let Some(row) = self.row_mut(row) else {
            return;
        };
        row.pixel_y = metrics.pixel_y();
        row.height_px = metrics.height_px().max(0.0);
        row.ascent_px = metrics.ascent_px().max(0.0).min(row.height_px.max(0.0));
    }

    pub(crate) fn write_row_cursor(
        &mut self,
        row: usize,
        col: u16,
        style: neomacs_display_protocol::frame_glyphs::CursorStyle,
    ) {
        let Some(row) = self.row_mut(row) else {
            return;
        };
        row.cursor_col = Some(col);
        row.cursor_type = Some(style);
    }

    pub(crate) fn replace_row(&mut self, row: usize, source: GlyphRow) {
        let Some(row) = self.row_mut(row) else {
            return;
        };
        *row = source;
    }

    pub(crate) fn begin_row(&mut self, begin: OutputRowBeginRequest) {
        let Some(row) = self.row_mut(begin.row()) else {
            return;
        };
        row.role = begin.role();
        row.enabled = true;
        row.mode_line = begin.mode_line();
    }

    pub(crate) fn finalize_row(
        &mut self,
        window_id: u64,
        row: usize,
        pixel_bounds: Rect,
        phys_cursor: Option<&mut PhysCursor>,
    ) {
        let matrix_ncols = self.matrix.ncols;
        let Some(matrix_row) = self.row_mut(row) else {
            return;
        };
        GlyphRowFinalizationContext::new(window_id, row, pixel_bounds).finalize_row(
            matrix_row,
            matrix_ncols,
            phys_cursor,
        );
    }
}

impl OutputWindowGridEntry {
    pub(crate) fn new(
        window_id: u64,
        grid: OutputWindowRowGrid,
        pixel_bounds: Rect,
        text_pixel_bounds: Rect,
        selected: bool,
    ) -> Self {
        Self {
            window_id,
            grid,
            pixel_bounds,
            text_pixel_bounds,
            selected,
        }
    }

    pub(crate) fn edit_rows_with_matrix_cols(&mut self, f: impl FnMut(&mut GlyphRow, usize)) {
        self.grid.edit_rows_with_matrix_cols(f);
    }

    pub(crate) fn enabled_row_count(&self) -> usize {
        self.grid.enabled_row_count()
    }

    #[cfg(test)]
    pub(crate) fn window_id(&self) -> u64 {
        self.window_id
    }

    pub(crate) fn into_window_matrix_entry(mut self) -> WindowMatrixEntry {
        self.grid.ensure_hashes();
        WindowMatrixEntry {
            window_id: self.window_id,
            matrix: self.grid.into_matrix(),
            pixel_bounds: self.pixel_bounds,
            text_pixel_bounds: self.text_pixel_bounds,
            selected: self.selected,
        }
    }
}
