use crate::display_cursor::cursor_window_matches_current;
use neomacs_display_protocol::frame_glyphs::PhysCursor;
use neomacs_display_protocol::glyph_matrix::GlyphRow;
use neomacs_display_protocol::types::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GlyphRowFinalizationContext {
    pub(crate) window_id: u64,
    pub(crate) row: usize,
    pub(crate) window_pixel_bounds: Rect,
}

impl GlyphRowFinalizationContext {
    pub(crate) fn new(window_id: u64, row: usize, window_pixel_bounds: Rect) -> Self {
        Self {
            window_id,
            row,
            window_pixel_bounds,
        }
    }

    pub(crate) fn finalize_row(
        self,
        row: &mut GlyphRow,
        matrix_ncols: usize,
        phys_cursor: Option<&mut PhysCursor>,
    ) {
        GlyphRowFinalizer::new(self, matrix_ncols, phys_cursor).finalize(row);
    }

    fn cursor_matches(self, cursor: &PhysCursor) -> bool {
        cursor_window_matches_current(cursor.window_id, self.window_id) && cursor.row == self.row
    }
}

pub(crate) struct GlyphRowFinalizer<'cursor> {
    context: GlyphRowFinalizationContext,
    matrix_ncols: usize,
    phys_cursor: Option<&'cursor mut PhysCursor>,
}

impl<'cursor> GlyphRowFinalizer<'cursor> {
    pub(crate) fn new(
        context: GlyphRowFinalizationContext,
        matrix_ncols: usize,
        phys_cursor: Option<&'cursor mut PhysCursor>,
    ) -> Self {
        Self {
            context,
            matrix_ncols,
            phys_cursor,
        }
    }

    pub(crate) fn finalize(&mut self, row: &mut GlyphRow) {
        let phys_cursor_col = self
            .phys_cursor
            .as_deref()
            .filter(|cursor| self.context.cursor_matches(cursor))
            .map(|cursor| cursor.col);

        let remapped_cursor_col = crate::glyph_row_writer::reorder_row_bidi(row, phys_cursor_col);
        self.apply_phys_cursor_remap(remapped_cursor_col);
    }

    fn apply_phys_cursor_remap(&mut self, remapped_cursor_col: Option<u16>) {
        let Some(col) = remapped_cursor_col else {
            return;
        };
        let Some(cursor) = self.phys_cursor.as_deref_mut() else {
            return;
        };
        if !self.context.cursor_matches(cursor) {
            return;
        }

        cursor.col = col;
        cursor.slot_id.col = col;
        if self.matrix_ncols > 0 {
            let char_w = self.context.window_pixel_bounds.width / self.matrix_ncols as f32;
            cursor.x = self.context.window_pixel_bounds.x + col as f32 * char_w;
        }
    }
}

#[cfg(test)]
#[path = "display_row_finalizer_test.rs"]
mod tests;
