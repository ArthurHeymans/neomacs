//! Row finalization after source-specific acquisition has ended.
//!
//! [`DisplayRowLineEndFinalizer`] applies GNU line-end semantics while the row
//! is still in logical order. [`GlyphRowFinalizer`] performs presentation
//! commit work (bidi reorder, pointer runs, and cursor remapping) after the
//! completed row is installed. Keeping both phases here gives display sources
//! one narrow destination without mixing source acquisition into row policy.

use crate::display_cursor::cursor_window_matches_current;
use crate::display_item::{DisplayLineHeightPolicy, DisplayRowBreak};
use crate::display_row_face_state::DisplayRowFace;
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::glyph_row_writer::push_stretch_to_area;
use neomacs_display_protocol::frame_glyphs::PhysCursor;
use neomacs_display_protocol::glyph_matrix::{
    Glyph, GlyphArea, GlyphRow, GlyphType, NO_BUFFER_POSITION_CHARPOS,
};
use neomacs_display_protocol::types::{Color, FaceId, Rect};

/// Finalizes the semantic effects of a typed newline after every source has
/// converged into the shared display-item stream. This is deliberately below
/// buffer text, Lisp strings, display replacements, and overlay strings: GNU's
/// `line-height` and `:extend` behavior depends on the completed display row,
/// not on which source happened to produce the newline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowLineEndFinalizer {
    row_break: DisplayRowBreak,
    row_break_face_id: FaceId,
    remaining_width_px: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
    base_background: Color,
}

impl DisplayRowLineEndFinalizer {
    pub(crate) fn new(
        row_break: DisplayRowBreak,
        row_break_face_id: FaceId,
        remaining_width_px: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        base_background: Color,
    ) -> Self {
        Self {
            row_break,
            row_break_face_id,
            remaining_width_px,
            fallback_metrics,
            base_background,
        }
    }

    pub(crate) fn finalize(self, row: &mut GlyphRow, faces: &[DisplayRowFace]) {
        if self.row_break.line_height == DisplayLineHeightPolicy::ContentOnly {
            let (height, ascent) = display_row_visible_content_metrics(
                row,
                faces,
                self.fallback_metrics.row_height(),
                self.fallback_metrics.ascent(),
            );
            row.height_px = height;
            row.ascent_px = ascent;
        }

        let Some(extend_face) = faces
            .iter()
            .find(|face| face.face_id == self.row_break_face_id && face.extend)
        else {
            return;
        };
        if self.remaining_width_px <= 0.0 || extend_face.background == self.base_background {
            return;
        }
        RowExtendFill::new(
            extend_face.background,
            extend_face.face_id,
            self.remaining_width_px,
            row.height_px.max(1.0),
            row.ascent_px.max(0.0).min(row.height_px.max(1.0)),
            extend_face
                .metrics
                .char_width_px(self.fallback_metrics.char_width()),
        )
        .apply_to(row);
    }
}

/// Metrics contributed by visible glyphs, excluding the row's default
/// minimum. GNU keeps these as iterator `max_ascent`/`max_descent` values.
fn display_row_visible_content_metrics(
    row: &GlyphRow,
    faces: &[DisplayRowFace],
    fallback_height: f32,
    fallback_ascent: f32,
) -> (f32, f32) {
    let mut max_ascent = 0.0f32;
    let mut max_descent = 0.0f32;
    let mut saw_glyph = false;
    for glyph in row.glyphs.iter().flatten() {
        saw_glyph = true;
        let (height, ascent) =
            display_glyph_visible_metrics(glyph, faces, fallback_height, fallback_ascent);
        let ascent = ascent.max(0.0).min(height);
        let descent = (height - ascent).max(0.0);
        max_ascent = max_ascent.max((ascent - glyph.vertical_offset_px).max(0.0));
        max_descent = max_descent.max((descent + glyph.vertical_offset_px).max(0.0));
    }
    if saw_glyph {
        let height = (max_ascent + max_descent).max(1.0);
        (height, max_ascent.min(height))
    } else {
        (1.0, 1.0)
    }
}

fn display_glyph_visible_metrics(
    glyph: &Glyph,
    faces: &[DisplayRowFace],
    fallback_height: f32,
    fallback_ascent: f32,
) -> (f32, f32) {
    if glyph.pixel_height > 0.0 {
        return (glyph.pixel_height, glyph.pixel_ascent);
    }
    faces
        .iter()
        .find(|face| face.face_id == glyph.face_id)
        .map(|face| (face.metrics.line_height_px(), face.metrics.ascent_px()))
        .unwrap_or_else(|| {
            let height = fallback_height.max(1.0);
            (height, fallback_ascent.max(0.0).min(height))
        })
}

/// Geometry + face payload for GNU `extend_face_to_end_of_line`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowExtendFill {
    bg: Color,
    face_id: FaceId,
    width_px: f32,
    height_px: f32,
    ascent_px: f32,
    char_width: f32,
}

impl RowExtendFill {
    pub(crate) fn new(
        bg: Color,
        face_id: FaceId,
        width_px: f32,
        height_px: f32,
        ascent_px: f32,
        char_width: f32,
    ) -> Self {
        Self {
            bg,
            face_id,
            width_px,
            height_px,
            ascent_px,
            char_width,
        }
    }

    /// Apply the fill to `row`. The operation is idempotent so the unified
    /// item renderer can coexist with an older source caller while that caller
    /// is migrated to the shared row-finalization seam.
    pub(crate) fn apply_to(self, row: &mut GlyphRow) -> bool {
        if row.reversed_p || self.width_px <= 0.0 {
            return false;
        }
        let text_index = GlyphArea::Text.index();
        if row.glyphs[text_index]
            .last()
            .is_some_and(|glyph| self.matches_existing_fill(glyph))
        {
            return true;
        }
        if row.glyphs[text_index].is_empty() {
            row.glyphs[text_index].push(
                Glyph::char(' ', self.face_id, NO_BUFFER_POSITION_CHARPOS)
                    .with_pixel_width(self.char_width.max(1.0)),
            );
            row.displays_text = true;
        }
        push_stretch_to_area(
            row,
            text_index,
            self.width_cols(),
            self.face_id,
            self.width_px,
            self.height_px,
            self.ascent_px,
        );
        if let Some(last) = row.glyphs[text_index].last_mut() {
            last.charpos = NO_BUFFER_POSITION_CHARPOS;
        }
        true
    }

    pub(crate) fn width_cols(self) -> u16 {
        let char_width = self.char_width.max(1.0);
        ((self.width_px / char_width).ceil() as i64).clamp(1, u16::MAX as i64) as u16
    }

    fn matches_existing_fill(self, glyph: &Glyph) -> bool {
        const PIXEL_TOLERANCE: f32 = 0.01;
        glyph.charpos == NO_BUFFER_POSITION_CHARPOS
            && glyph.face_id == self.face_id
            && matches!(
                glyph.glyph_type,
                GlyphType::Stretch { width_cols } if width_cols == self.width_cols()
            )
            && (glyph.pixel_width - self.width_px).abs() <= PIXEL_TOLERANCE
            && (glyph.pixel_height - self.height_px).abs() <= PIXEL_TOLERANCE
            && (glyph.pixel_ascent - self.ascent_px).abs() <= PIXEL_TOLERANCE
    }
}

#[cfg(test)]
thread_local! {
    static POINTER_RUN_GLYPH_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_pointer_run_glyph_visits() {
    POINTER_RUN_GLYPH_VISITS.with(|visits| visits.set(0));
}

#[cfg(test)]
pub(crate) fn pointer_run_glyph_visits() -> usize {
    POINTER_RUN_GLYPH_VISITS.with(std::cell::Cell::get)
}

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
        cursor_window_matches_current(cursor.window_id.get(), self.window_id)
            && cursor.row == self.row
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
        let char_width = if self.matrix_ncols > 0 {
            self.context.window_pixel_bounds.width / self.matrix_ncols as f32
        } else {
            1.0
        };
        row.rebuild_pointer_runs(char_width, self.context.window_pixel_bounds.width);
        #[cfg(test)]
        POINTER_RUN_GLYPH_VISITS.with(|visits| {
            visits.set(visits.get().saturating_add(row.total_glyphs()));
        });
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

#[cfg(test)]
#[path = "display_row_extend_fill_test.rs"]
mod extend_fill_tests;
