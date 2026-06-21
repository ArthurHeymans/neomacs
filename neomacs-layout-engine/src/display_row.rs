use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_ref::render_face_ref_id;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLengthExpr, DisplayMediaReplacement,
    DisplaySourceMappedText, DisplaySourcePosition, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_origin::DisplayOrigin;
use crate::display_property::parse_display_length_expr;
use crate::display_row_builder::{
    DisplayGlyphMeasurer, DisplayRowAppendProgress, DisplayRowAppendStatus,
    DisplayRowItemMeasurement, DisplayRowLayout, DisplayRowPosition, DisplayRowProgressWriter,
    DisplayTabPolicy, new_display_row_for_role,
};
use crate::display_row_geometry::DisplayRowGeometryState;
pub(crate) use crate::display_row_geometry::DisplayRowMaxX;
pub(crate) use crate::display_row_metrics::{
    DisplayRowFallbackMetrics, DisplayRowMeasuredFaceMetrics,
};
#[cfg(test)]
pub(crate) use crate::display_row_render_state::{
    CurrentTextRowRenderOutcome, RenderedDisplayRowMediaKind,
};
pub(crate) use crate::display_row_render_state::{
    DisplayRowOutputProgress, DisplayRowRenderBounds, DisplayRowRenderIntoRowResult,
    DisplayRowRenderResult, DisplayRowRenderStop, RenderedDisplayRow, RenderedDisplayRowMedia,
    display_row_progress,
};
use crate::display_row_width::DisplayRowCharWidthPolicy;
use crate::display_source::{DisplayItemSource, LispStringSourceCursor};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_source_resolver::{
    DisplaySourceFaceBasis, DisplaySourceResolveParams, DisplaySourceResolveState,
    ResolvedDisplaySourceItem, resolve_next_display_source_item,
};
use crate::display_text_run_measurement::{
    ComplexTextRunAdvancePolicy, ComplexTextRunAdvanceResolver, DisplayTextRunAdvance,
    DisplayTextRunMeasurement, DisplayTextRunMeasurementPlan,
};
use crate::font_metrics::{FontMetrics, FontMetricsService};
use crate::glyph_advance::GlyphAdvanceQuantization;
use crate::neovm_bridge::FaceResolver;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::face::{BoxType, Face, FaceAttributes, UnderlineStyle};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{CharPos0, EmacsBytePos};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;

fn underline_style_from_code(code: u8) -> UnderlineStyle {
    UnderlineStyle::from_gnu_code(code).unwrap_or_default()
}

/// Shared render-facing face state for display rows.
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
    pub(crate) metrics: DisplayRowFaceMetrics,
    pub(crate) underline_position: i32,
    pub(crate) underline_thickness: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DisplayRowFaceMetrics {
    char_width_px: f32,
    ascent_px: f32,
    descent_px: i32,
}

impl DisplayRowFaceMetrics {
    pub(crate) fn new(char_width_px: f32, ascent_px: f32, descent_px: i32) -> Self {
        Self {
            char_width_px,
            ascent_px,
            descent_px,
        }
    }

    pub(crate) fn from_resolved(face: &ResolvedFace) -> Self {
        let descent_px = if face.font_line_height > 0.0 && face.font_ascent > 0.0 {
            (face.font_line_height - face.font_ascent).max(0.0).ceil() as i32
        } else {
            0
        };
        Self::new(face.measured_char_width_px(), face.font_ascent, descent_px)
    }

    pub(crate) fn from_font_metrics(metrics: FontMetrics) -> Self {
        Self::new(
            metrics.char_width,
            metrics.ascent,
            metrics.descent.max(0.0).ceil() as i32,
        )
    }

    pub(crate) fn ascent_px(self) -> f32 {
        self.ascent_px
    }

    pub(crate) fn descent_px(self) -> i32 {
        self.descent_px
    }

    pub(crate) fn set_char_width_px(&mut self, char_width_px: f32) {
        self.char_width_px = char_width_px;
    }

    pub(crate) fn set_ascent_px(&mut self, ascent_px: f32) {
        self.ascent_px = ascent_px;
    }

    pub(crate) fn set_descent_px(&mut self, descent_px: i32) {
        self.descent_px = descent_px;
    }

    pub(crate) fn line_height_px(self) -> f32 {
        (self.ascent_px + self.descent_px as f32).max(1.0)
    }

    pub(crate) fn has_char_width(self, fallback_char_width: f32) -> bool {
        DisplayRowCharWidthPolicy::new(fallback_char_width).has_width(self.char_width_px)
    }

    pub(crate) fn char_width_px(self, fallback_char_width: f32) -> f32 {
        DisplayRowCharWidthPolicy::new(fallback_char_width).width(self.char_width_px)
    }

    pub(crate) fn char_width_or_measured_px(
        self,
        fallback_char_width: f32,
        measured_width: Option<f32>,
    ) -> f32 {
        DisplayRowCharWidthPolicy::new(fallback_char_width)
            .width_or_measured(self.char_width_px, measured_width)
    }

    pub(crate) fn normalize_char_width(&mut self, fallback_char_width: f32) {
        self.set_char_width_px(self.char_width_px(fallback_char_width));
    }

    pub(crate) fn include_in_layout(self, layout: &mut DisplayRowLayout) {
        let glyph_ascent = self.ascent_px().max(0.0);
        let glyph_height = self.line_height_px();
        let glyph_descent = (glyph_height - glyph_ascent).max(0.0);
        let row_descent = (layout.height_px - layout.ascent_px).max(0.0);
        layout.ascent_px = layout
            .ascent_px
            .max(glyph_ascent)
            .min(glyph_height.max(layout.height_px));
        layout.height_px = (layout.ascent_px + row_descent.max(glyph_descent)).max(glyph_height);
    }
}

impl DisplayRowFace {
    pub(crate) fn from_resolved(face_id: u32, face: &ResolvedFace) -> Self {
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
            metrics: DisplayRowFaceMetrics::from_resolved(face),
            underline_position: 1,
            underline_thickness: 1,
        }
    }

    pub(crate) fn has_char_width(&self, fallback_char_width: f32) -> bool {
        self.metrics.has_char_width(fallback_char_width)
    }

    pub(crate) fn char_width_px(&self, fallback_char_width: f32) -> f32 {
        self.metrics.char_width_px(fallback_char_width)
    }

    pub(crate) fn char_width_or_measured_px(
        &self,
        fallback_char_width: f32,
        measured_width: Option<f32>,
    ) -> f32 {
        self.metrics
            .char_width_or_measured_px(fallback_char_width, measured_width)
    }

    pub(crate) fn normalize_char_width(&mut self, fallback_char_width: f32) {
        self.metrics.normalize_char_width(fallback_char_width);
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
            font_ascent: self.metrics.ascent_px() as i32,
            font_descent: self.metrics.descent_px(),
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
        render_face.metrics = DisplayRowFaceMetrics::from_font_metrics(metrics);
    }
    render_face
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
        let line_height = face.metrics.line_height_px().ceil();
        let box_pixels = if face.box_type != BoxType::None && face.box_h_line_width != 0 {
            2.0 * face.box_h_line_width.unsigned_abs() as f32
        } else {
            0.0
        };
        let minimum_row_height = fallback_row_height.ceil().max(1.0);
        (line_height + box_pixels).max(minimum_row_height)
    }

    pub(crate) fn char_width(&mut self, face: &DisplayRowFace, fallback_char_width: f32) -> f32 {
        let metrics = self.measured_font_metrics_for_face(face);
        face.char_width_or_measured_px(
            fallback_char_width,
            metrics.map(|metrics| metrics.char_width),
        )
    }

    fn measured_font_metrics_for_face(&mut self, face: &DisplayRowFace) -> Option<FontMetrics> {
        self.font_metrics.as_mut().map(|svc| {
            svc.font_metrics(
                &face.font_family,
                face.font_weight,
                face.italic,
                face.font_size,
            )
        })
    }

    pub(crate) fn font_metrics_service_mut(&mut self) -> Option<&mut FontMetricsService> {
        self.font_metrics.as_mut()
    }

    pub(crate) fn font_metrics_mut(&mut self) -> &mut Option<FontMetricsService> {
        self.font_metrics
    }

    fn ensure_face_metrics(
        &mut self,
        face: &mut DisplayRowFace,
        fallback_char_width: f32,
        fallback_ascent: f32,
        row_height: f32,
    ) {
        let needs_metrics = !face.has_char_width(fallback_char_width)
            || face.metrics.ascent_px() <= 0.0
            || face.metrics.line_height_px() <= 0.0;

        if needs_metrics && let Some(metrics) = self.measured_font_metrics_for_face(face) {
            if !face.has_char_width(fallback_char_width) {
                face.metrics.set_char_width_px(
                    DisplayRowCharWidthPolicy::new(fallback_char_width).width(metrics.char_width),
                );
            }
            if face.metrics.ascent_px() <= 0.0 && metrics.ascent > 0.0 {
                face.metrics.set_ascent_px(metrics.ascent);
            }
            if face.metrics.line_height_px() <= 0.0 && metrics.line_height > 0.0 {
                face.metrics
                    .set_descent_px((metrics.line_height - metrics.ascent).max(0.0).ceil() as i32);
            }
        }

        face.normalize_char_width(fallback_char_width);
        if face.metrics.ascent_px() <= 0.0 {
            face.metrics.set_ascent_px(fallback_ascent.max(1.0));
        }
        if face.metrics.line_height_px() <= 0.0
            || (face.metrics.descent_px() <= 0 && row_height > face.metrics.ascent_px())
        {
            face.metrics
                .set_descent_px((row_height - face.metrics.ascent_px()).max(0.0).ceil() as i32);
        }
    }
}

pub(crate) struct DisplayRowGlyphMeasurer<'a> {
    faces: &'a [DisplayRowFace],
    font_metrics: Option<&'a mut FontMetricsService>,
    fallback_char_width: f32,
    quantization: GlyphAdvanceQuantization,
}

impl<'a> DisplayRowGlyphMeasurer<'a> {
    pub(crate) fn new(
        faces: &'a [DisplayRowFace],
        font_metrics: Option<&'a mut FontMetricsService>,
        fallback_char_width: f32,
    ) -> Self {
        Self::with_quantization(
            faces,
            font_metrics,
            fallback_char_width,
            GlyphAdvanceQuantization::PreserveLogicalPixels,
        )
    }

    pub(crate) fn with_quantization(
        faces: &'a [DisplayRowFace],
        font_metrics: Option<&'a mut FontMetricsService>,
        fallback_char_width: f32,
        quantization: GlyphAdvanceQuantization,
    ) -> Self {
        Self {
            faces,
            font_metrics,
            fallback_char_width,
            quantization,
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
        let face_char_width = face.char_width_px(self.fallback_char_width);
        let min_advance = f32::from(columns) * face_char_width;
        let font_family = face.font_family.clone();
        let font_weight = face.font_weight;
        let italic = face.italic;
        let font_size = face.font_size.max(1.0);
        let measured = self
            .font_metrics
            .as_mut()
            .map(|svc| svc.char_width(ch, &font_family, font_weight, italic, font_size));

        Some(
            self.quantization
                .resolve(measured, fallback_advance_px.max(min_advance), min_advance),
        )
    }

    fn text_run_advances_px(
        &mut self,
        text: &str,
        face_id: u32,
        fallback_char_width_px: f32,
    ) -> DisplayTextRunMeasurement {
        if text.is_empty() {
            return DisplayTextRunMeasurement::PerChar;
        }

        let Some(face) = self.face(face_id).cloned() else {
            return DisplayTextRunMeasurement::PerChar;
        };
        let Some(font_metrics) = self.font_metrics.as_mut() else {
            return DisplayTextRunMeasurement::PerChar;
        };

        let shaped = font_metrics.shape_run(
            text,
            &face.font_family,
            face.font_weight,
            face.italic,
            face.font_size.max(1.0),
        );
        if shaped.is_empty() {
            return DisplayTextRunMeasurement::PerChar;
        }

        let face_char_width = face
            .char_width_px(self.fallback_char_width)
            .max(DisplayRowCharWidthPolicy::new(fallback_char_width_px).fallback());
        DisplayTextRunMeasurementPlan::from_shaped_glyphs(
            text,
            shaped,
            face_char_width,
            DisplayRowCharWidthPolicy::new(fallback_char_width_px).fallback(),
            self.quantization,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowMeasurementMode {
    FontMetrics,
    FallbackMetrics,
}

impl DisplayRowMeasurementMode {
    pub(crate) fn from_frame_window_system(window_system: bool) -> Self {
        if window_system {
            Self::FontMetrics
        } else {
            Self::FallbackMetrics
        }
    }

    fn uses_font_metrics(self) -> bool {
        matches!(self, Self::FontMetrics)
    }

    fn quantization(self) -> GlyphAdvanceQuantization {
        match self {
            Self::FontMetrics => GlyphAdvanceQuantization::PreserveLogicalPixels,
            Self::FallbackMetrics => GlyphAdvanceQuantization::SnapToIntegerPixels,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DisplayRowMeasurementPolicy {
    mode: DisplayRowMeasurementMode,
}

impl DisplayRowMeasurementPolicy {
    pub(crate) fn for_frame(window_system: bool) -> Self {
        Self {
            mode: DisplayRowMeasurementMode::from_frame_window_system(window_system),
        }
    }

    pub(crate) fn measurement_face(
        self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<FontMetrics>,
        fallback_char_width: f32,
    ) -> DisplayRowGlyphMeasurementFace {
        DisplayRowGlyphMeasurementFace::with_mode(
            resolved_display_row_face(face_id, face, metrics),
            self.mode,
            fallback_char_width,
            self.mode.quantization(),
        )
    }

    pub(crate) fn measured_face(
        self,
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<FontMetrics>,
        fallback_char_width: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowMeasuredFace {
        let measurement_width_policy = DisplayRowCharWidthPolicy::new(fallback_char_width);
        let fallback_width_policy = DisplayRowCharWidthPolicy::new(fallback_metrics.char_width);
        let measurement_face =
            self.measurement_face(face_id, face, metrics, measurement_width_policy.fallback());
        let space_width =
            measurement_face.advance_for_char(font_metrics, ' ', fallback_width_policy.fallback());
        let (char_width, row_height, ascent) = metrics
            .map(|metrics| {
                (
                    fallback_width_policy.width(metrics.char_width),
                    metrics.line_height,
                    metrics.ascent,
                )
            })
            .unwrap_or((
                fallback_width_policy.fallback(),
                fallback_metrics.row_height,
                fallback_metrics.ascent,
            ));
        DisplayRowMeasuredFace {
            measurement_face,
            metrics: DisplayRowMeasuredFaceMetrics::new(
                char_width,
                row_height,
                ascent,
                space_width,
            ),
        }
    }

    pub(crate) fn resolved_measured_face(
        self,
        face_id: u32,
        face: ResolvedFace,
        metrics: Option<FontMetrics>,
        fallback_char_width: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowResolvedMeasuredFace {
        let measured_face = self.measured_face(
            face_id,
            &face,
            metrics,
            fallback_char_width,
            fallback_metrics,
            font_metrics,
        );
        DisplayRowResolvedMeasuredFace {
            face,
            metrics,
            measured_face,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowGlyphMeasurementFace {
    face: DisplayRowFace,
    mode: DisplayRowMeasurementMode,
    fallback_char_width: f32,
    quantization: GlyphAdvanceQuantization,
}

impl DisplayRowGlyphMeasurementFace {
    pub(crate) fn face_id(&self) -> u32 {
        self.face.face_id
    }

    pub(crate) fn with_mode(
        face: DisplayRowFace,
        mode: DisplayRowMeasurementMode,
        fallback_char_width: f32,
        quantization: GlyphAdvanceQuantization,
    ) -> Self {
        let width_policy = DisplayRowCharWidthPolicy::new(fallback_char_width);
        Self {
            face,
            mode,
            fallback_char_width: width_policy.fallback(),
            quantization,
        }
    }

    pub(crate) fn glyph_advance_px(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        columns: u8,
        fallback_advance_px: f32,
    ) -> f32 {
        let faces = [self.face.clone()];
        let font_metrics = if self.mode.uses_font_metrics() {
            font_metrics.as_mut()
        } else {
            None
        };
        let mut measurer = DisplayRowGlyphMeasurer::with_quantization(
            &faces,
            font_metrics,
            self.fallback_char_width,
            self.quantization,
        );
        measurer
            .glyph_advance_px(ch, self.face.face_id, columns, fallback_advance_px)
            .unwrap_or(fallback_advance_px)
    }

    pub(crate) fn advance_for_char(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        fallback_advance_px: f32,
    ) -> f32 {
        let columns = crate::composition::base_width_cols(ch);
        if columns == 0 {
            return 0.0;
        }
        self.glyph_advance_px(font_metrics, ch, columns, fallback_advance_px)
    }

    fn shaped_text_run_measurement(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        text: &str,
    ) -> DisplayTextRunMeasurement {
        if !self.mode.uses_font_metrics() {
            return DisplayTextRunMeasurement::PerChar;
        }
        let faces = [self.face.clone()];
        let mut measurer = DisplayRowGlyphMeasurer::with_quantization(
            &faces,
            font_metrics.as_mut(),
            self.fallback_char_width,
            self.quantization,
        );
        measurer.text_run_advances_px(text, self.face.face_id, self.fallback_char_width)
    }

    fn fallback_text_run_measurement(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        text: &str,
    ) -> DisplayTextRunMeasurement {
        let advances = text
            .char_indices()
            .enumerate()
            .map(|(char_offset, (byte_offset, ch))| {
                let columns = crate::composition::base_width_cols(ch).max(1);
                let fallback_advance_px = DisplayRowCharWidthPolicy::new(self.fallback_char_width)
                    .advance_for_columns(columns);
                DisplayTextRunAdvance::new(
                    char_offset,
                    byte_offset,
                    self.advance_for_char(font_metrics, ch, fallback_advance_px),
                )
            })
            .collect();
        DisplayTextRunMeasurement::Measured(advances)
    }

    pub(crate) fn text_run_measurement(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        text: &str,
    ) -> DisplayTextRunMeasurement {
        let measurement = self.shaped_text_run_measurement(font_metrics, text);
        if measurement.measured_advances().is_some() {
            return measurement;
        }
        self.fallback_text_run_measurement(font_metrics, text)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowMeasuredFace {
    measurement_face: DisplayRowGlyphMeasurementFace,
    metrics: DisplayRowMeasuredFaceMetrics,
}

impl DisplayRowMeasuredFace {
    pub(crate) fn face_id(&self) -> u32 {
        self.measurement_face.face_id()
    }

    pub(crate) fn into_measurement_face(self) -> DisplayRowGlyphMeasurementFace {
        self.measurement_face
    }

    pub(crate) fn metrics(&self) -> DisplayRowMeasuredFaceMetrics {
        self.metrics
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowResolvedMeasuredFace {
    face: ResolvedFace,
    metrics: Option<FontMetrics>,
    measured_face: DisplayRowMeasuredFace,
}

impl DisplayRowResolvedMeasuredFace {
    pub(crate) fn face_id(&self) -> u32 {
        self.measured_face.face_id()
    }

    pub(crate) fn into_active_face_state(self) -> DisplayRowActiveFaceState {
        DisplayRowActiveFaceState::new(self.face, self.measured_face)
    }

    pub(crate) fn resolved_face(&self) -> &ResolvedFace {
        &self.face
    }

    pub(crate) fn font_metrics(&self) -> Option<FontMetrics> {
        self.metrics
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowActiveFaceRenderState {
    pub(crate) face_id: u32,
    pub(crate) background: Color,
    resolved_face: ResolvedFace,
}

impl DisplayRowActiveFaceRenderState {
    pub(crate) fn resolved_face(&self) -> &ResolvedFace {
        &self.resolved_face
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowActiveFaceMeasurementState {
    measurement_face: DisplayRowGlyphMeasurementFace,
    metrics: DisplayRowMeasuredFaceMetrics,
}

impl DisplayRowActiveFaceMeasurementState {
    pub(crate) fn metrics(&self) -> DisplayRowMeasuredFaceMetrics {
        self.metrics
    }

    pub(crate) fn advance_for_char(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        fallback_advance_px: f32,
    ) -> f32 {
        self.measurement_face
            .advance_for_char(font_metrics, ch, fallback_advance_px)
    }

    pub(crate) fn text_run_measurement(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        text: &str,
    ) -> DisplayTextRunMeasurement {
        self.measurement_face
            .text_run_measurement(font_metrics, text)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowActiveFaceState {
    render: DisplayRowActiveFaceRenderState,
    measurement: DisplayRowActiveFaceMeasurementState,
}

struct DisplayRowComplexTextRunAdvancePolicy<'a> {
    active_face_state: &'a DisplayRowActiveFaceState,
    font_metrics: &'a mut Option<FontMetricsService>,
}

impl<'a> DisplayRowComplexTextRunAdvancePolicy<'a> {
    fn new(
        active_face_state: &'a DisplayRowActiveFaceState,
        font_metrics: &'a mut Option<FontMetricsService>,
    ) -> Self {
        Self {
            active_face_state,
            font_metrics,
        }
    }
}

impl ComplexTextRunAdvancePolicy for DisplayRowComplexTextRunAdvancePolicy<'_> {
    fn text_run_measurement(&mut self, text: &str) -> DisplayTextRunMeasurement {
        self.active_face_state
            .text_run_measurement(self.font_metrics, text)
    }

    fn advance_for_columns(&mut self, ch: char, columns: usize) -> f32 {
        self.active_face_state
            .advance_for_columns(self.font_metrics, ch, columns)
    }
}

impl DisplayRowActiveFaceState {
    pub(crate) fn new(resolved_face: ResolvedFace, measured_face: DisplayRowMeasuredFace) -> Self {
        let face_id = measured_face.face_id();
        let background = Color::from_pixel(resolved_face.bg);
        let metrics = measured_face.metrics();
        Self {
            render: DisplayRowActiveFaceRenderState {
                face_id,
                background,
                resolved_face,
            },
            measurement: DisplayRowActiveFaceMeasurementState {
                measurement_face: measured_face.into_measurement_face(),
                metrics,
            },
        }
    }

    pub(crate) fn face_id(&self) -> u32 {
        self.render.face_id
    }

    pub(crate) fn background(&self) -> Color {
        self.render.background
    }

    pub(crate) fn resolved_face(&self) -> &ResolvedFace {
        self.render.resolved_face()
    }

    pub(crate) fn metrics(&self) -> DisplayRowMeasuredFaceMetrics {
        self.measurement.metrics()
    }

    pub(crate) fn advance_for_char(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        fallback_advance_px: f32,
    ) -> f32 {
        self.measurement
            .advance_for_char(font_metrics, ch, fallback_advance_px)
    }

    pub(crate) fn advance_for_columns(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        columns: usize,
    ) -> f32 {
        if columns == 0 {
            return 0.0;
        }
        let fallback_advance_px = self.metrics().char_width() * columns as f32;
        self.advance_for_char(font_metrics, ch, fallback_advance_px)
    }

    pub(crate) fn display_replacement_string_cursor_slot_width(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        replacement: &str,
    ) -> f32 {
        let face_metrics = self.metrics();
        replacement
            .chars()
            .next()
            .map(|ch| self.advance_for_char(font_metrics, ch, face_metrics.char_width()))
            .unwrap_or_else(|| face_metrics.char_width().max(1.0))
    }

    pub(crate) fn display_replacement_stretch_source_char_width(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        source_char: char,
    ) -> f32 {
        self.advance_for_char(font_metrics, source_char, self.metrics().char_width())
    }

    pub(crate) fn complex_text_run_advance(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        resolver: &mut ComplexTextRunAdvanceResolver,
        text: &[u8],
        byte_idx: usize,
        ch: char,
        is_cluster_continuation: bool,
    ) -> f32 {
        let mut policy = DisplayRowComplexTextRunAdvancePolicy::new(self, font_metrics);
        resolver.advance_for_char(text, byte_idx, ch, is_cluster_continuation, &mut policy)
    }

    pub(crate) fn text_run_measurement(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        text: &str,
    ) -> DisplayTextRunMeasurement {
        self.measurement.text_run_measurement(font_metrics, text)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WindowChromeKind {
    TabLine,
    HeaderLine,
    ModeLine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameChromeKind {
    TabBar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowOwner {
    WindowChrome {
        window_id: u64,
        kind: WindowChromeKind,
    },
    FrameChrome {
        kind: FrameChromeKind,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowBoundsPolicy {
    PreserveAllocatedMinimum,
    MeasureContent,
}

pub(crate) struct MeasuredDisplayRow {
    owner: DisplayRowOwner,
    row_index: u32,
    bounds: Rect,
    rendered: RenderedDisplayRow,
}

fn stable_pixel_ceil(px: f32) -> f32 {
    if !px.is_finite() {
        return 1.0;
    }
    let px = px.max(1.0);
    let rounded = px.round();
    if (px - rounded).abs() <= 0.01 {
        rounded.max(1.0)
    } else {
        px.ceil().max(1.0)
    }
}

fn rendered_display_row_content_height(rendered: &RenderedDisplayRow) -> f32 {
    let mut height = 1.0_f32;
    for glyph in rendered.row().glyphs.iter().flatten() {
        if let Some(face) = rendered
            .faces()
            .iter()
            .find(|face| face.id == glyph.face_id)
        {
            let face_height = (face.font_ascent + face.font_descent).max(1) as f32;
            height = height.max(face_height + glyph.vertical_offset_px.abs());
        }
    }
    for media in rendered.media() {
        height = height.max(media.height);
    }
    height
}

impl MeasuredDisplayRow {
    pub(crate) fn new(
        owner: DisplayRowOwner,
        row_index: u32,
        fallback_bounds: Rect,
        rendered: RenderedDisplayRow,
        bounds_policy: DisplayRowBoundsPolicy,
    ) -> Self {
        let content_height = stable_pixel_ceil(rendered_display_row_content_height(&rendered));
        let allocated_height = stable_pixel_ceil(
            fallback_bounds
                .height
                .max(rendered.row().height_px)
                .max(rendered.progress().height())
                .max(content_height),
        );
        let height = match bounds_policy {
            DisplayRowBoundsPolicy::PreserveAllocatedMinimum => allocated_height,
            DisplayRowBoundsPolicy::MeasureContent => content_height,
        };
        Self {
            owner,
            row_index,
            bounds: Rect::new(
                fallback_bounds.x,
                fallback_bounds.y,
                fallback_bounds.width,
                height,
            ),
            rendered,
        }
    }

    pub(crate) fn owner(&self) -> DisplayRowOwner {
        self.owner
    }

    pub(crate) fn row_index(&self) -> u32 {
        self.row_index
    }

    pub(crate) fn bounds(&self) -> Rect {
        self.bounds
    }

    pub(crate) fn rendered(&self) -> &RenderedDisplayRow {
        &self.rendered
    }

    pub(crate) fn row_height(&self) -> f32 {
        self.bounds.height.max(1.0)
    }

    pub(crate) fn row_ascent(&self) -> f32 {
        self.rendered
            .row()
            .ascent_px
            .max(0.0)
            .min(self.row_height())
    }

    pub(crate) fn output_progress(&self) -> DisplayRowOutputProgress {
        self.rendered
            .progress()
            .with_y(self.bounds.y)
            .with_height(self.bounds.height)
    }

    pub(crate) fn absolute_output_row(&self) -> GlyphRow {
        self.rendered
            .materialize_output_row(self.bounds.y, self.row_height(), self.row_ascent())
    }

    pub(crate) fn window_relative_output_row(&self, window_bounds: Rect) -> GlyphRow {
        self.rendered.materialize_output_row(
            self.bounds.y - window_bounds.y,
            self.row_height(),
            self.row_ascent(),
        )
    }
}

#[derive(Default)]
pub(crate) struct DisplayRowSourceState {
    resolve_state: DisplaySourceResolveState,
    pending_item: Option<DisplayItem>,
    exhausted: bool,
}

impl DisplayRowSourceState {
    pub(crate) fn next_resolved_item(
        &mut self,
        source: &mut impl DisplayItemSource,
        params: DisplaySourceResolveParams<'_>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> ResolvedDisplaySourceItem {
        if self.is_finished() {
            return ResolvedDisplaySourceItem::empty();
        }
        if let Some(item) = self.take_pending_item() {
            return ResolvedDisplaySourceItem::new(Some(item), Vec::new());
        }
        let resolved =
            resolve_next_display_source_item(source, params, &mut self.resolve_state, face_ids);
        if resolved.item().is_none() {
            self.mark_exhausted();
        }
        resolved
    }

    pub(crate) fn resolved_face(&self, face_id: u32) -> Option<&ResolvedFace> {
        self.resolve_state.resolved_face(face_id)
    }

    fn take_pending_item(&mut self) -> Option<DisplayItem> {
        self.pending_item.take()
    }

    pub(crate) fn remember_pending_item(&mut self, item: Option<DisplayItem>) {
        self.pending_item = item;
    }

    pub(crate) fn discard_pending_item(&mut self) {
        self.pending_item = None;
    }

    fn mark_exhausted(&mut self) {
        self.exhausted = true;
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.exhausted && self.pending_item.is_none()
    }
}

#[cfg(test)]
pub(crate) struct DisplayRowSourceStep {
    item: DisplayItem,
    pending_faces: Vec<PendingDisplaySourceFace>,
}

#[cfg(test)]
impl DisplayRowSourceStep {
    pub(crate) fn into_parts(self) -> (DisplayItem, Vec<PendingDisplaySourceFace>) {
        (self.item, self.pending_faces)
    }
}

#[cfg(test)]
pub(crate) struct DisplayRowSourceWalker<S> {
    source: S,
    state: DisplayRowSourceState,
}

#[cfg(test)]
impl<S> DisplayRowSourceWalker<S> {
    pub(crate) fn new(source: S) -> Self {
        Self {
            source,
            state: DisplayRowSourceState::default(),
        }
    }
}

#[cfg(test)]
impl<S: DisplayItemSource> DisplayRowSourceWalker<S> {
    pub(crate) fn next_step(
        &mut self,
        face_resolver: &FaceResolver,
        base_face: &ResolvedFace,
        base_face_id: u32,
        face_ids: &mut FrameFaceIdAllocator,
        display_host: Option<&dyn DisplayHost>,
        fallback_char_width: f32,
        fallback_ascent: f32,
        fallback_row_height: f32,
    ) -> Option<DisplayRowSourceStep> {
        let face_basis = DisplaySourceFaceBasis::new(
            face_resolver,
            base_face_id,
            base_face,
            DisplayRowFallbackMetrics::from_default_face_extents(
                fallback_char_width,
                fallback_row_height,
                fallback_ascent,
            ),
        );
        let resolved = self.state.next_resolved_item(
            &mut self.source,
            DisplaySourceResolveParams::new(face_basis, display_host),
            face_ids,
        );
        let (item, pending_faces) = resolved.into_parts();
        item.map(|item| DisplayRowSourceStep {
            item,
            pending_faces,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowRenderClipBehavior {
    PreserveRemainderAndStop,
    Stop,
    Continue,
}

pub(crate) trait DisplayRowRenderPolicy {
    fn stop_before_item(&mut self, _item: &DisplayItem) -> bool {
        false
    }

    fn measurement_for(
        &mut self,
        _item: &DisplayItem,
        _face_id: u32,
        _font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        DisplayRowItemMeasurement::Default
    }

    fn clipped_behavior(&mut self, _item: &DisplayItem) -> DisplayRowRenderClipBehavior {
        DisplayRowRenderClipBehavior::PreserveRemainderAndStop
    }
}

struct NaturalDisplayRowRenderPolicy;

impl DisplayRowRenderPolicy for NaturalDisplayRowRenderPolicy {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DisplayRowLispStringSourceId(u64);

impl DisplayRowLispStringSourceId {
    const ROOT: Self = Self(1);

    fn raw(self) -> u64 {
        self.0
    }
}

pub(crate) struct DisplayRowItemSourceRenderRequest<'a> {
    row_request: DisplayRowSourceRenderRequest<'a>,
}

pub(crate) struct DisplayRowSourceFragmentRenderRequest<'a> {
    item_request: DisplayRowItemSourceRenderRequest<'a>,
}

#[derive(Clone)]
pub(crate) struct DisplayRowSourceFragmentFrame<'face> {
    policy: DisplayRowSourceRequestPolicy,
    base_face_id: u32,
    base_face: &'face ResolvedFace,
}

impl<'face> DisplayRowSourceFragmentFrame<'face> {
    pub(crate) fn new(
        geometry: DisplayRowGeometry,
        role: GlyphRowRole,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        Self {
            policy: DisplayRowSourceRequestPolicy::from_display_row_geometry(geometry, role),
            base_face_id,
            base_face,
        }
    }

    pub(crate) fn render_request(
        self,
        render_bounds: DisplayRowRenderBounds,
    ) -> DisplayRowSourceFragmentRenderRequest<'face> {
        DisplayRowSourceFragmentRenderRequest::from_base_face_id_policy_with_render_bounds(
            self.policy,
            self.base_face_id,
            self.base_face,
            render_bounds,
        )
    }

    pub(crate) fn render_request_for_area(
        self,
        render_bounds: DisplayRowRenderBounds,
        area: GlyphArea,
    ) -> DisplayRowSourceFragmentRenderRequest<'face> {
        self.render_request(render_bounds).with_glyph_area(area)
    }

    pub(crate) fn from_glyph_row_columns(
        row: &GlyphRow,
        matrix_cols: usize,
        char_width: f32,
        role: GlyphRowRole,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        let char_width = char_width.max(1.0);
        let height = row.height_px.max(1.0);
        Self::new(
            DisplayRowGeometry::new(
                row.pixel_y,
                matrix_cols.max(1) as f32 * char_width,
                height,
                char_width,
                row.ascent_px.max(0.0).min(height),
                DisplayTabPolicy::every(8),
            ),
            role,
            base_face_id,
            base_face,
        )
    }

    pub(crate) fn from_row_geometry_columns(
        row_geometry: &DisplayRowGeometryState,
        columns: usize,
        char_width: f32,
        role: GlyphRowRole,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        let char_width = char_width.max(1.0);
        Self::new(
            DisplayRowGeometry::new(
                row_geometry.y(),
                columns.max(1) as f32 * char_width,
                row_geometry.height(),
                char_width,
                row_geometry.ascent(),
                DisplayTabPolicy::every(8),
            ),
            role,
            base_face_id,
            base_face,
        )
    }

    pub(crate) fn render_request_from_column(
        self,
        start_col: usize,
        max_col: usize,
    ) -> DisplayRowSourceFragmentRenderRequest<'face> {
        let char_width = self.policy.geometry.char_width;
        self.render_request(DisplayRowRenderBounds::new(
            DisplayRowPosition::new(start_col as f32 * char_width, start_col),
            DisplayRowMaxX::Bounded(max_col as f32 * char_width),
        ))
    }

    pub(crate) fn render_request_from_column_for_area(
        self,
        start_col: usize,
        max_col: usize,
        area: GlyphArea,
    ) -> DisplayRowSourceFragmentRenderRequest<'face> {
        self.render_request_from_column(start_col, max_col)
            .with_glyph_area(area)
    }
}

pub(crate) struct DisplayRowLispStringSourceSessionRequest {
    source_id: DisplayRowLispStringSourceId,
    value: Value,
    base_face_id: u32,
}

pub(crate) struct DisplayRowLispStringSourceRenderRequest<'a> {
    row_request: DisplayRowSourceRenderRequest<'a>,
    session_request: DisplayRowLispStringSourceSessionRequest,
}

impl<'a> DisplayRowLispStringSourceRenderRequest<'a> {
    pub(crate) fn from_value(row_request: DisplayRowSourceRenderRequest<'a>, value: Value) -> Self {
        let session_request = DisplayRowLispStringSourceSessionRequest::for_base_face_id(
            value,
            row_request.base_face_id(),
        );
        Self {
            row_request,
            session_request,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_origin_value(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        origin: DisplayOrigin,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        value: Value,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        let row_request = DisplayRowSourceRequestPolicy::from_origin(
            y, width, height, char_width, ascent, tab_policy, origin,
        )
        .with_symbol_values(symbol_values)
        .source_request_from_base_face(face_ids, base_face);
        Self::from_value(row_request, value)
    }

    fn into_render_parts(
        self,
    ) -> (
        DisplayRowRenderPlan<'a>,
        DisplayRowLispStringSourceSessionRequest,
    ) {
        (self.row_request.into_render_plan(), self.session_request)
    }
}

impl<'a> DisplayRowItemSourceRenderRequest<'a> {
    fn new(row_request: DisplayRowSourceRenderRequest<'a>) -> Self {
        Self { row_request }
    }

    fn from_base_face_id_policy_with_render_bounds(
        policy: DisplayRowSourceRequestPolicy,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        render_bounds: DisplayRowRenderBounds,
    ) -> Self {
        Self::new(
            policy
                .source_request_for_base_face_id(base_face_id, base_face)
                .with_render_bounds(render_bounds),
        )
    }

    fn with_glyph_area(mut self, area: GlyphArea) -> Self {
        self.row_request = self.row_request.with_glyph_area(area);
        self
    }

    fn into_render_plan(self) -> DisplayRowRenderPlan<'a> {
        self.row_request.into_render_plan()
    }

    #[cfg(test)]
    pub(crate) fn render<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<RenderedDisplayRow> {
        let mut context = DisplayRowRenderContext::new(face_resolver, None, face_ids);
        self.render_with_context(renderer, source, &mut context)
    }

    #[cfg(test)]
    fn render_with_context<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<RenderedDisplayRow> {
        let mut state = DisplayRowSourceState::default();
        self.render_step_with_context(renderer, source, &mut state, context)
            .map(DisplayRowRenderResult::into_rendered)
    }

    #[cfg(test)]
    pub(crate) fn render_step_with_context<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderResult> {
        renderer.render_display_item_source_row_step_with_context(
            self.into_render_plan(),
            source,
            state,
            context,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_fragment_step_with_display_host<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderResult> {
        let mut context = DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        renderer.render_display_item_source_row_fragment_step_with_context(
            self.into_render_plan(),
            source,
            state,
            &mut context,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_fragment_step_into_row_with_display_host<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let mut context = DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        renderer.render_display_item_source_row_fragment_step_into_row_with_context(
            self.into_render_plan(),
            row,
            source,
            state,
            &mut context,
        )
    }

    pub(crate) fn render_fragment_step_into_row_with_policy<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
        policy: &mut P,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        renderer.render_display_item_source_row_fragment_step_into_row_with_policy(
            self.into_render_plan(),
            row,
            source,
            state,
            context,
            policy,
        )
    }
}

impl<'a> DisplayRowSourceFragmentRenderRequest<'a> {
    fn from_base_face_id_policy_with_render_bounds(
        policy: DisplayRowSourceRequestPolicy,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        render_bounds: DisplayRowRenderBounds,
    ) -> Self {
        Self {
            item_request:
                DisplayRowItemSourceRenderRequest::from_base_face_id_policy_with_render_bounds(
                    policy,
                    base_face_id,
                    base_face,
                    render_bounds,
                ),
        }
    }

    pub(crate) fn with_glyph_area(mut self, area: GlyphArea) -> Self {
        self.item_request = self.item_request.with_glyph_area(area);
        self
    }

    fn into_item_request(self) -> DisplayRowItemSourceRenderRequest<'a> {
        self.item_request
    }

    #[cfg(test)]
    pub(crate) fn geometry(&self) -> &DisplayRowGeometry {
        self.item_request.row_request.geometry()
    }

    #[cfg(test)]
    pub(crate) fn render_bounds(&self) -> DisplayRowRenderBounds {
        self.item_request.row_request.render_bounds()
    }

    #[cfg(test)]
    pub(crate) fn glyph_area(&self) -> GlyphArea {
        self.item_request.row_request.area
    }

    #[cfg(test)]
    pub(crate) fn render<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<RenderedDisplayRow> {
        self.into_item_request()
            .render(renderer, source, face_resolver, face_ids)
    }

    #[cfg(test)]
    pub(crate) fn render_fragment_step_with_display_host<S: DisplayItemSource>(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<DisplayRowRenderResult> {
        self.into_item_request()
            .render_fragment_step_with_display_host(
                renderer,
                source,
                state,
                face_resolver,
                display_host,
                face_ids,
            )
    }
}

impl DisplayRowLispStringSourceSessionRequest {
    fn for_base_face_id(value: Value, base_face_id: u32) -> Self {
        Self {
            source_id: DisplayRowLispStringSourceId::ROOT,
            value,
            base_face_id,
        }
    }
}

pub(crate) struct DisplayRowLispStringSourceSession {
    source: LispStringSourceCursor,
    state: DisplayRowSourceState,
}

impl DisplayRowLispStringSourceSession {
    pub(crate) fn new(request: DisplayRowLispStringSourceSessionRequest) -> Option<Self> {
        let source = LispStringSourceCursor::new(
            request.source_id.raw(),
            request.value,
            RenderFaceRef::FaceId(request.base_face_id),
        )?;
        Some(Self {
            source,
            state: DisplayRowSourceState::default(),
        })
    }

    fn render_next_row_plan_with_context(
        &mut self,
        renderer: &mut DisplayRowRenderer<'_>,
        plan: DisplayRowRenderPlan<'_>,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderResult> {
        renderer.render_display_item_source_row_step_with_context(
            plan,
            &mut self.source,
            &mut self.state,
            context,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowGeometry {
    y: f32,
    width: f32,
    height: f32,
    char_width: f32,
    ascent: f32,
    tab_policy: DisplayTabPolicy,
}

impl DisplayRowGeometry {
    pub(crate) fn new(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
    ) -> Self {
        Self {
            y,
            width,
            height,
            char_width,
            ascent,
            tab_policy,
        }
    }

    pub(crate) fn y(&self) -> f32 {
        self.y
    }

    pub(crate) fn width(&self) -> f32 {
        self.width
    }

    pub(crate) fn height(&self) -> f32 {
        self.height
    }

    pub(crate) fn char_width(&self) -> f32 {
        self.char_width
    }

    pub(crate) fn ascent(&self) -> f32 {
        self.ascent
    }

    pub(crate) fn tab_policy(&self) -> &DisplayTabPolicy {
        &self.tab_policy
    }

    pub(crate) fn with_char_width(mut self, char_width: f32) -> Self {
        self.char_width = char_width;
        self
    }

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

#[derive(Clone, Debug, PartialEq)]
struct DisplayRowSourceGeometry {
    y: f32,
    width: f32,
    height: f32,
    char_width: f32,
    ascent: f32,
    tab_policy: DisplayTabPolicy,
}

impl DisplayRowSourceGeometry {
    fn new(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
    ) -> Self {
        Self {
            y,
            width,
            height,
            char_width,
            ascent,
            tab_policy,
        }
    }

    fn from_display_row_geometry(geometry: DisplayRowGeometry) -> Self {
        Self::new(
            geometry.y(),
            geometry.width(),
            geometry.height(),
            geometry.char_width(),
            geometry.ascent(),
            geometry.tab_policy().clone(),
        )
    }

    fn into_geometry(self) -> DisplayRowGeometry {
        DisplayRowGeometry::new(
            self.y,
            self.width,
            self.height,
            self.char_width,
            self.ascent,
            self.tab_policy,
        )
    }

    pub(crate) fn source_request_from_base_face<'face>(
        self,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'face ResolvedFace,
        role: GlyphRowRole,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> DisplayRowSourceRenderRequest<'face> {
        DisplayRowSourceRenderRequest::from_base_face(
            self.into_geometry(),
            face_ids,
            base_face,
            role,
            symbol_values,
        )
    }

    pub(crate) fn source_request_for_base_face_id<'face>(
        self,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
        role: GlyphRowRole,
    ) -> DisplayRowSourceRenderRequest<'face> {
        DisplayRowSourceRenderRequest::whole_row(
            self.into_geometry(),
            base_face_id,
            base_face,
            role,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DisplayRowSourceRequestPolicy {
    geometry: DisplayRowSourceGeometry,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
}

impl DisplayRowSourceRequestPolicy {
    fn new(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        role: GlyphRowRole,
    ) -> Self {
        Self {
            geometry: DisplayRowSourceGeometry::new(
                y, width, height, char_width, ascent, tab_policy,
            ),
            role,
            symbol_values: std::collections::HashMap::new(),
        }
    }

    fn from_display_row_geometry(geometry: DisplayRowGeometry, role: GlyphRowRole) -> Self {
        Self {
            geometry: DisplayRowSourceGeometry::from_display_row_geometry(geometry),
            role,
            symbol_values: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn from_origin(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        origin: DisplayOrigin,
    ) -> Self {
        let role = origin
            .glyph_row_role()
            .expect("display row source origin must map to a glyph row role");
        Self::new(y, width, height, char_width, ascent, tab_policy, role)
    }

    pub(crate) fn with_symbol_values(
        mut self,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        self.symbol_values = symbol_values;
        self
    }

    pub(crate) fn source_request_from_base_face<'face>(
        self,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'face ResolvedFace,
    ) -> DisplayRowSourceRenderRequest<'face> {
        self.geometry.source_request_from_base_face(
            face_ids,
            base_face,
            self.role,
            self.symbol_values,
        )
    }

    pub(crate) fn source_request_for_base_face_id<'face>(
        self,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> DisplayRowSourceRenderRequest<'face> {
        debug_assert!(self.symbol_values.is_empty());
        self.geometry
            .source_request_for_base_face_id(base_face_id, base_face, self.role)
    }
}

struct DisplayRowRenderPlan<'a> {
    geometry: DisplayRowGeometry,
    render_bounds: DisplayRowRenderBounds,
    area: GlyphArea,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
}

pub(crate) struct DisplayRowSourceRenderRequest<'a> {
    geometry: DisplayRowGeometry,
    render_bounds: DisplayRowRenderBounds,
    area: GlyphArea,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
}

impl<'a> DisplayRowSourceRenderRequest<'a> {
    fn whole_row(
        geometry: DisplayRowGeometry,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
    ) -> Self {
        let render_bounds = DisplayRowRenderBounds::whole_row(geometry.width());
        Self {
            geometry,
            render_bounds,
            area: GlyphArea::Text,
            base_face_id,
            base_face,
            role,
            symbol_values: std::collections::HashMap::new(),
        }
    }

    fn from_base_face(
        geometry: DisplayRowGeometry,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        let base_face_id = if base_face.face_id != 0 {
            base_face.face_id
        } else {
            face_ids.allocate()
        };
        let render_bounds = DisplayRowRenderBounds::whole_row(geometry.width());
        Self {
            geometry,
            render_bounds,
            area: GlyphArea::Text,
            base_face_id,
            base_face,
            role,
            symbol_values,
        }
    }

    pub(crate) fn from_display_row_geometry_for_base_face_id(
        geometry: DisplayRowGeometry,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
    ) -> Self {
        DisplayRowSourceRequestPolicy::from_display_row_geometry(geometry, role)
            .source_request_for_base_face_id(base_face_id, base_face)
    }

    #[cfg(test)]
    pub(crate) fn from_display_row_geometry(
        geometry: DisplayRowGeometry,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        DisplayRowSourceRequestPolicy::from_display_row_geometry(geometry, role)
            .with_symbol_values(symbol_values)
            .source_request_from_base_face(face_ids, base_face)
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_origin(
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        origin: DisplayOrigin,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        DisplayRowSourceRequestPolicy::from_origin(
            y, width, height, char_width, ascent, tab_policy, origin,
        )
        .with_symbol_values(symbol_values)
        .source_request_from_base_face(face_ids, base_face)
    }

    pub(crate) fn with_render_bounds(mut self, render_bounds: DisplayRowRenderBounds) -> Self {
        self.render_bounds = render_bounds;
        self
    }

    fn with_glyph_area(mut self, area: GlyphArea) -> Self {
        self.area = area;
        self
    }

    #[cfg(test)]
    pub(crate) fn base_face_ref(&self) -> RenderFaceRef {
        RenderFaceRef::FaceId(self.base_face_id)
    }

    pub(crate) fn base_face_id(&self) -> u32 {
        self.base_face_id
    }

    #[cfg(test)]
    pub(crate) fn base_face(&self) -> &'a ResolvedFace {
        self.base_face
    }

    #[cfg(test)]
    pub(crate) fn geometry(&self) -> &DisplayRowGeometry {
        &self.geometry
    }

    #[cfg(test)]
    pub(crate) fn render_bounds(&self) -> DisplayRowRenderBounds {
        self.render_bounds
    }

    pub(crate) fn role(&self) -> GlyphRowRole {
        self.role
    }

    pub(crate) fn render_fragment_step_into_row_with_policy<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
        render_policy: &mut P,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        DisplayRowItemSourceRenderRequest::new(self).render_fragment_step_into_row_with_policy(
            renderer,
            row,
            source,
            source_state,
            context,
            render_policy,
        )
    }

    #[cfg(test)]
    pub(crate) fn symbol_values(&self) -> &std::collections::HashMap<String, Value> {
        &self.symbol_values
    }

    fn into_render_plan(self) -> DisplayRowRenderPlan<'a> {
        DisplayRowRenderPlan {
            geometry: self.geometry,
            render_bounds: self.render_bounds,
            area: self.area,
            base_face_id: self.base_face_id,
            base_face: self.base_face,
            role: self.role,
            symbol_values: self.symbol_values,
        }
    }
}

fn clipped_display_item_remainder(
    item: DisplayItem,
    progress: &crate::display_row_builder::DisplayRowAppendProgress,
) -> Option<DisplayItem> {
    let DisplayItem {
        span,
        face,
        kind,
        layout,
    } = item;
    let emitted_chars = progress.slots().len();
    match kind {
        DisplayItemKind::TextRun(run) => {
            let (split_byte, remaining) = clipped_text_remainder(run.text.as_ref(), emitted_chars)?;
            Some(DisplayItem {
                span: SourceSpan::new(
                    display_source_position_advance(&span.start, emitted_chars, split_byte),
                    span.end,
                ),
                face,
                kind: DisplayItemKind::TextRun(DisplayTextRun::new(remaining)),
                layout,
            })
        }
        DisplayItemKind::SourceMappedText(text) => {
            let (_, remaining) = clipped_text_remainder(text.text.as_ref(), emitted_chars)?;
            Some(DisplayItem {
                span,
                face,
                kind: DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(remaining)),
                layout,
            })
        }
        _ => None,
    }
}

fn clipped_text_remainder(text: &str, emitted_chars: usize) -> Option<(usize, String)> {
    if emitted_chars >= text.chars().count() {
        return None;
    }
    let split_byte = text
        .char_indices()
        .nth(emitted_chars)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len());
    Some((split_byte, text[split_byte..].to_string()))
}

fn display_source_position_advance(
    start: &DisplaySourcePosition,
    char_offset: usize,
    byte_offset: usize,
) -> DisplaySourcePosition {
    match start {
        DisplaySourcePosition::Buffer {
            buffer_id,
            char_pos,
            byte_pos,
        } => DisplaySourcePosition::buffer(
            *buffer_id,
            CharPos0::new(char_pos.get() + char_offset),
            EmacsBytePos::new(byte_pos.get() + byte_offset),
        ),
        DisplaySourcePosition::LispString {
            source_id,
            char_index,
            byte_index,
        } => DisplaySourcePosition::lisp_string(
            source_id.get(),
            char_index + char_offset,
            byte_index + byte_offset,
        ),
        DisplaySourcePosition::Synthetic { source_id, offset } => {
            DisplaySourcePosition::synthetic(source_id.get(), offset + char_offset)
        }
    }
}

fn include_display_row_face_metrics(layout: &mut DisplayRowLayout, face: &DisplayRowFace) {
    face.metrics.include_in_layout(layout);
}

impl DisplayMediaReplacement {
    fn rendered_media(self, start: DisplayRowPosition, y: f32) -> RenderedDisplayRowMedia {
        RenderedDisplayRowMedia {
            kind: self.kind.into(),
            x: start.x_px(),
            y,
            col: start.col().min(usize::from(u16::MAX)) as u16,
            width: self.width,
            height: self.height,
        }
    }
}

struct DisplayRowRenderItem {
    source_item: DisplayItem,
    row_item: DisplayItem,
    media_descriptor: Option<DisplayMediaReplacement>,
}

impl DisplayRowRenderItem {
    fn from_source_item(source_item: DisplayItem) -> Self {
        let media_descriptor = match &source_item.kind {
            DisplayItemKind::MediaReplacement(media) => Some(*media),
            _ => None,
        };
        let row_item = media_descriptor
            .map(|descriptor| descriptor.replacement_item(source_item.clone()))
            .unwrap_or_else(|| source_item.clone());
        Self {
            source_item,
            row_item,
            media_descriptor,
        }
    }

    fn source_item(&self) -> &DisplayItem {
        &self.source_item
    }

    fn row_face(&self) -> RenderFaceRef {
        self.row_item.face
    }

    fn row_item(&self) -> &DisplayItem {
        &self.row_item
    }

    fn row_item_for_write(&self) -> DisplayItem {
        self.row_item.clone()
    }

    fn rendered_media_for_progress(
        &self,
        progress: &DisplayRowAppendProgress,
        y: f32,
    ) -> Option<RenderedDisplayRowMedia> {
        let descriptor = self.media_descriptor?;
        progress
            .is_complete_with_positive_width()
            .then(|| descriptor.rendered_media(progress.start(), y))
    }

    fn clipped_remainder(self, progress: &DisplayRowAppendProgress) -> Option<DisplayItem> {
        clipped_display_item_remainder(self.source_item, progress)
    }
}

pub(crate) struct DisplayRowRenderContext<'a, 'ids> {
    face_resolver: &'a FaceResolver,
    display_host: Option<&'a dyn DisplayHost>,
    face_ids: &'ids mut FrameFaceIdAllocator,
}

impl<'a, 'ids> DisplayRowRenderContext<'a, 'ids> {
    pub(crate) fn new(
        face_resolver: &'a FaceResolver,
        display_host: Option<&'a dyn DisplayHost>,
        face_ids: &'ids mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            face_resolver,
            display_host,
            face_ids,
        }
    }

    pub(crate) fn source_resolve_params<'b>(
        &self,
        base_face_id: u32,
        base_face: &'b ResolvedFace,
        fallback: DisplayRowFallbackMetrics,
    ) -> DisplaySourceResolveParams<'b>
    where
        'a: 'b,
    {
        DisplaySourceResolveParams::new(
            DisplaySourceFaceBasis::new(self.face_resolver, base_face_id, base_face, fallback),
            self.display_host.map(|host| host as &'b dyn DisplayHost),
        )
    }

    fn face_ids(&mut self) -> &mut FrameFaceIdAllocator {
        self.face_ids
    }
}

pub(crate) struct DisplayRowRenderer<'metrics> {
    font_metrics: &'metrics mut Option<FontMetricsService>,
}

pub(crate) struct DisplayRowRenderExecutor<'metrics, 'context, 'ids> {
    renderer: DisplayRowRenderer<'metrics>,
    context: DisplayRowRenderContext<'context, 'ids>,
}

impl<'metrics, 'context, 'ids> DisplayRowRenderExecutor<'metrics, 'context, 'ids> {
    pub(crate) fn new(
        font_metrics: &'metrics mut Option<FontMetricsService>,
        face_resolver: &'context FaceResolver,
        display_host: Option<&'context dyn DisplayHost>,
        face_ids: &'ids mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            renderer: DisplayRowRenderer::new(font_metrics),
            context: DisplayRowRenderContext::new(face_resolver, display_host, face_ids),
        }
    }

    pub(crate) fn render_lisp_string_source_request(
        &mut self,
        request: DisplayRowLispStringSourceRenderRequest<'_>,
    ) -> Option<RenderedDisplayRow> {
        let (plan, session_request) = request.into_render_parts();
        self.renderer
            .render_lisp_string_plan_with_context(plan, session_request, &mut self.context)
    }

    pub(crate) fn render_item_source_fragment_into_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowSourceFragmentRenderRequest<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        request
            .into_item_request()
            .render_fragment_step_into_row_with_policy(
                &mut self.renderer,
                row,
                source,
                source_state,
                &mut self.context,
                &mut NaturalDisplayRowRenderPolicy,
            )
    }
}

impl<'metrics> DisplayRowRenderer<'metrics> {
    pub(crate) fn new(font_metrics: &'metrics mut Option<FontMetricsService>) -> Self {
        Self { font_metrics }
    }

    fn render_lisp_string_plan_with_context(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        session_request: DisplayRowLispStringSourceSessionRequest,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<RenderedDisplayRow> {
        let mut session = DisplayRowLispStringSourceSession::new(session_request)?;
        session
            .render_next_row_plan_with_context(self, plan, context)
            .map(DisplayRowRenderResult::into_rendered)
    }

    fn render_display_item_source_row_step_with_context(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderResult> {
        let mut result = self.render_display_item_source_row_fragment_step_with_context(
            plan, source, state, context,
        )?;
        result.finalize_external_row();
        Some(result)
    }

    fn render_display_item_source_row_fragment_step_with_context(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderResult> {
        let mut row = new_display_row_for_role(plan.role);
        let result = self.render_display_item_source_row_fragment_step_into_row_with_context(
            plan, &mut row, source, state, context,
        )?;
        Some(result.with_row(row))
    }

    fn render_display_item_source_row_fragment_step_into_row_with_context(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        row: &mut GlyphRow,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let mut policy = NaturalDisplayRowRenderPolicy;
        self.render_display_item_source_row_fragment_step_into_row_with_policy(
            plan,
            row,
            source,
            state,
            context,
            &mut policy,
        )
    }

    fn render_display_item_source_row_fragment_step_into_row_with_policy<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        plan: DisplayRowRenderPlan<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        context: &mut DisplayRowRenderContext<'_, '_>,
        policy: &mut P,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        if state.is_finished() {
            return None;
        }

        let DisplayRowRenderPlan {
            geometry,
            render_bounds,
            area,
            base_face_id,
            base_face,
            role,
            symbol_values,
        } = plan;
        context.face_ids().reserve_after(base_face_id);
        let mut face_realizer = DisplayRowFaceRealizer::new(&mut *self.font_metrics);
        let row_face = face_realizer.realize_face(
            base_face_id,
            base_face,
            geometry.char_width(),
            geometry.ascent(),
            geometry.height(),
        );
        let char_width = face_realizer
            .char_width(&row_face, geometry.char_width())
            .max(1.0);
        let mut row_faces = vec![row_face.clone()];

        let parsed_symbol_values = symbol_values
            .into_iter()
            .filter_map(|(name, value)| parse_display_length_expr(value).map(|expr| (name, expr)))
            .collect();
        let row_ascent = row_face
            .metrics
            .ascent_px
            .max(geometry.ascent())
            .min(geometry.height().max(1.0));
        let mut row_layout = geometry.to_layout(
            role,
            char_width,
            row_ascent,
            RenderFaceRef::FaceId(row_face.face_id),
            parsed_symbol_values,
        );
        let mut position = render_bounds.start();
        let mut source_slots = Vec::new();
        let mut media = Vec::new();
        let fallback_metrics = DisplayRowFallbackMetrics::from_default_face_extents(
            char_width,
            geometry.height(),
            geometry.ascent(),
        );
        let stop = loop {
            let params =
                context.source_resolve_params(row_face.face_id, base_face, fallback_metrics);
            let resolved = state.next_resolved_item(source, params, context.face_ids());
            let (item, pending_faces) = resolved.into_parts();
            for pending in pending_faces {
                let (face_id, resolved) = pending.into_parts();
                let row_face = face_realizer.realize_face(
                    face_id,
                    &resolved,
                    char_width,
                    geometry.ascent(),
                    geometry.height(),
                );
                include_display_row_face_metrics(&mut row_layout, &row_face);
                row_faces.push(row_face);
            }
            let Some(item) = item else {
                break DisplayRowRenderStop::SourceExhausted;
            };
            if policy.stop_before_item(&item) {
                break DisplayRowRenderStop::SourceExhausted;
            }
            if let RenderFaceRef::FaceId(face_id) = item.face {
                if face_id != row_face.face_id
                    && !row_faces.iter().any(|face| face.face_id == face_id)
                    && let Some(resolved) = state.resolved_face(face_id).cloned()
                {
                    let realized = face_realizer.realize_face(
                        face_id,
                        &resolved,
                        char_width,
                        geometry.ascent(),
                        geometry.height(),
                    );
                    include_display_row_face_metrics(&mut row_layout, &realized);
                    row_faces.push(realized);
                }
            }
            let render_item = DisplayRowRenderItem::from_source_item(item);
            let item_face_id = render_face_ref_id(render_item.row_face(), row_face.face_id);
            let measurement = policy.measurement_for(
                render_item.row_item(),
                item_face_id,
                face_realizer.font_metrics_mut(),
            );
            let progress = match measurement {
                DisplayRowItemMeasurement::Default => {
                    let mut glyph_measurer = DisplayRowGlyphMeasurer::new(
                        &row_faces,
                        face_realizer.font_metrics_service_mut(),
                        char_width,
                    );
                    let mut row_writer = DisplayRowProgressWriter::with_glyph_measurer_for_area(
                        &row_layout,
                        &mut *row,
                        &mut glyph_measurer,
                        position,
                        render_bounds.max_x().to_f32(),
                        area,
                    );
                    row_writer.push_item(render_item.row_item_for_write())
                }
                DisplayRowItemMeasurement::TextRun(measurement) => {
                    let mut row_writer =
                        DisplayRowProgressWriter::with_text_run_measurement_for_area(
                            &row_layout,
                            &mut *row,
                            measurement,
                            position,
                            render_bounds.max_x().to_f32(),
                            area,
                        );
                    row_writer.push_item(render_item.row_item_for_write())
                }
            };
            position = progress.end();
            source_slots.extend(progress.slots().iter().cloned());
            if let Some(rendered) =
                render_item.rendered_media_for_progress(&progress, row_layout.y_px)
            {
                media.push(rendered);
            }
            match progress.status() {
                DisplayRowAppendStatus::Complete => {}
                DisplayRowAppendStatus::Clipped => {
                    match policy.clipped_behavior(render_item.source_item()) {
                        DisplayRowRenderClipBehavior::PreserveRemainderAndStop => {
                            state.remember_pending_item(render_item.clipped_remainder(&progress));
                            break DisplayRowRenderStop::Clipped;
                        }
                        DisplayRowRenderClipBehavior::Stop => {
                            break DisplayRowRenderStop::Clipped;
                        }
                        DisplayRowRenderClipBehavior::Continue => {}
                    }
                }
                DisplayRowAppendStatus::RowBreak => {
                    break DisplayRowRenderStop::RowBreak;
                }
            }
        };
        let progress_height = if row.height_px > 0.0 {
            row.height_px
        } else {
            row_layout.height_px
        };
        let progress = display_row_progress(position, geometry.y(), progress_height);
        let faces = row_faces
            .into_iter()
            .map(|face| face.render_face())
            .collect();
        Some(DisplayRowRenderIntoRowResult::new(
            progress,
            source_slots,
            faces,
            media,
            stop,
        ))
    }
}

#[cfg(test)]
#[path = "display_row_test.rs"]
mod tests;
