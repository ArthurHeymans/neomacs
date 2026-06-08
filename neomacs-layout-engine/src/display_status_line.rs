//! Display-walker status-line rendering.
//!
//! Mode-line, header-line, and tab-line flow through the
//! walker defined here. The walker produces
//! glyphs via TtyDisplayBackend::produce_glyph and installs the
//! completed row into GlyphMatrixBuilder wholesale; above the
//! backend trait boundary the code is frontend-agnostic, matching
//! GNU Emacs's display_mode_line -> display_mode_element ->
//! display_line -> PRODUCE_GLYPHS architecture.
//!
//! Housed types include status-line face metric helpers. The generic
//! display-row spec, property harvester, and row renderer live in
//! `display_row`.
//!
//! History: this module started as status_line.rs, a divergent
//! parallel implementation of display-line rendering that did not
//! process display properties and dropped doom-modeline's
//! (space :align-to ...) forms. Steps 3.3' through 3.6 of the
//! display-engine unification plan merged it into the backend
//! trait and renamed the file to reflect its new role.

use super::engine::LayoutEngine;
use super::neovm_bridge::ResolvedFace;
#[cfg(test)]
pub(crate) use crate::display_row::{
    OverlayFaceRun, apply_overlay_face_run, parse_overlay_face_runs,
};
pub(crate) use crate::display_row::{StatusLineFace, StatusLineOutputProgress};
use neomacs_display_protocol::face::BoxType;

impl LayoutEngine {
    pub(crate) fn realize_status_line_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        char_w: f32,
        ascent: f32,
        row_height: f32,
    ) -> StatusLineFace {
        let mut face = StatusLineFace::from_resolved(face_id, face);
        self.ensure_status_line_face_metrics(&mut face, char_w, ascent, row_height);
        face
    }

    pub(crate) fn status_line_row_height_for_face(
        &mut self,
        face: &ResolvedFace,
        char_w: f32,
        fallback_ascent: f32,
        fallback_row_height: f32,
    ) -> f32 {
        // GNU Emacs frame.c:1184-1185 — non-window (TTY) frames have
        //   f->column_width = 1;
        //   f->line_height  = 1;
        // and every row (including mode-line, header-line, tab-line) is
        // exactly one character cell tall. Face font metrics are GUI
        // pixel measurements and must not contribute to row sizing on
        // a TTY frame: the layout engine's `char_w` and
        // `fallback_row_height` are both 1.0 in that case
        // (set by `bootstrap_buffers` at neomacs-bin/src/main.rs:1691-1694),
        // so detect the TTY context by the 1.0-cell markers and return
        // the cell height directly. Without this early return, the
        // face-derived `line_height` above was producing a 3-row-tall
        // mode-line region in the TTY pty capture: the mode-line text
        // painted on the first row and the remaining two rows showed up
        // as blank padding that looked like extra echo-area rows.
        if char_w <= 1.0 && fallback_row_height <= 1.0 {
            return fallback_row_height.max(1.0);
        }
        let face =
            self.realize_status_line_face(0, face, char_w, fallback_ascent, fallback_row_height);
        let line_height = (face.font_ascent + face.font_descent as f32)
            .max(1.0)
            .ceil();
        let box_pixels = if face.box_type != BoxType::None && face.box_h_line_width != 0 {
            2.0 * face.box_h_line_width.unsigned_abs() as f32
        } else {
            0.0
        };
        let minimum_row_height = fallback_row_height.ceil().max(1.0);
        (line_height + box_pixels).max(minimum_row_height)
    }

    fn ensure_status_line_face_metrics(
        &mut self,
        face: &mut StatusLineFace,
        fallback_char_width: f32,
        fallback_ascent: f32,
        row_height: f32,
    ) {
        let needs_metrics = face.font_char_width <= 0.0
            || face.font_ascent <= 0.0
            || (face.font_ascent + face.font_descent as f32) <= 0.0;

        if needs_metrics {
            let metrics = self.status_line_font_metrics(face);

            if face.font_char_width <= 0.0 && metrics.char_width > 0.0 {
                face.font_char_width = metrics.char_width;
            }
            if face.font_ascent <= 0.0 && metrics.ascent > 0.0 {
                face.font_ascent = metrics.ascent;
            }
            if (face.font_ascent + face.font_descent as f32) <= 0.0 && metrics.line_height > 0.0 {
                face.font_descent = (metrics.line_height - metrics.ascent).max(0.0).ceil() as i32;
            }
        }

        if face.font_char_width <= 0.0 {
            face.font_char_width = fallback_char_width.max(1.0);
        }
        if face.font_ascent <= 0.0 {
            face.font_ascent = fallback_ascent.max(1.0);
        }
        if (face.font_ascent + face.font_descent as f32) <= 0.0 {
            face.font_descent = (row_height - face.font_ascent).max(0.0).ceil() as i32;
        }
    }
}

#[cfg(test)]
#[path = "display_status_line_test.rs"]
mod tests;
