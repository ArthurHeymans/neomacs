use crate::display_item::{
    DisplayLengthExpr, DisplayMediaReplacement, DisplayMediaReplacementKind, RenderFaceRef,
};
use crate::display_row_builder::{
    DisplayGlyphMeasurer, DisplayRowAppendStatus, DisplayRowLayout, DisplayRowPosition,
    DisplayRowProgressWriter, DisplayTabPolicy,
};
use crate::display_source::{
    DisplayItemSource, DisplaySourceContext, LispStringSourceCursor, parse_display_length_expr,
};
use crate::display_source_resolver::{
    DisplaySourcePropertyResolver, DisplaySourceResolveParams, DisplaySourceResolveState,
};
use crate::engine::LayoutEngine;
use crate::font_metrics::{FontMetrics, FontMetricsService};
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::FaceResolver;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::face::{BoxType, Face, FaceAttributes, UnderlineStyle};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow, GlyphType};
use neomacs_display_protocol::types::Color;
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;

fn underline_style_from_code(code: u8) -> UnderlineStyle {
    UnderlineStyle::from_gnu_code(code).unwrap_or_default()
}

/// Shared render-facing face spec for Lisp-string display rows.
#[derive(Debug, Clone)]
pub(crate) struct DisplayRowFace {
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

impl DisplayRowFace {
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

pub(crate) fn resolved_display_row_face(
    face_id: u32,
    face: &ResolvedFace,
    metrics: Option<FontMetrics>,
) -> DisplayRowFace {
    let mut render_face = DisplayRowFace::from_resolved(face_id, face);
    if let Some(metrics) = metrics {
        render_face.font_char_width = metrics.char_width;
        render_face.font_ascent = metrics.ascent;
        render_face.font_descent = metrics.descent.max(0.0).ceil() as i32;
    }
    render_face
}

pub(crate) fn insert_resolved_display_row_face(
    builder: &mut GlyphMatrixBuilder,
    face_id: u32,
    face: &ResolvedFace,
    metrics: Option<FontMetrics>,
) {
    let render_face = resolved_display_row_face(face_id, face, metrics);
    let rendered = render_face.render_face();
    builder.insert_face(render_face.face_id, rendered);
}

pub(crate) struct DisplayRowFaceRealizer<'a> {
    font_metrics: &'a mut Option<FontMetricsService>,
}

impl<'a> DisplayRowFaceRealizer<'a> {
    pub(crate) fn new(font_metrics: &'a mut Option<FontMetricsService>) -> Self {
        Self { font_metrics }
    }

    pub(crate) fn realize_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        char_w: f32,
        ascent: f32,
        row_height: f32,
    ) -> DisplayRowFace {
        let mut face = DisplayRowFace::from_resolved(face_id, face);
        self.ensure_face_metrics(&mut face, char_w, ascent, row_height);
        face
    }

    pub(crate) fn row_height_for_face(
        &mut self,
        face: &ResolvedFace,
        char_w: f32,
        fallback_ascent: f32,
        fallback_row_height: f32,
    ) -> f32 {
        // GNU TTY frames use 1x1 character cells for chrome rows; GUI font
        // metrics must not make mode/header/tab-line rows taller there.
        if char_w <= 1.0 && fallback_row_height <= 1.0 {
            return fallback_row_height.max(1.0);
        }
        let face = self.realize_face(0, face, char_w, fallback_ascent, fallback_row_height);
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

    pub(crate) fn char_width(&mut self, face: &DisplayRowFace, fallback_char_width: f32) -> f32 {
        if face.font_char_width > 0.0 {
            return face.font_char_width;
        }
        if let Some(svc) = self.font_metrics.as_mut() {
            let metrics = svc.font_metrics(
                &face.font_family,
                face.font_weight,
                face.italic,
                face.font_size,
            );
            return metrics.char_width;
        }
        fallback_char_width
    }

    pub(crate) fn font_metrics_for_face(&mut self, face: &DisplayRowFace) -> FontMetrics {
        if let Some(svc) = self.font_metrics.as_mut() {
            return svc.font_metrics(
                &face.font_family,
                face.font_weight,
                face.italic,
                face.font_size,
            );
        }

        FontMetrics {
            ascent: face.font_ascent.max(1.0),
            descent: face.font_descent.max(0) as f32,
            line_height: (face.font_ascent + face.font_descent as f32).max(1.0),
            char_width: face.font_char_width.max(1.0),
        }
    }

    pub(crate) fn font_metrics_service_mut(&mut self) -> Option<&mut FontMetricsService> {
        self.font_metrics.as_mut()
    }

    fn ensure_face_metrics(
        &mut self,
        face: &mut DisplayRowFace,
        fallback_char_width: f32,
        fallback_ascent: f32,
        row_height: f32,
    ) {
        let needs_metrics = face.font_char_width <= 0.0
            || face.font_ascent <= 0.0
            || (face.font_ascent + face.font_descent as f32) <= 0.0;

        if needs_metrics {
            let metrics = self.font_metrics_for_face(face);

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

struct DisplayRowGlyphMeasurer<'a> {
    faces: &'a [DisplayRowFace],
    font_metrics: Option<&'a mut FontMetricsService>,
    fallback_char_width: f32,
}

impl<'a> DisplayRowGlyphMeasurer<'a> {
    fn new(
        faces: &'a [DisplayRowFace],
        font_metrics: Option<&'a mut FontMetricsService>,
        fallback_char_width: f32,
    ) -> Self {
        Self {
            faces,
            font_metrics,
            fallback_char_width,
        }
    }

    fn face(&self, face_id: u32) -> Option<&DisplayRowFace> {
        self.faces.iter().find(|face| face.face_id == face_id)
    }
}

impl DisplayGlyphMeasurer for DisplayRowGlyphMeasurer<'_> {
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
pub(crate) struct DisplayRowOutputProgress {
    pub end_x: f32,
    pub end_col: i64,
    pub y: f32,
    pub height: f32,
}

pub(crate) struct RenderedDisplaySourceRow {
    pub(crate) row: GlyphRow,
    pub(crate) progress: DisplayRowOutputProgress,
    pub(crate) faces: Vec<Face>,
    pub(crate) media: Vec<RenderedDisplayRowMedia>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderedDisplayRowMedia {
    pub(crate) kind: RenderedDisplayRowMediaKind,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) col: u16,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RenderedDisplayRowMediaKind {
    Image {
        image_id: u32,
    },
    Video {
        video_id: u32,
        loop_count: i32,
        autoplay: bool,
    },
    Xwidget {
        xwidget_id: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowGeometry {
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) tab_policy: DisplayTabPolicy,
}

impl DisplayRowGeometry {
    pub(crate) fn to_layout(
        &self,
        role: GlyphRowRole,
        char_width_px: f32,
        ascent_px: f32,
        base_face: RenderFaceRef,
        symbol_values: std::collections::HashMap<String, DisplayLengthExpr>,
    ) -> DisplayRowLayout {
        DisplayRowLayout {
            role,
            y_px: self.y,
            width_px: self.width.max(1.0),
            height_px: self.height,
            ascent_px,
            char_width_px,
            tab_policy: self.tab_policy.clone(),
            base_face,
            symbol_values,
        }
    }
}

pub(crate) struct DisplayRowSpec<'a> {
    pub(crate) geometry: DisplayRowGeometry,
    pub(crate) base_face_id: u32,
    pub(crate) base_face: &'a ResolvedFace,
    pub(crate) role: GlyphRowRole,
    pub(crate) symbol_values: std::collections::HashMap<String, Value>,
}

impl<'a> DisplayRowSpec<'a> {
    pub(crate) fn from_base_face(
        geometry: DisplayRowGeometry,
        next_face_id: &mut u32,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        let base_face_id = if base_face.face_id != 0 {
            base_face.face_id
        } else {
            let id = *next_face_id;
            *next_face_id += 1;
            id
        };
        Self {
            geometry,
            base_face_id,
            base_face,
            role,
            symbol_values,
        }
    }
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
    for media in &rendered.media {
        media.install(builder, rendered.row.role, matrix_row);
    }
}

impl RenderedDisplayRowMedia {
    fn install(&self, builder: &mut GlyphMatrixBuilder, role: GlyphRowRole, matrix_row: usize) {
        let row = matrix_row.min(u32::MAX as usize) as u32;
        match self.kind {
            RenderedDisplayRowMediaKind::Image { image_id } => builder.push_current_window_image(
                role,
                row,
                self.col,
                image_id,
                self.x,
                self.y,
                self.width,
                self.height,
            ),
            RenderedDisplayRowMediaKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => builder.push_current_window_video(
                role,
                row,
                self.col,
                video_id,
                self.x,
                self.y,
                self.width,
                self.height,
                loop_count,
                autoplay,
            ),
            RenderedDisplayRowMediaKind::Xwidget { xwidget_id } => builder
                .push_current_window_xwidget(
                    role,
                    row,
                    self.col,
                    xwidget_id,
                    self.x,
                    self.y,
                    self.width,
                    self.height,
                ),
        }
    }
}

fn display_source_row_progress(
    row: &GlyphRow,
    width: f32,
    char_width: f32,
    y: f32,
    height: f32,
) -> DisplayRowOutputProgress {
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
    DisplayRowOutputProgress {
        end_x: end_x.min(width).max(0.0),
        end_col: (end_x / fallback).round().max(0.0) as i64,
        y,
        height,
    }
}

impl DisplayMediaReplacement {
    fn rendered_media(self, start: DisplayRowPosition, y: f32) -> RenderedDisplayRowMedia {
        RenderedDisplayRowMedia {
            kind: self.kind.into(),
            x: start.x_px,
            y,
            col: start.col.min(usize::from(u16::MAX)) as u16,
            width: self.width,
            height: self.height,
        }
    }
}

impl From<DisplayMediaReplacementKind> for RenderedDisplayRowMediaKind {
    fn from(kind: DisplayMediaReplacementKind) -> Self {
        match kind {
            DisplayMediaReplacementKind::Image { image_id } => Self::Image { image_id },
            DisplayMediaReplacementKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => Self::Video {
                video_id,
                loop_count,
                autoplay,
            },
            DisplayMediaReplacementKind::Xwidget { xwidget_id } => Self::Xwidget { xwidget_id },
        }
    }
}

pub(crate) struct DisplayRowRenderer<'metrics> {
    font_metrics: &'metrics mut Option<FontMetricsService>,
}

impl<'metrics> DisplayRowRenderer<'metrics> {
    pub(crate) fn new(font_metrics: &'metrics mut Option<FontMetricsService>) -> Self {
        Self { font_metrics }
    }

    #[cfg(test)]
    pub(crate) fn render_display_source_row(
        &mut self,
        spec: DisplayRowSpec<'_>,
        rendered: Value,
        face_resolver: &FaceResolver,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplaySourceRow> {
        self.render_display_source_row_with_display_host(
            spec,
            rendered,
            face_resolver,
            None,
            next_face_id,
        )
    }

    pub(crate) fn render_display_source_row_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        rendered: Value,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplaySourceRow> {
        let base_face_id = spec.base_face_id;
        let mut source =
            LispStringSourceCursor::new(1, rendered, RenderFaceRef::FaceId(base_face_id))?;
        self.render_display_item_source_row_with_display_host(
            spec,
            &mut source,
            face_resolver,
            display_host,
            next_face_id,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_display_item_source_row(
        &mut self,
        spec: DisplayRowSpec<'_>,
        source: &mut impl DisplayItemSource,
        face_resolver: &FaceResolver,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplaySourceRow> {
        self.render_display_item_source_row_with_display_host(
            spec,
            source,
            face_resolver,
            None,
            next_face_id,
        )
    }

    pub(crate) fn render_display_item_source_row_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        source: &mut impl DisplayItemSource,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplaySourceRow> {
        let DisplayRowSpec {
            geometry,
            base_face_id,
            base_face,
            role,
            symbol_values,
        } = spec;
        *next_face_id = (*next_face_id).max(base_face_id.saturating_add(1));
        let mut face_realizer = DisplayRowFaceRealizer::new(&mut *self.font_metrics);
        let row_face = face_realizer.realize_face(
            base_face_id,
            base_face,
            geometry.char_width,
            geometry.ascent,
            geometry.height,
        );
        let char_width = face_realizer
            .char_width(&row_face, geometry.char_width)
            .max(1.0);
        let mut row_faces = vec![row_face.clone()];
        let mut resolve_state = DisplaySourceResolveState::default();

        let parsed_symbol_values = symbol_values
            .into_iter()
            .filter_map(|(name, value)| parse_display_length_expr(value).map(|expr| (name, expr)))
            .collect();
        let row_ascent = row_face
            .font_ascent
            .max(geometry.ascent)
            .min(geometry.height.max(1.0));
        let row_layout = geometry.to_layout(
            role,
            char_width,
            row_ascent,
            RenderFaceRef::FaceId(row_face.face_id),
            parsed_symbol_values,
        );
        let mut row = GlyphRow::new(role);
        let mut position = DisplayRowPosition { x_px: 0.0, col: 0 };
        let mut media = Vec::new();
        loop {
            let mut pending_faces = Vec::new();
            let item = {
                let params = DisplaySourceResolveParams {
                    face_resolver,
                    display_host,
                    base_face,
                    base_face_id: row_face.face_id,
                    fallback_char_width: char_width,
                    fallback_row_height: geometry.height,
                };
                let mut row_face_resolver = DisplaySourcePropertyResolver::new(
                    params,
                    &mut resolve_state,
                    next_face_id,
                    &mut pending_faces,
                );
                let mut context = DisplaySourceContext::with_face_resolver(&mut row_face_resolver);
                source.next_item(&mut context)
            };
            for pending in pending_faces {
                let row_face = face_realizer.realize_face(
                    pending.face_id,
                    &pending.resolved,
                    char_width,
                    geometry.ascent,
                    geometry.height,
                );
                row_faces.push(row_face);
            }
            let Some(item) = item else {
                break;
            };
            let media_descriptor = DisplayMediaReplacement::from_item_kind(&item.kind);
            let item = media_descriptor
                .map(|descriptor| descriptor.replacement_item(item.clone()))
                .unwrap_or(item);
            let mut glyph_measurer = DisplayRowGlyphMeasurer::new(
                &row_faces,
                face_realizer.font_metrics_service_mut(),
                char_width,
            );
            let mut row_writer = DisplayRowProgressWriter::with_glyph_measurer(
                &row_layout,
                &mut row,
                &mut glyph_measurer,
                position,
                f32::INFINITY,
            );
            let progress = row_writer.push_item(item);
            position = row_writer.position();
            if let Some(descriptor) = media_descriptor
                && progress.status == DisplayRowAppendStatus::Complete
                && progress.metrics.width_px > 0.0
            {
                media.push(descriptor.rendered_media(progress.start, row_layout.y_px));
            }
            if progress.status != DisplayRowAppendStatus::Complete {
                break;
            }
        }
        GlyphMatrixBuilder::normalize_external_row(&mut row);
        let progress = display_source_row_progress(
            &row,
            geometry.width,
            char_width,
            geometry.y,
            geometry.height,
        );
        let faces = row_faces
            .into_iter()
            .map(|face| face.render_face())
            .collect();
        Some(RenderedDisplaySourceRow {
            row,
            progress,
            faces,
            media,
        })
    }
}

impl LayoutEngine {
    #[cfg(test)]
    pub(crate) fn render_display_source_row(
        &mut self,
        spec: DisplayRowSpec<'_>,
        rendered: Value,
        face_resolver: &FaceResolver,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplaySourceRow> {
        self.render_display_source_row_with_display_host(
            spec,
            rendered,
            face_resolver,
            None,
            next_face_id,
        )
    }

    pub(crate) fn render_display_source_row_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        rendered: Value,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplaySourceRow> {
        DisplayRowRenderer::new(&mut self.font_metrics).render_display_source_row_with_display_host(
            spec,
            rendered,
            face_resolver,
            display_host,
            next_face_id,
        )
    }
}

#[cfg(test)]
#[path = "display_row_test.rs"]
mod tests;
