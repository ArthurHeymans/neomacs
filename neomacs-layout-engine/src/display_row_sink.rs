//! Neutral glyph-row accumulation shared by display backends.
//!
//! This module owns no measurement policy. GUI and TTY backends measure
//! advances differently, then pass the measured width here so row
//! materialization can use authoritative glyph geometry.

use crate::display_backend::GlyphKind;
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphRow};

#[derive(Default)]
pub(crate) struct GlyphRowSink {
    pending_glyphs: Vec<Glyph>,
    pending_rows: Vec<GlyphRow>,
}

impl GlyphRowSink {
    pub(crate) fn produce_glyph(
        &mut self,
        kind: GlyphKind,
        face: &Face,
        charpos: usize,
        stretch_cell_width_px: f32,
        pixel_width: f32,
    ) {
        let face_id = face.id;
        let pixel_width = if pixel_width.is_finite() && pixel_width > 0.0 {
            pixel_width
        } else {
            0.0
        };
        let glyph = match kind {
            GlyphKind::Char(ch) => Glyph::char(ch, face_id, charpos).with_pixel_width(pixel_width),
            GlyphKind::Glyphless(ch) => {
                Glyph::char(ch, face_id, charpos).with_pixel_width(pixel_width)
            }
            GlyphKind::Stretch { width_px, .. } => {
                let cols = (width_px / stretch_cell_width_px.max(1.0)).round() as u16;
                Glyph::stretch(cols.max(1), face_id).with_pixel_width(width_px)
            }
        };
        self.pending_glyphs.push(glyph);
    }

    pub(crate) fn finish_row(&mut self, mut row: GlyphRow) {
        let text_glyphs = std::mem::take(&mut self.pending_glyphs);
        row.glyphs[1] = text_glyphs;
        self.pending_rows.push(row);
    }

    pub(crate) fn take_rows(&mut self) -> Vec<GlyphRow> {
        std::mem::take(&mut self.pending_rows)
    }

    pub(crate) fn pending_glyphs(&self) -> &[Glyph] {
        &self.pending_glyphs
    }
}
