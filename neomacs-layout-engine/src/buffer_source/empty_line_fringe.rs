//! Empty-line (`~`) fringe indicator rows below buffer end.
//!
//! GNU's `indicate-empty-lines` (vi-tilde-fringe in Doom) draws the periodic
//! `empty-line` bitmap in the LEFT fringe of every visual row that displays no
//! buffer text. The most visible case is the run of blank rows BELOW the
//! buffer's last line: GNU's redisplay tail keeps producing rows until the
//! window's text area is full (`try_window` / the `get_next_display_element`
//! returns-false branch in `display_line`, which sets
//! `row->indicate_empty_line_p` — see src/xdisp.c near `indicate_empty_line_p`).
//!
//! neomacs's buffer-text walk stops once the buffer is exhausted, leaving the
//! remaining text-area rows as bare frame background. This module mirrors GNU's
//! tail: starting just below the last rendered buffer row, it installs synthetic
//! blank [`GlyphRow`]s (each carrying `left_fringe_bitmap = empty-line`) down to
//! the bottom of the text area, never overrunning the mode-line / echo-area
//! boundary.

use crate::display_row::geometry::DisplayRowGeometryState;
use crate::frame_face_arena::FrameFaceAttempt;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, resolve_fringe_indicator_bitmap_index};
use crate::output::row_request::OutputRowLifecycleRequest;
use crate::types::LayoutCharPos0;
use crate::types::WindowParams;
use crate::window_output::TextWindowOutputTarget;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{FringeBitmapInfo, GlyphRow};
use neovm_core::emacs_core::intern::intern;
use neovm_core::emacs_core::{Context, Value};

/// Geometry + policy needed to fill empty-line indicator rows past buffer end.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EmptyLineFringeFillRequest {
    /// `indicate-empty-lines` value: 0 = off, 1 = left fringe, 2 = right fringe.
    /// Only the buffer-local capture (0/1) is produced today; 2 is honored if it
    /// ever appears.
    indicate_empty_lines: i32,
    /// Window-relative row index of the first text-area row.
    display_text_row_base: usize,
    /// Hard cap on visual text rows (`geometry.max_rows`). The mode-line lives
    /// just past the text rows, so the filler must stop at this count — the
    /// pixel guard (`text_y + text_height`) enforces the same boundary in
    /// pixels; whichever hits first wins.
    max_rows: usize,
    /// Top of the text area in absolute frame pixels (matches the frame of
    /// `DisplayRowGeometryState::y`, so they compare directly).
    text_y: f32,
    /// Height of the text area (pixels). Filler stops at `text_y + text_height`,
    /// the mode-line / echo-area boundary.
    text_height: f32,
    /// Per-row height to advance by (the window's default line height).
    char_height: f32,
    /// Per-row ascent for the synthetic rows.
    char_ascent: f32,
    /// Whether this window is a minibuffer (GNU never indicates empty lines in
    /// the mini-window: `!MINI_WINDOW_P (it->w)`).
    is_minibuffer: bool,
    /// End of the accessible buffer (0-based char count). Every filler row
    /// carries `start = end = ZV` with `ends_at_zv` — GNU display_line keeps
    /// producing rows at ZV until the window is full, each with real
    /// MATRIX_ROW_START/END_CHARPOS, so the fillers shift with ZV on edits
    /// exactly like the EOB placeholder.
    zv: LayoutCharPos0,
}

impl EmptyLineFringeFillRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        params: &WindowParams,
        display_text_row_base: usize,
        max_rows: usize,
        text_y: f32,
        text_height: f32,
        char_height: f32,
        char_ascent: f32,
    ) -> Self {
        Self {
            indicate_empty_lines: params.indicate_empty_lines,
            display_text_row_base,
            max_rows,
            text_y,
            text_height,
            char_height,
            char_ascent,
            is_minibuffer: params.kind.is_minibuffer(),
            zv: params.accessible_end_charpos(),
        }
    }

    /// Build directly from the two policy inputs, bypassing `WindowParams`. Used
    /// by unit tests to exercise the side/suppression logic in isolation.
    #[cfg(test)]
    pub(crate) fn from_parts(indicate_empty_lines: i32, is_minibuffer: bool) -> Self {
        Self {
            indicate_empty_lines,
            display_text_row_base: 0,
            max_rows: 10,
            text_y: 0.0,
            text_height: 200.0,
            char_height: 20.0,
            char_ascent: 16.0,
            is_minibuffer,
            zv: LayoutCharPos0::new(0),
        }
    }

    /// The fringe side the empty-line bitmap goes on, or `None` when the feature
    /// is off / suppressed. GNU: `indicate-empty-lines` = `t`/`left` -> left,
    /// `right` -> right.
    fn side(&self) -> Option<EmptyLineFringeSide> {
        if self.is_minibuffer {
            return None;
        }
        match self.indicate_empty_lines {
            1 => Some(EmptyLineFringeSide::Left),
            2 => Some(EmptyLineFringeSide::Right),
            _ => None,
        }
    }

    /// Emit the blank filler rows from below the last buffer row down to the
    /// bottom of the text area. `row_geometry` reflects the position *after* the
    /// last rendered buffer row: `row_geometry.row()` is the next free visual
    /// row (0-based within the text area) and `row_geometry.y()` its top.
    ///
    /// Returns the number of filler rows installed.
    pub(crate) fn fill<B: LayoutBufferView>(
        &self,
        buffer: &B,
        mut output: TextWindowOutputTarget<'_>,
        evaluator: &Context,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceAttempt,
        row_geometry: &DisplayRowGeometryState,
    ) -> usize {
        let Some(side) = self.side() else {
            return 0;
        };
        // Resolve the `empty-line` LOGICAL indicator through GNU's
        // `fringe-indicator-alist` resolver (`get_logical_fringe_bitmap`), not by
        // the hardcoded standard name: Doom rebinds the buffer-local entry to
        // `(empty-line . vi-tilde-fringe-bitmap)`, so this yields the `~` bitmap
        // instead of the dotted standard `empty-line` glyph (the GUI parity fix).
        // The empty-line filler always draws the LEFT/full element (GNU produces
        // these synthetic rows with `right_p = partial_p = 0`).
        let empty_line_sym = Value::from_sym_id(intern("empty-line"));
        let Some(bitmap_index) = resolve_fringe_indicator_bitmap_index(
            buffer,
            evaluator,
            empty_line_sym,
            /* right_p */ false,
            /* partial_p */ false,
        ) else {
            return 0;
        };
        let bitmap_index = u32::from(bitmap_index);
        let char_height = self.char_height.max(1.0);
        let ascent = self.char_ascent.max(0.0).min(char_height);

        // Resolve the `fringe` face once and register it so the renderer can
        // resolve fg/bg for the bitmap quads.
        let resolved = face_resolver.resolve_named_face("fringe");
        let face_id =
            crate::display_row::face_state::stable_face_id_for_resolved(face_ids, &resolved);
        output.install_resolved_face(face_id, &resolved, None);

        let info = FringeBitmapInfo {
            bitmap_index: bitmap_index as u16,
            face_id,
        };

        // Buffer-text rows store `pixel_y` window-relative (absolute frame y
        // minus the window's top); match that so the `FringeBitmap` projects to
        // the right device row.
        let window_y = output.builder().current_window_pixel_bounds().y;

        // The next free visual row and its top y, just below the last buffer
        // row. `row_geometry.y()` is an absolute frame y (it starts at
        // `geometry.text_y`, which is `text_bounds.y + header + tab-line`).
        let mut row = row_geometry.row();
        let mut y = row_geometry.y();
        let bottom_y = self.text_y + self.text_height;

        let mut installed = 0usize;
        // Fill until we run out of visual rows (mode-line guard) or pixels
        // (echo-area / text-area bottom guard). The `+ char_height <= bottom_y`
        // check mirrors GNU's `current_row_is_visible`: a row only counts if it
        // fits entirely within the text area.
        while row < self.max_rows && y + char_height <= bottom_y + 0.5 {
            let display_row_index = self.display_text_row_base + row;
            let mut glyph_row = GlyphRow::new(GlyphRowRole::Text);
            glyph_row.enabled = true;
            glyph_row.displays_text = false;
            glyph_row.ends_at_zv = true;
            let zv = self.zv.get().max(0) as usize;
            glyph_row.start_charpos = zv;
            glyph_row.end_charpos = zv;
            glyph_row.pixel_y = y - window_y;
            glyph_row.height_px = char_height;
            glyph_row.ascent_px = ascent;
            match side {
                EmptyLineFringeSide::Left => glyph_row.left_fringe_bitmap = Some(info),
                EmptyLineFringeSide::Right => glyph_row.right_fringe_bitmap = Some(info),
            }

            output
                .builder()
                .install_output_row_lifecycle(OutputRowLifecycleRequest::complete(
                    display_row_index,
                    GlyphRowRole::Text,
                    false,
                    glyph_row,
                ));

            installed += 1;
            row += 1;
            y += char_height;
        }
        installed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EmptyLineFringeSide {
    Left,
    Right,
}

#[cfg(test)]
#[path = "empty_line_fringe_test.rs"]
mod tests;
