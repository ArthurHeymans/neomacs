use crate::display_item::RenderFaceRef;
use crate::display_row_builder::{
    DisplayGlyphMeasurer, DisplayRowBuilder, DisplayRowLayout, DisplayTabPolicy,
};
use crate::display_source::{
    DisplayItemFaceResolver, DisplayItemSource, DisplaySourceContext, LispStringSourceCursor,
    parse_display_length_expr,
};
use crate::engine::LayoutEngine;
use crate::font_metrics::FontMetricsService;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::FaceResolver;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::face::{BoxType, Face, FaceAttributes, UnderlineStyle};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow, GlyphType};
use neomacs_display_protocol::types::Color;
use neovm_core::emacs_core::Value;

fn underline_style_from_code(code: u8) -> UnderlineStyle {
    UnderlineStyle::from_gnu_code(code).unwrap_or_default()
}

/// Shared render-facing face spec for all status-line backends.
#[derive(Debug, Clone)]
pub(crate) struct StatusLineFace {
    pub(crate) face_id: u32,
    pub(crate) foreground: Color,
    pub(crate) background: Color,
    pub(crate) use_default_foreground: bool,
    pub(crate) use_default_background: bool,
    pub(crate) font_family: String,
    pub(crate) font_file_path: Option<String>,
    pub(crate) font_weight: u16,
    pub(crate) italic: bool,
    pub(crate) font_size: f32,
    pub(crate) underline_style: u8,
    pub(crate) underline_color: Option<Color>,
    pub(crate) strike_through: bool,
    pub(crate) strike_through_color: Option<Color>,
    pub(crate) overline: bool,
    pub(crate) overline_color: Option<Color>,
    pub(crate) box_type: BoxType,
    pub(crate) box_color: Option<Color>,
    pub(crate) box_line_width: i32,
    pub(crate) box_corner_radius: i32,
    pub(crate) box_border_style: u32,
    pub(crate) box_border_speed: f32,
    pub(crate) box_color2: Option<Color>,
    pub(crate) box_h_line_width: i32,
    pub(crate) terminal_inverse_video: bool,
    pub(crate) font_char_width: f32,
    pub(crate) font_ascent: f32,
    pub(crate) font_descent: i32,
    pub(crate) underline_position: i32,
    pub(crate) underline_thickness: i32,
}

impl StatusLineFace {
    pub(crate) fn from_resolved(face_id: u32, face: &ResolvedFace) -> Self {
        let font_descent = if face.font_line_height > 0.0 && face.font_ascent > 0.0 {
            (face.font_line_height - face.font_ascent).max(0.0).ceil() as i32
        } else {
            0
        };
        let box_type = BoxType::from_gnu_code(face.box_type).unwrap_or_default();
        Self {
            face_id,
            foreground: Color::from_pixel(face.fg),
            background: Color::from_pixel(face.bg),
            use_default_foreground: face.use_default_foreground,
            use_default_background: face.use_default_background,
            font_family: if face.font_family.is_empty() {
                "monospace".to_string()
            } else {
                face.font_family.clone()
            },
            font_file_path: None,
            font_weight: face.font_weight,
            italic: face.italic,
            font_size: face.font_size,
            underline_style: face.underline_style,
            underline_color: (face.underline_style > 0)
                .then(|| Color::from_pixel(face.underline_color)),
            strike_through: face.strike_through,
            strike_through_color: face
                .strike_through
                .then(|| Color::from_pixel(face.strike_through_color)),
            overline: face.overline,
            overline_color: face
                .overline
                .then(|| Color::from_pixel(face.overline_color)),
            box_type,
            box_color: (box_type != BoxType::None && face.box_color != 0)
                .then(|| Color::from_pixel(face.box_color)),
            box_line_width: face.box_line_width,
            box_corner_radius: 0,
            box_border_style: 0,
            box_border_speed: 1.0,
            box_color2: None,
            box_h_line_width: face.box_line_width,
            terminal_inverse_video: face.terminal_inverse_video,
            font_char_width: face.font_char_width,
            font_ascent: face.font_ascent,
            font_descent,
            underline_position: 1,
            underline_thickness: 1,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_color_override(
        &self,
        face_id: u32,
        fg: Option<Color>,
        bg: Option<Color>,
    ) -> Self {
        let mut face = self.clone();
        face.face_id = face_id;
        if let Some(color) = fg {
            face.foreground = color;
            face.use_default_foreground = false;
        }
        if let Some(color) = bg {
            face.background = color;
            face.use_default_background = false;
        }
        if fg.is_some() || bg.is_some() {
            face.terminal_inverse_video = false;
        }
        face
    }

    pub(crate) fn render_face(&self) -> Face {
        let underline_style = underline_style_from_code(self.underline_style);
        let mut attrs = FaceAttributes::empty();
        if self.font_weight >= 700 {
            attrs |= FaceAttributes::BOLD;
        }
        if self.italic {
            attrs |= FaceAttributes::ITALIC;
        }
        if underline_style != UnderlineStyle::None {
            attrs |= FaceAttributes::UNDERLINE;
        }
        if self.strike_through {
            attrs |= FaceAttributes::STRIKE_THROUGH;
        }
        if self.overline {
            attrs |= FaceAttributes::OVERLINE;
        }
        if !matches!(self.box_type, BoxType::None) {
            attrs |= FaceAttributes::BOX;
        }
        if self.terminal_inverse_video {
            attrs |= FaceAttributes::INVERSE;
        }
        Face {
            id: self.face_id,
            foreground: self.foreground,
            background: self.background,
            use_default_foreground: self.use_default_foreground,
            use_default_background: self.use_default_background,
            underline_color: self.underline_color,
            overline_color: self.overline_color,
            strike_through_color: self.strike_through_color,
            box_color: self.box_color,
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight,
            attributes: attrs,
            underline_style,
            box_type: self.box_type,
            box_line_width: self.box_line_width,
            box_corner_radius: self.box_corner_radius,
            box_border_style: self.box_border_style,
            box_border_speed: self.box_border_speed,
            box_color2: self.box_color2,
            font_file_path: self.font_file_path.clone(),
            font_ascent: self.font_ascent as i32,
            font_descent: self.font_descent,
            underline_position: self.underline_position.max(1),
            underline_thickness: self.underline_thickness.max(1),
            background_gradient: None,
        }
    }
}

struct StatusLineGlyphMeasurer<'a> {
    faces: &'a [StatusLineFace],
    font_metrics: Option<&'a mut FontMetricsService>,
    fallback_char_width: f32,
}

impl<'a> StatusLineGlyphMeasurer<'a> {
    fn new(
        faces: &'a [StatusLineFace],
        font_metrics: Option<&'a mut FontMetricsService>,
        fallback_char_width: f32,
    ) -> Self {
        Self {
            faces,
            font_metrics,
            fallback_char_width,
        }
    }

    fn face(&self, face_id: u32) -> Option<&StatusLineFace> {
        self.faces.iter().find(|face| face.face_id == face_id)
    }
}

impl DisplayGlyphMeasurer for StatusLineGlyphMeasurer<'_> {
    fn glyph_advance_px(
        &mut self,
        ch: char,
        face_id: u32,
        columns: u8,
        fallback_advance_px: f32,
    ) -> Option<f32> {
        if columns == 0 {
            return Some(0.0);
        }

        let face = self.face(face_id)?;
        let face_char_width = face.font_char_width.max(self.fallback_char_width).max(1.0);
        let min_advance = f32::from(columns) * face_char_width;
        let font_family = face.font_family.clone();
        let font_weight = face.font_weight;
        let italic = face.italic;
        let font_size = face.font_size.max(1.0);
        let measured = self
            .font_metrics
            .as_mut()
            .map(|svc| svc.char_width(ch, &font_family, font_weight, italic, font_size));

        Some(snap_glyph_advance(
            measured.unwrap_or(fallback_advance_px.max(min_advance)),
            min_advance,
        ))
    }
}

fn snap_glyph_advance(advance: f32, min_advance: f32) -> f32 {
    let snapped_min = min_advance.round().max(1.0);
    if !advance.is_finite() || advance <= 0.0 {
        return snapped_min;
    }
    advance.round().max(snapped_min)
}

/// A face run within an overlay/display string: byte offset + fg/bg colors + face_id.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct OverlayFaceRun {
    pub byte_offset: u16,
    pub fg: u32,
    pub bg: u32,
    #[cfg(test)]
    /// Emacs face ID for full face attribute resolution via FFI
    pub extend: bool,
    /// Emacs face ID for full face attribute resolution via FFI
    pub face_id: u32,
}

/// Parse face runs appended after text in a buffer.
/// Runs are stored as 14-byte records: u16 byte_offset + u32 fg + u32 bg + u32 face_id.
#[cfg(test)]
pub(crate) fn parse_overlay_face_runs(
    buf: &[u8],
    text_len: usize,
    nruns: i32,
) -> Vec<OverlayFaceRun> {
    let mut runs = Vec::with_capacity(nruns as usize);
    let runs_start = text_len;
    for ri in 0..nruns as usize {
        let off = runs_start + ri * 14;
        if off + 14 <= buf.len() {
            let byte_offset = u16::from_ne_bytes([buf[off], buf[off + 1]]);
            let fg = u32::from_ne_bytes([buf[off + 2], buf[off + 3], buf[off + 4], buf[off + 5]]);
            let raw_bg =
                u32::from_ne_bytes([buf[off + 6], buf[off + 7], buf[off + 8], buf[off + 9]]);
            #[cfg(test)]
            let extend = (raw_bg & 0x80000000) != 0;
            let bg = raw_bg & 0x00FFFFFF;
            let face_id =
                u32::from_ne_bytes([buf[off + 10], buf[off + 11], buf[off + 12], buf[off + 13]]);
            runs.push(OverlayFaceRun {
                byte_offset,
                fg,
                bg,
                #[cfg(test)]
                extend,
                face_id,
            });
        }
    }
    runs
}

/// Apply the face run covering the current byte index.
/// Returns the updated current_run index.
#[cfg(test)]
pub(crate) fn apply_overlay_face_run(
    runs: &[OverlayFaceRun],
    byte_idx: usize,
    current_run: usize,
) -> usize {
    let mut cr = current_run;
    // Advance to the correct run
    while cr + 1 < runs.len() && byte_idx >= runs[cr + 1].byte_offset as usize {
        cr += 1;
    }
    if byte_idx >= runs[cr].byte_offset as usize {
        // Pre-advance if next run starts at next byte
        if cr + 1 < runs.len() && byte_idx + 1 >= runs[cr + 1].byte_offset as usize {
            cr += 1;
        }
    }
    cr
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StatusLineOutputProgress {
    pub end_x: f32,
    pub end_col: i64,
    pub y: f32,
    pub height: f32,
}

pub(crate) struct RenderedDisplaySourceRow {
    pub(crate) row: GlyphRow,
    pub(crate) progress: StatusLineOutputProgress,
    pub(crate) faces: Vec<Face>,
}

pub(crate) fn install_rendered_display_source_row(
    builder: &mut GlyphMatrixBuilder,
    rendered: &RenderedDisplaySourceRow,
    matrix_row: usize,
) {
    for face in &rendered.faces {
        builder.insert_face(face.id, face.clone());
    }
    builder.begin_row(matrix_row, rendered.row.role);
    builder.install_prebuilt_current_row(&rendered.row);
    builder.end_prebuilt_row();
}

fn same_resolved_face(lhs: &ResolvedFace, rhs: &ResolvedFace) -> bool {
    lhs.fg == rhs.fg
        && lhs.bg == rhs.bg
        && lhs.font_family == rhs.font_family
        && lhs.font_weight == rhs.font_weight
        && lhs.italic == rhs.italic
        && (lhs.font_size - rhs.font_size).abs() <= f32::EPSILON
        && lhs.underline_style == rhs.underline_style
        && lhs.underline_color == rhs.underline_color
        && lhs.strike_through == rhs.strike_through
        && lhs.strike_through_color == rhs.strike_through_color
        && lhs.overline == rhs.overline
        && lhs.overline_color == rhs.overline_color
        && lhs.box_type == rhs.box_type
        && lhs.box_color == rhs.box_color
        && lhs.box_line_width == rhs.box_line_width
        && lhs.extend == rhs.extend
        && lhs.terminal_inverse_video == rhs.terminal_inverse_video
}

fn display_source_row_progress(
    row: &GlyphRow,
    width: f32,
    char_width: f32,
    y: f32,
    height: f32,
) -> StatusLineOutputProgress {
    let fallback = char_width.max(1.0);
    let end_x: f32 = row.glyphs[GlyphArea::Text.index()]
        .iter()
        .map(|glyph| match glyph.glyph_type {
            GlyphType::Stretch { width_cols } => {
                if glyph.pixel_width > 0.0 {
                    glyph.pixel_width
                } else {
                    f32::from(width_cols) * fallback
                }
            }
            _ if glyph.padding => 0.0,
            _ if glyph.pixel_width > 0.0 => glyph.pixel_width,
            _ if glyph.wide => fallback * 2.0,
            _ => fallback,
        })
        .sum();
    StatusLineOutputProgress {
        end_x: end_x.min(width).max(0.0),
        end_col: (end_x / fallback).round().max(0.0) as i64,
        y,
        height,
    }
}

impl LayoutEngine {
    pub(crate) fn render_display_source_row(
        &mut self,
        y: f32,
        width: f32,
        height: f32,
        char_w: f32,
        ascent: f32,
        next_face_id: &mut u32,
        base_face: &ResolvedFace,
        rendered: Value,
        face_resolver: &FaceResolver,
        symbol_values: std::collections::HashMap<String, Value>,
        role: GlyphRowRole,
    ) -> Option<RenderedDisplaySourceRow> {
        let base_face_id = if base_face.face_id != 0 {
            base_face.face_id
        } else {
            let id = *next_face_id;
            *next_face_id += 1;
            id
        };
        let status_face =
            self.realize_status_line_face(base_face_id, base_face, char_w, ascent, height);
        let char_width = self.status_line_char_width(&status_face, char_w).max(1.0);
        let mut status_faces = vec![status_face.clone()];
        let mut source =
            LispStringSourceCursor::new(1, rendered, RenderFaceRef::FaceId(status_face.face_id))?;
        let mut items = Vec::new();

        struct RowFaceResolver<'a> {
            engine: &'a mut LayoutEngine,
            face_resolver: &'a FaceResolver,
            base_face: &'a ResolvedFace,
            base_face_id: u32,
            next_face_id: &'a mut u32,
            char_w: f32,
            ascent: f32,
            height: f32,
            status_faces: &'a mut Vec<StatusLineFace>,
        }

        impl DisplayItemFaceResolver for RowFaceResolver<'_> {
            fn resolve_face_ref(
                &mut self,
                base: crate::display_item::RenderFaceRef,
                face_value: Value,
            ) -> crate::display_item::RenderFaceRef {
                let Some(resolved) = self
                    .face_resolver
                    .resolve_face_value_over(self.base_face, &face_value)
                else {
                    return base;
                };
                if same_resolved_face(&resolved, self.base_face) {
                    return crate::display_item::RenderFaceRef::FaceId(self.base_face_id);
                }

                let face_id = *self.next_face_id;
                *self.next_face_id += 1;
                let status_face = self.engine.realize_status_line_face(
                    face_id,
                    &resolved,
                    self.char_w,
                    self.ascent,
                    self.height,
                );
                self.status_faces.push(status_face);
                crate::display_item::RenderFaceRef::FaceId(face_id)
            }
        }

        {
            let mut row_face_resolver = RowFaceResolver {
                engine: self,
                face_resolver,
                base_face,
                base_face_id: status_face.face_id,
                next_face_id,
                char_w,
                ascent,
                height,
                status_faces: &mut status_faces,
            };
            let mut context = DisplaySourceContext::with_face_resolver(&mut row_face_resolver);
            while let Some(item) = source.next_item(&mut context) {
                items.push(item);
            }
        }

        let parsed_symbol_values = symbol_values
            .into_iter()
            .filter_map(|(name, value)| parse_display_length_expr(value).map(|expr| (name, expr)))
            .collect();
        let row_layout = DisplayRowLayout {
            role,
            y_px: y,
            width_px: width,
            height_px: height,
            ascent_px: status_face.font_ascent.max(ascent).min(height.max(1.0)),
            char_width_px: char_width,
            tab_policy: DisplayTabPolicy::every(8),
            base_face: RenderFaceRef::FaceId(status_face.face_id),
            symbol_values: parsed_symbol_values,
        };
        let row = {
            let mut glyph_measurer =
                StatusLineGlyphMeasurer::new(&status_faces, self.font_metrics.as_mut(), char_width);
            let mut row_builder =
                DisplayRowBuilder::with_glyph_measurer(row_layout, &mut glyph_measurer);
            for item in items {
                row_builder.push_item(item);
            }
            row_builder.finish()
        };
        let progress = display_source_row_progress(&row, width, char_width, y, height);
        let faces = status_faces
            .into_iter()
            .map(|face| face.render_face())
            .collect();
        Some(RenderedDisplaySourceRow {
            row,
            progress,
            faces,
        })
    }
}

#[cfg(test)]
#[path = "display_row_test.rs"]
mod tests;
