#![allow(dead_code)]

use crate::composition::{base_width_cols, continues_cluster, continues_complex_run};
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLength, DisplayLengthExpr, DisplaySourcePosition,
    DisplayStretch, DisplayStretchWidth, RenderFaceRef, SourceSpan,
};
use crate::matrix_builder::GlyphMatrixBuilder;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow, GlyphType};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowLayout {
    pub(crate) role: GlyphRowRole,
    pub(crate) y_px: f32,
    pub(crate) width_px: f32,
    pub(crate) height_px: f32,
    pub(crate) ascent_px: f32,
    pub(crate) char_width_px: f32,
    pub(crate) base_face: RenderFaceRef,
    pub(crate) symbol_values: std::collections::HashMap<String, DisplayLengthExpr>,
}

pub(crate) trait DisplayGlyphMeasurer {
    fn glyph_advance_px(
        &mut self,
        ch: char,
        face_id: u32,
        columns: u8,
        fallback_advance_px: f32,
    ) -> Option<f32>;
}

pub(crate) struct DisplayRowBuilder<'a> {
    layout: DisplayRowLayout,
    row: GlyphRow,
    glyph_measurer: Option<&'a mut dyn DisplayGlyphMeasurer>,
}

impl DisplayRowBuilder<'_> {
    pub(crate) fn new(layout: DisplayRowLayout) -> Self {
        let mut row = GlyphRow::new(layout.role);
        row.enabled = true;
        row.pixel_y = layout.y_px;
        row.height_px = layout.height_px.max(1.0);
        row.ascent_px = layout.ascent_px.max(0.0).min(row.height_px);
        Self {
            layout,
            row,
            glyph_measurer: None,
        }
    }
}

impl<'a> DisplayRowBuilder<'a> {
    pub(crate) fn with_glyph_measurer(
        layout: DisplayRowLayout,
        glyph_measurer: &'a mut dyn DisplayGlyphMeasurer,
    ) -> Self {
        let mut builder = Self::new(layout);
        builder.glyph_measurer = Some(glyph_measurer);
        builder
    }

    pub(crate) fn push_item(&mut self, item: DisplayItem) {
        let face_id = self.face_id(item.face);
        match item.kind {
            DisplayItemKind::TextRun(run) => {
                let mut charpos = source_span_start_char(&item.span);
                for ch in run.text.chars() {
                    self.push_text_char(ch, face_id, charpos);
                    charpos += 1;
                }
            }
            DisplayItemKind::Stretch(stretch) => self.push_stretch(stretch, face_id),
            DisplayItemKind::Image(image) => {
                let image_id = image.image_id.max(0);
                let glyph = Glyph {
                    glyph_type: GlyphType::Image { image_id },
                    face_id,
                    charpos: source_span_start_char(&item.span),
                    bidi_level: 0,
                    wide: false,
                    pixel_width: 0.0,
                    pixel_height: 0.0,
                    pixel_ascent: 0.0,
                    padding: false,
                };
                self.row.glyphs[GlyphArea::Text.index()].push(glyph);
                self.row.displays_text = true;
            }
            DisplayItemKind::ControlChar { ch } => {
                GlyphMatrixBuilder::push_char_to_row(&mut self.row, ch, face_id, 0, 0.0);
            }
            DisplayItemKind::Glyphless(glyphless) => {
                let glyph = Glyph {
                    glyph_type: GlyphType::Glyphless { ch: glyphless.ch },
                    face_id,
                    charpos: source_span_start_char(&item.span),
                    bidi_level: 0,
                    wide: false,
                    pixel_width: 0.0,
                    pixel_height: 0.0,
                    pixel_ascent: 0.0,
                    padding: false,
                };
                self.row.glyphs[GlyphArea::Text.index()].push(glyph);
                self.row.displays_text = true;
            }
            DisplayItemKind::Video(_)
            | DisplayItemKind::Xwidget(_)
            | DisplayItemKind::RowBreak(_)
            | DisplayItemKind::CursorAnchor(_)
            | DisplayItemKind::HitTestAnchor(_) => {}
        }
    }

    pub(crate) fn finish(mut self) -> GlyphRow {
        GlyphMatrixBuilder::normalize_external_row(&mut self.row);
        self.row
    }

    fn push_text_char(&mut self, ch: char, face_id: u32, charpos: usize) {
        let tail = GlyphMatrixBuilder::last_text_cluster_tail_in_row(&self.row);
        if continues_cluster(ch, tail) {
            GlyphMatrixBuilder::push_cluster_continuation_to_row(
                &mut self.row,
                ch,
                face_id,
                charpos,
            );
            return;
        }
        if continues_complex_run(ch, tail) {
            let advance = self.glyph_advance_px(ch, face_id, 1);
            GlyphMatrixBuilder::push_run_member_to_row(
                &mut self.row,
                ch,
                face_id,
                charpos,
                advance,
            );
            return;
        }
        let cols = base_width_cols(ch);
        let advance = self.glyph_advance_px(ch, face_id, cols);
        if cols > 1 {
            GlyphMatrixBuilder::push_wide_char_to_row(&mut self.row, ch, face_id, charpos, advance);
        } else {
            GlyphMatrixBuilder::push_char_to_row(&mut self.row, ch, face_id, charpos, advance);
        }
    }

    fn glyph_advance_px(&mut self, ch: char, face_id: u32, columns: u8) -> f32 {
        let fallback = self.layout.char_width_px.max(1.0) * f32::from(columns.max(1));
        self.glyph_measurer
            .as_mut()
            .and_then(|measurer| measurer.glyph_advance_px(ch, face_id, columns, fallback))
            .filter(|advance| advance.is_finite() && *advance >= 0.0)
            .unwrap_or(fallback)
    }

    fn push_stretch(&mut self, stretch: DisplayStretch, face_id: u32) {
        let Some((width_cols, pixel_width)) = self.stretch_width(&stretch.width) else {
            return;
        };
        let pixel_height = stretch
            .height
            .as_ref()
            .and_then(|length| self.length_pixels(length, self.layout.height_px))
            .unwrap_or(0.0);
        let pixel_ascent = stretch
            .ascent
            .as_ref()
            .and_then(|length| self.length_pixels(length, self.layout.ascent_px))
            .unwrap_or(0.0);

        GlyphMatrixBuilder::push_stretch_to_row(
            &mut self.row,
            width_cols,
            face_id,
            pixel_width,
            pixel_height,
            pixel_ascent,
        );
    }

    fn stretch_width(&self, width: &DisplayStretchWidth) -> Option<(u16, f32)> {
        match width {
            DisplayStretchWidth::Length(length) => {
                let pixels = self.length_pixels(length, self.layout.char_width_px)?;
                let cols = (pixels / self.layout.char_width_px.max(1.0))
                    .round()
                    .max(1.0) as u16;
                Some((cols, pixels))
            }
            DisplayStretchWidth::AlignTo(expr) => {
                let target = self.length_expr_pixels(expr)?;
                let current = self.current_text_width_px();
                let pixels = (target - current).max(0.0);
                let cols = (pixels / self.layout.char_width_px.max(1.0)).round() as u16;
                Some((cols, pixels))
            }
        }
    }

    fn current_text_width_px(&self) -> f32 {
        self.row.glyphs[GlyphArea::Text.index()]
            .iter()
            .map(|glyph| match glyph.glyph_type {
                GlyphType::Stretch { width_cols } => {
                    if glyph.pixel_width > 0.0 {
                        glyph.pixel_width
                    } else {
                        f32::from(width_cols) * self.layout.char_width_px.max(1.0)
                    }
                }
                _ if glyph.pixel_width > 0.0 => glyph.pixel_width,
                _ if glyph.wide => self.layout.char_width_px.max(1.0) * 2.0,
                _ if glyph.padding => 0.0,
                _ => self.layout.char_width_px.max(1.0),
            })
            .sum()
    }

    fn length_pixels(&self, length: &DisplayLength, em_px: f32) -> Option<f32> {
        match length {
            DisplayLength::Columns(cols) => Some(f32::from(*cols) * self.layout.char_width_px),
            DisplayLength::Pixels(px) => Some(*px),
            DisplayLength::Em(em) => Some(*em * em_px.max(1.0)),
            DisplayLength::Expr(expr) => self.length_expr_pixels(expr),
        }
        .filter(|pixels| pixels.is_finite() && *pixels >= 0.0)
    }

    fn length_expr_pixels(&self, expr: &DisplayLengthExpr) -> Option<f32> {
        match expr {
            DisplayLengthExpr::Pixels(px) => Some(*px),
            DisplayLengthExpr::Em(em) => Some(*em * self.layout.char_width_px.max(1.0)),
            DisplayLengthExpr::Symbol(symbol) => match symbol {
                crate::display_item::DisplayLengthSymbol::Width => Some(self.layout.char_width_px),
                crate::display_item::DisplayLengthSymbol::Height => Some(self.layout.height_px),
                crate::display_item::DisplayLengthSymbol::Text
                | crate::display_item::DisplayLengthSymbol::Left => Some(0.0),
                crate::display_item::DisplayLengthSymbol::Right => Some(self.layout.width_px),
                crate::display_item::DisplayLengthSymbol::Center => {
                    Some(self.layout.width_px / 2.0)
                }
                crate::display_item::DisplayLengthSymbol::LeftFringe
                | crate::display_item::DisplayLengthSymbol::RightFringe
                | crate::display_item::DisplayLengthSymbol::LeftMargin
                | crate::display_item::DisplayLengthSymbol::RightMargin
                | crate::display_item::DisplayLengthSymbol::ScrollBar => Some(0.0),
            },
            DisplayLengthExpr::Variable(name) => self
                .layout
                .symbol_values
                .get(name.as_ref())
                .and_then(|expr| self.length_expr_pixels(expr)),
            DisplayLengthExpr::Add(parts) => parts.iter().try_fold(0.0, |sum, part| {
                self.length_expr_pixels(part).map(|value| sum + value)
            }),
            DisplayLengthExpr::Sub(parts) => {
                let mut iter = parts.iter();
                let first = self.length_expr_pixels(iter.next()?)?;
                iter.try_fold(first, |sum, part| {
                    self.length_expr_pixels(part).map(|value| sum - value)
                })
            }
        }
        .filter(|pixels| pixels.is_finite())
    }

    fn face_id(&self, face: RenderFaceRef) -> u32 {
        match face {
            RenderFaceRef::FaceId(face_id) => face_id,
            RenderFaceRef::Inherit => match self.layout.base_face {
                RenderFaceRef::FaceId(face_id) => face_id,
                RenderFaceRef::Inherit => 0,
            },
        }
    }
}

fn source_span_start_char(span: &SourceSpan) -> usize {
    match &span.start {
        DisplaySourcePosition::Buffer { char_pos, .. } => char_pos.get(),
        DisplaySourcePosition::LispString { char_index, .. } => *char_index,
        DisplaySourcePosition::Synthetic { offset, .. } => *offset,
    }
}

#[cfg(test)]
#[path = "display_row_builder_test.rs"]
mod tests;
