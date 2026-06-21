//! Mutable output window state owned while layout builds a frame snapshot.

use crate::display_cursor::CursorVisualColumnResolutionContext;
use crate::display_output_row_grid::{OutputWindowGridEntry, OutputWindowRowGrid};
use crate::display_output_row_request::{
    DisplayWindowRowMutation, DisplayWindowRowsMutation, OutputCompleteRowInstallRequest,
    OutputCurrentRowDecorationRequest, OutputRowBeginRequest, OutputRowLifecycleRequest,
    OutputRowMetricsRequest,
};
use crate::display_output_window_request::OutputWindowLifecycleRequest;
use neomacs_display_protocol::frame_glyphs::PhysCursor;
use neomacs_display_protocol::glyph_matrix::{GlyphRow, WindowMatrixEntry};
use neomacs_display_protocol::types::Rect;

pub(crate) struct OutputWindowBuildState {
    windows: Vec<OutputWindowGridEntry>,
    current_row_grid: Option<OutputWindowRowGrid>,
    current_window_id: u64,
    current_pixel_bounds: Rect,
    current_text_pixel_bounds: Rect,
    current_selected: bool,
    current_row: usize,
}

impl OutputWindowBuildState {
    pub(crate) fn new() -> Self {
        Self {
            windows: Vec::new(),
            current_row_grid: None,
            current_window_id: 0,
            current_pixel_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            current_text_pixel_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            current_selected: false,
            current_row: 0,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.windows.clear();
        self.current_row_grid = None;
        self.current_window_id = 0;
        self.current_selected = false;
        self.current_row = 0;
    }

    pub(crate) fn install_window_lifecycle(&mut self, request: OutputWindowLifecycleRequest) {
        match request {
            OutputWindowLifecycleRequest::Begin(begin) => {
                self.current_row_grid = Some(OutputWindowRowGrid::new(begin.nrows, begin.ncols));
                self.current_window_id = begin.window_id;
                self.current_pixel_bounds = begin.pixel_bounds;
                self.current_text_pixel_bounds = begin.text_pixel_bounds;
                self.current_selected = begin.selected;
                self.current_row = 0;
            }
            OutputWindowLifecycleRequest::End => {
                if let Some(grid) = self.current_row_grid.take() {
                    self.windows.push(OutputWindowGridEntry::new(
                        self.current_window_id,
                        grid,
                        self.current_pixel_bounds,
                        self.current_text_pixel_bounds,
                        self.current_selected,
                    ));
                }
            }
        }
    }

    pub(crate) fn install_row_lifecycle(
        &mut self,
        request: OutputRowLifecycleRequest,
        phys_cursor: Option<&mut PhysCursor>,
    ) {
        match request {
            OutputRowLifecycleRequest::Begin(begin) => self.begin_current_row(begin),
            OutputRowLifecycleRequest::Complete(complete) => {
                self.install_complete_row(complete, phys_cursor);
            }
            OutputRowLifecycleRequest::Metrics { row, metrics } => {
                self.write_row_metrics_at(row, metrics);
            }
            OutputRowLifecycleRequest::Finalize { row } => {
                self.finalize_output_row(row, phys_cursor)
            }
            OutputRowLifecycleRequest::Cursor { row, col, style } => {
                self.write_row_cursor(row, col, style);
            }
            OutputRowLifecycleRequest::CurrentDecoration(decoration) => {
                self.decorate_current_row(decoration);
            }
        }
    }

    pub(crate) fn edit_current_row<R>(&mut self, f: impl FnOnce(&mut GlyphRow) -> R) -> Option<R> {
        let grid = self.current_row_grid.as_mut()?;
        let row = grid.row_mut(self.current_row)?;
        Some(f(row))
    }

    pub(crate) fn current_row_for_render(&self) -> Option<&GlyphRow> {
        self.current_row_grid.as_ref()?.row(self.current_row)
    }

    pub(crate) fn apply_current_window_row_mutation<M>(
        &mut self,
        row_idx: usize,
        mutation: M,
    ) -> Option<M::Output>
    where
        M: DisplayWindowRowMutation,
    {
        self.current_row_grid
            .as_mut()?
            .edit_row_with_matrix_cols(row_idx, |row, matrix_cols| mutation.apply(row, matrix_cols))
    }

    pub(crate) fn apply_last_window_rows_mutation<M>(&mut self, mut mutation: M)
    where
        M: DisplayWindowRowsMutation,
    {
        if let Some(entry) = self.windows.last_mut() {
            entry.edit_rows_with_matrix_cols(|row, matrix_cols| {
                mutation.apply(row, matrix_cols);
            });
        }
    }

    #[cfg(test)]
    pub(crate) fn current_row_index(&self) -> usize {
        self.current_row
    }

    pub(crate) fn current_window_id_i64(&self) -> i64 {
        self.current_window_id as i64
    }

    pub(crate) fn current_window_pixel_bounds(&self) -> Rect {
        self.current_pixel_bounds
    }

    pub(crate) fn current_window_text_pixel_bounds(&self) -> Rect {
        self.current_text_pixel_bounds
    }

    pub(crate) fn cursor_visual_column_context(&self) -> CursorVisualColumnResolutionContext<'_> {
        CursorVisualColumnResolutionContext::new(
            self.current_window_id,
            self.current_pixel_bounds,
            self.current_row_grid
                .as_ref()
                .map(OutputWindowRowGrid::cursor_rows),
        )
    }

    pub(crate) fn write_row_cursor(
        &mut self,
        row: usize,
        col: u16,
        style: neomacs_display_protocol::frame_glyphs::CursorStyle,
    ) {
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.write_row_cursor(row, col, style);
        }
    }

    pub(crate) fn latest_window_enabled_rows(&self) -> Option<usize> {
        self.windows
            .last()
            .map(OutputWindowGridEntry::enabled_row_count)
    }

    #[cfg(test)]
    pub(crate) fn completed_window_count(&self) -> usize {
        self.windows.len()
    }

    #[cfg(test)]
    pub(crate) fn completed_window_id(&self, index: usize) -> Option<u64> {
        self.windows
            .get(index)
            .map(OutputWindowGridEntry::window_id)
    }

    pub(crate) fn into_window_matrix_entries(self) -> Vec<WindowMatrixEntry> {
        self.windows
            .into_iter()
            .map(OutputWindowGridEntry::into_window_matrix_entry)
            .collect()
    }

    fn decorate_current_row(&mut self, decoration: OutputCurrentRowDecorationRequest) {
        let _ = self.edit_current_row(|row| match decoration {
            OutputCurrentRowDecorationRequest::MarkTruncatedLeft => {
                row.truncated_left = true;
            }
        });
    }

    fn write_row_metrics_at(&mut self, row: usize, metrics: OutputRowMetricsRequest) {
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.write_row_metrics(row, metrics);
        }
    }

    fn replace_current_row(&mut self, source: GlyphRow) {
        let current_row = self.current_row;
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.replace_row(current_row, source);
        }
    }

    fn begin_current_row(&mut self, begin: OutputRowBeginRequest) {
        self.current_row = begin.row;
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.begin_row(begin);
        }
    }

    fn install_complete_row(
        &mut self,
        request: OutputCompleteRowInstallRequest,
        phys_cursor: Option<&mut PhysCursor>,
    ) {
        let row = request.row_index();
        self.begin_current_row(request.begin_request());
        self.replace_current_row(request.into_glyph_row());
        self.finalize_output_row(row, phys_cursor);
    }

    fn finalize_output_row(&mut self, row: usize, phys_cursor: Option<&mut PhysCursor>) {
        if let Some(grid) = self.current_row_grid.as_mut() {
            grid.finalize_row(
                self.current_window_id,
                row,
                self.current_pixel_bounds,
                phys_cursor,
            );
        }
    }
}
