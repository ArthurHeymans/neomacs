use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLengthExpr, DisplayMediaReplacement,
    DisplayMediaReplacementKind, DisplaySourceMappedText, DisplaySourcePosition, DisplayTextRun,
    RenderFaceRef, SourceSpan,
};
use crate::display_origin::DisplayOrigin;
use crate::display_property::parse_display_length_expr;
#[cfg(test)]
use crate::display_row_builder::display_row_text_is_empty;
use crate::display_row_builder::{
    DisplayGlyphMeasurer, DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowGlyphSlot,
    DisplayRowItemMeasurement, DisplayRowLayout, DisplayRowPosition, DisplayRowProgressWriter,
    DisplayTabPolicy, apply_display_row_source_slot_bounds, merge_display_row_source_slot_bounds,
    new_display_row_for_role,
};
use crate::display_row_geometry::DisplayRowGeometryState;
pub(crate) use crate::display_row_geometry::DisplayRowMaxX;
use crate::display_source::{DisplayItemSource, LispStringSourceCursor};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_source_resolver::{
    DisplaySourceFaceBasis, DisplaySourceFallbackMetrics, DisplaySourceResolveParams,
    DisplaySourceResolveState, ResolvedDisplaySourceItem, resolve_next_display_source_item,
};
use crate::display_text_run_measurement::{
    ComplexTextRunAdvancePolicy, DisplayTextRunAdvance, DisplayTextRunMeasurement,
    DisplayTextRunMeasurementPlan,
};
use crate::font_metrics::{FontMetrics, FontMetricsService};
use crate::glyph_advance::GlyphAdvanceQuantization;
use crate::glyph_row_writer;
use crate::matrix_builder::{
    FRAME_CHROME_WINDOW_ID, GlyphMatrixBuilder, MatrixFrameStateInstallRequest,
    MatrixMediaInstallKind, MatrixMediaInstallRequest, MatrixRowBeginRequest,
    ResolvedMatrixMediaInstallTarget,
};
use crate::neovm_bridge::FaceResolver;
use crate::neovm_bridge::ResolvedFace;
use crate::window_output::{TextRowOutput, WindowOutputEmitter};
use neomacs_display_protocol::face::{BoxType, Face, FaceAttributes, UnderlineStyle};
use neomacs_display_protocol::frame_glyphs::{DisplaySlotId, GlyphRowRole};
use neomacs_display_protocol::glyph_matrix::{FrameChromeRow, GlyphArea, GlyphRow};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{CharPos0, EmacsBytePos};
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::{Context, Value};

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
    builder.install_frame_state(MatrixFrameStateInstallRequest::Face {
        id: render_face.face_id,
        face: rendered,
    });
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
        let fallback_char_width = fallback_char_width.max(1.0);
        if face.font_char_width > 0.0 {
            return face.font_char_width.max(fallback_char_width);
        }
        if let Some(svc) = self.font_metrics.as_mut() {
            let metrics = svc.font_metrics(
                &face.font_family,
                face.font_weight,
                face.italic,
                face.font_size,
            );
            return metrics.char_width.max(fallback_char_width);
        }
        fallback_char_width
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
        let needs_metrics = face.font_char_width <= 0.0
            || face.font_ascent <= 0.0
            || (face.font_ascent + face.font_descent as f32) <= 0.0;

        if needs_metrics && let Some(metrics) = self.measured_font_metrics_for_face(face) {
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
        face.font_char_width = face.font_char_width.max(fallback_char_width.max(1.0));
        if face.font_ascent <= 0.0 {
            face.font_ascent = fallback_ascent.max(1.0);
        }
        if (face.font_ascent + face.font_descent as f32) <= 0.0
            || (face.font_descent <= 0 && row_height > face.font_ascent)
        {
            face.font_descent = (row_height - face.font_ascent).max(0.0).ceil() as i32;
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
            .font_char_width
            .max(self.fallback_char_width)
            .max(fallback_char_width_px)
            .max(1.0);
        DisplayTextRunMeasurementPlan::from_shaped_glyphs(
            text,
            shaped,
            face_char_width,
            fallback_char_width_px,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowFallbackMetrics {
    pub(crate) char_width: f32,
    pub(crate) row_height: f32,
    pub(crate) ascent: f32,
}

impl DisplayRowFallbackMetrics {
    pub(crate) fn from_default_face_extents(char_width: f32, row_height: f32, ascent: f32) -> Self {
        Self {
            char_width,
            row_height,
            ascent,
        }
    }
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
        let measurement_face = self.measurement_face(face_id, face, metrics, fallback_char_width);
        let space_width =
            measurement_face.advance_for_char(font_metrics, ' ', fallback_metrics.char_width);
        let (char_width, row_height, ascent) = metrics
            .map(|metrics| (metrics.char_width, metrics.line_height, metrics.ascent))
            .unwrap_or((
                fallback_metrics.char_width,
                fallback_metrics.row_height,
                fallback_metrics.ascent,
            ));
        DisplayRowMeasuredFace {
            measurement_face,
            metrics: DisplayRowMeasuredFaceMetrics {
                char_width,
                row_height,
                ascent,
                space_width,
            },
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
        Self {
            face,
            mode,
            fallback_char_width,
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
                let fallback_advance_px = self.fallback_char_width * f32::from(columns);
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowMeasuredFaceMetrics {
    pub(crate) char_width: f32,
    pub(crate) row_height: f32,
    pub(crate) ascent: f32,
    pub(crate) space_width: f32,
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

    pub(crate) fn install_into(&self, builder: &mut GlyphMatrixBuilder) {
        insert_resolved_display_row_face(builder, self.face_id(), &self.face, self.metrics);
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

pub(crate) struct DisplayRowComplexTextRunAdvancePolicy<'a> {
    active_face_state: &'a DisplayRowActiveFaceState,
    font_metrics: &'a mut Option<FontMetricsService>,
}

impl<'a> DisplayRowComplexTextRunAdvancePolicy<'a> {
    pub(crate) fn new(
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
        let fallback_advance_px = self.metrics().char_width * columns as f32;
        self.advance_for_char(font_metrics, ch, fallback_advance_px)
    }

    pub(crate) fn text_run_measurement(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        text: &str,
    ) -> DisplayTextRunMeasurement {
        self.measurement.text_run_measurement(font_metrics, text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DisplayRowOutputProgress {
    pub end_x: f32,
    pub end_col: i64,
    pub y: f32,
    pub height: f32,
}

pub(crate) struct RenderedDisplayRow {
    pub(crate) row: GlyphRow,
    pub(crate) progress: DisplayRowOutputProgress,
    pub(crate) source_slots: Vec<DisplayRowGlyphSlot>,
    pub(crate) faces: Vec<Face>,
    pub(crate) media: Vec<RenderedDisplayRowMedia>,
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
    pub(crate) owner: DisplayRowOwner,
    pub(crate) row_index: u32,
    pub(crate) bounds: Rect,
    pub(crate) rendered: RenderedDisplayRow,
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
    for glyph in rendered.row.glyphs.iter().flatten() {
        if let Some(face) = rendered.faces.iter().find(|face| face.id == glyph.face_id) {
            let face_height = (face.font_ascent + face.font_descent).max(1) as f32;
            height = height.max(face_height + glyph.vertical_offset_px.abs());
        }
    }
    for media in &rendered.media {
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
                .max(rendered.row.height_px)
                .max(rendered.progress.height)
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

    pub(crate) fn row_height(&self) -> f32 {
        self.bounds.height.max(1.0)
    }

    pub(crate) fn row_ascent(&self) -> f32 {
        self.rendered.row.ascent_px.max(0.0).min(self.row_height())
    }

    pub(crate) fn output_progress(&self) -> DisplayRowOutputProgress {
        DisplayRowOutputProgress {
            y: self.bounds.y,
            height: self.bounds.height,
            ..self.rendered.progress
        }
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
            return ResolvedDisplaySourceItem {
                item: None,
                pending_faces: Vec::new(),
            };
        }
        if let Some(item) = self.take_pending_item() {
            return ResolvedDisplaySourceItem {
                item: Some(item),
                pending_faces: Vec::new(),
            };
        }
        let resolved =
            resolve_next_display_source_item(source, params, &mut self.resolve_state, face_ids);
        if resolved.item.is_none() {
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
    pub(crate) item: DisplayItem,
    pub(crate) pending_faces: Vec<PendingDisplaySourceFace>,
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
            DisplaySourceFallbackMetrics::new(
                fallback_char_width,
                fallback_ascent,
                fallback_row_height,
            ),
        );
        let resolved = self.state.next_resolved_item(
            &mut self.source,
            DisplaySourceResolveParams::new(face_basis, display_host),
            face_ids,
        );
        resolved.item.map(|item| DisplayRowSourceStep {
            item,
            pending_faces: resolved.pending_faces,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowRenderStop {
    SourceExhausted,
    Clipped,
    RowBreak,
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

pub(crate) struct NaturalDisplayRowAppendRenderPolicy;

impl DisplayRowRenderPolicy for NaturalDisplayRowAppendRenderPolicy {}

pub(crate) struct ResolvedSourceAdvanceRenderPolicy {
    advance_px: f32,
}

impl ResolvedSourceAdvanceRenderPolicy {
    pub(crate) fn new(advance_px: f32) -> Self {
        Self { advance_px }
    }

    fn measurement_for_text(&self, text: &str) -> DisplayRowItemMeasurement {
        DisplayRowItemMeasurement::TextRun(
            DisplayTextRunMeasurementPlan::from_resolved_source_advance(text, self.advance_px),
        )
    }
}

impl DisplayRowRenderPolicy for ResolvedSourceAdvanceRenderPolicy {
    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        _face_id: u32,
        _font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        match &item.kind {
            DisplayItemKind::TextRun(run) => self.measurement_for_text(&run.text),
            DisplayItemKind::SourceMappedText(text) => self.measurement_for_text(&text.text),
            _ => DisplayRowItemMeasurement::Default,
        }
    }
}

pub(crate) enum DisplaySourceAppendRenderPolicy {
    Natural(NaturalDisplayRowAppendRenderPolicy),
    Resolved(ResolvedSourceAdvanceRenderPolicy),
}

impl DisplaySourceAppendRenderPolicy {
    pub(crate) fn natural() -> Self {
        Self::Natural(NaturalDisplayRowAppendRenderPolicy)
    }

    pub(crate) fn resolved_advance(advance_px: f32) -> Self {
        Self::Resolved(ResolvedSourceAdvanceRenderPolicy::new(advance_px))
    }
}

impl DisplayRowRenderPolicy for DisplaySourceAppendRenderPolicy {
    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        face_id: u32,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        match self {
            Self::Natural(policy) => policy.measurement_for(item, face_id, font_metrics),
            Self::Resolved(policy) => policy.measurement_for(item, face_id, font_metrics),
        }
    }
}

pub(crate) struct DisplayRowRenderResult {
    pub(crate) rendered: RenderedDisplayRow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DisplayRowLispStringSourceId(u64);

impl DisplayRowLispStringSourceId {
    const ROOT: Self = Self(1);

    fn raw(self) -> u64 {
        self.0
    }
}

pub(crate) struct DisplayRowLispStringRenderRequest<'a> {
    row_request: DisplayRowSourceRenderRequest<'a>,
    value: Value,
}

pub(crate) struct DisplayRowItemSourceRenderRequest<'a> {
    row_request: DisplayRowSourceRenderRequest<'a>,
}

pub(crate) struct DisplayRowLispStringSourceSessionRequest {
    source_id: DisplayRowLispStringSourceId,
    value: Value,
    base_face_id: u32,
}

impl<'a> DisplayRowLispStringRenderRequest<'a> {
    fn new(row_request: DisplayRowSourceRenderRequest<'a>, value: Value) -> Self {
        Self { row_request, value }
    }

    pub(crate) fn from_base_face_policy(
        policy: DisplayRowSourceRequestPolicy,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &'a ResolvedFace,
        value: Value,
    ) -> Self {
        Self::new(
            policy.source_request_from_base_face(face_ids, base_face),
            value,
        )
    }

    fn into_render_parts(
        self,
    ) -> (
        DisplayRowRenderPlan<'a>,
        DisplayRowLispStringSourceSessionRequest,
    ) {
        let plan = self.row_request.into_render_plan();
        let session_request = DisplayRowLispStringSourceSessionRequest::for_base_face_id(
            self.value,
            plan.base_face_id,
        );
        (plan, session_request)
    }

    pub(crate) fn render_with_context(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<RenderedDisplayRow> {
        renderer.render_lisp_string_request_with_context(self, context)
    }

    #[cfg(test)]
    pub(crate) fn render(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<RenderedDisplayRow> {
        let mut context = DisplayRowRenderContext::new(face_resolver, None, face_ids);
        self.render_with_context(renderer, &mut context)
    }

    #[cfg(test)]
    pub(crate) fn render_with_display_host(
        self,
        renderer: &mut DisplayRowRenderer<'_>,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<RenderedDisplayRow> {
        let mut context = DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        self.render_with_context(renderer, &mut context)
    }
}

impl<'a> DisplayRowItemSourceRenderRequest<'a> {
    fn new(row_request: DisplayRowSourceRenderRequest<'a>) -> Self {
        Self { row_request }
    }

    pub(crate) fn from_base_face_id_policy_with_render_bounds(
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

    pub(crate) fn with_glyph_area(mut self, area: GlyphArea) -> Self {
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
            .map(|result| result.rendered)
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

pub(crate) struct DisplayRowRenderIntoRowResult {
    pub(crate) progress: DisplayRowOutputProgress,
    pub(crate) source_slots: Vec<DisplayRowGlyphSlot>,
    pub(crate) faces: Vec<Face>,
    pub(crate) media: Vec<RenderedDisplayRowMedia>,
    pub(crate) stop: DisplayRowRenderStop,
}

impl DisplayRowRenderIntoRowResult {
    fn with_row(self, row: GlyphRow) -> DisplayRowRenderResult {
        DisplayRowRenderResult {
            rendered: RenderedDisplayRow {
                row,
                progress: self.progress,
                source_slots: self.source_slots,
                faces: self.faces,
                media: self.media,
            },
        }
    }
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
            geometry.y,
            geometry.width,
            geometry.height,
            geometry.char_width,
            geometry.ascent,
            geometry.tab_policy,
        )
    }

    fn into_geometry(self) -> DisplayRowGeometry {
        DisplayRowGeometry {
            y: self.y,
            width: self.width,
            height: self.height,
            char_width: self.char_width,
            ascent: self.ascent,
            tab_policy: self.tab_policy,
        }
    }

    fn source_request_from_base_face<'face>(
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
pub(crate) struct DisplayRowSourceRequestPolicy {
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

    pub(crate) fn from_display_row_geometry(
        geometry: DisplayRowGeometry,
        role: GlyphRowRole,
    ) -> Self {
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

    #[cfg(test)]
    pub(crate) fn role(&self) -> GlyphRowRole {
        self.role
    }

    #[cfg(test)]
    pub(crate) fn geometry(&self) -> DisplayRowGeometry {
        self.geometry.clone().into_geometry()
    }

    #[cfg(test)]
    pub(crate) fn symbol_values(&self) -> &std::collections::HashMap<String, Value> {
        &self.symbol_values
    }

    fn source_request_from_base_face<'face>(
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

pub(crate) struct DisplayRowCurrentTextRenderState<'face, 'emit> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) output_emitter: &'emit mut WindowOutputEmitter,
    pub(crate) evaluator: &'emit mut Context,
    pub(crate) font_metrics: &'emit mut Option<FontMetricsService>,
    pub(crate) face_resolver: &'face FaceResolver,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct DisplayRowCurrentTextMeasureState<'face, 'emit> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) evaluator: &'emit mut Context,
    pub(crate) font_metrics: &'emit mut Option<FontMetricsService>,
    pub(crate) face_resolver: &'face FaceResolver,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

impl<'a> DisplayRowSourceRenderRequest<'a> {
    fn whole_row(
        geometry: DisplayRowGeometry,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        role: GlyphRowRole,
    ) -> Self {
        let render_bounds = DisplayRowRenderBounds::whole_row(geometry.width);
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
        let render_bounds = DisplayRowRenderBounds::whole_row(geometry.width);
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowRenderBounds {
    pub(crate) start: DisplayRowPosition,
    pub(crate) max_x: DisplayRowMaxX,
}

impl DisplayRowRenderBounds {
    pub(crate) fn whole_row(width_px: f32) -> Self {
        Self {
            start: DisplayRowPosition { x_px: 0.0, col: 0 },
            max_x: DisplayRowMaxX::Bounded(width_px.max(0.0)),
        }
    }

    pub(crate) fn unbounded_from(start: DisplayRowPosition) -> Self {
        Self {
            start,
            max_x: DisplayRowMaxX::Unbounded,
        }
    }
}

pub(crate) struct CurrentTextRowRenderOutcome {
    pub(crate) stop: DisplayRowRenderStop,
    pub(crate) source_slots: Vec<DisplayRowGlyphSlot>,
    pub(crate) end: DisplayRowPosition,
    pub(crate) row_height_px: f32,
    pub(crate) row_ascent_px: f32,
}

impl CurrentTextRowRenderOutcome {
    pub(crate) fn stop(&self) -> DisplayRowRenderStop {
        self.stop
    }

    pub(crate) fn source_slots(&self) -> &[DisplayRowGlyphSlot] {
        &self.source_slots
    }

    pub(crate) fn end_position(&self) -> DisplayRowPosition {
        self.end
    }

    pub(crate) fn include_vertical_metrics(&self, geometry: &mut DisplayRowGeometryState) {
        geometry.include_glyph_vertical_metrics(self.row_height_px, self.row_ascent_px);
    }

    pub(crate) fn into_append_progress(
        self,
        start: DisplayRowPosition,
    ) -> DisplayRowAppendProgress {
        display_row_append_progress_from_render_result(
            start,
            self.end,
            self.stop,
            self.source_slots,
        )
    }

    pub(crate) fn into_append_progress_and_position(
        self,
        start: DisplayRowPosition,
    ) -> (DisplayRowAppendProgress, DisplayRowPosition) {
        let end = self.end;
        (self.into_append_progress(start), end)
    }
}

fn display_row_append_progress_from_render_result(
    start: DisplayRowPosition,
    end: DisplayRowPosition,
    stop: DisplayRowRenderStop,
    slots: Vec<DisplayRowGlyphSlot>,
) -> DisplayRowAppendProgress {
    DisplayRowAppendProgress::from_positions(
        start,
        end,
        match stop {
            DisplayRowRenderStop::SourceExhausted => DisplayRowAppendStatus::Complete,
            DisplayRowRenderStop::Clipped => DisplayRowAppendStatus::Clipped,
            DisplayRowRenderStop::RowBreak => DisplayRowAppendStatus::RowBreak,
        },
        slots,
    )
}

struct DisplayRowCurrentTextSourceRenderRequest<'row, 'source, 'state, 'policy, S, P> {
    row_request: DisplayRowSourceRenderRequest<'row>,
    source: &'source mut S,
    source_state: &'state mut DisplayRowSourceState,
    render_policy: &'policy mut P,
}

struct DisplayRowCurrentTextSourceStepResult {
    role: GlyphRowRole,
    result: DisplayRowRenderIntoRowResult,
    row_height_px: f32,
    row_ascent_px: f32,
}

struct DisplayRowCurrentSourceSlotBoundsMergeRequest<'a> {
    slots: &'a [DisplayRowGlyphSlot],
}

impl<'a> DisplayRowCurrentSourceSlotBoundsMergeRequest<'a> {
    fn new(slots: &'a [DisplayRowGlyphSlot]) -> Self {
        Self { slots }
    }

    fn install(self, builder: &mut GlyphMatrixBuilder) {
        builder.with_current_row_mut(|row| {
            merge_display_row_source_slot_bounds(row, self.slots);
        });
    }
}

impl<'row, 'source, 'state, 'policy, S, P>
    DisplayRowCurrentTextSourceRenderRequest<'row, 'source, 'state, 'policy, S, P>
where
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
{
    fn new(
        row_request: DisplayRowSourceRenderRequest<'row>,
        source: &'source mut S,
        source_state: &'state mut DisplayRowSourceState,
        render_policy: &'policy mut P,
    ) -> Self {
        Self {
            row_request,
            source,
            source_state,
            render_policy,
        }
    }

    fn render_into_current_row(
        self,
        state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
    ) -> Option<DisplayRowCurrentTextSourceStepResult> {
        let Self {
            row_request,
            source,
            source_state,
            render_policy,
        } = self;
        let role = row_request.role();
        let DisplayRowCurrentTextRenderState {
            builder,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
            ..
        } = state;
        let mut renderer = DisplayRowRenderer::new(font_metrics);
        let mut context = DisplayRowRenderContext::new(
            face_resolver,
            evaluator.display_host.as_deref(),
            face_ids,
        );
        builder
            .with_current_row_mut(|row| {
                let result = DisplayRowItemSourceRenderRequest::new(row_request)
                    .render_fragment_step_into_row_with_policy(
                        &mut renderer,
                        row,
                        source,
                        source_state,
                        &mut context,
                        render_policy,
                    )?;
                Some(DisplayRowCurrentTextSourceStepResult {
                    role,
                    result,
                    row_height_px: row.height_px,
                    row_ascent_px: row.ascent_px,
                })
            })
            .flatten()
    }

    fn measure_against_current_row(
        self,
        state: &mut DisplayRowCurrentTextMeasureState<'_, '_>,
    ) -> Option<DisplayRowCurrentTextSourceStepResult> {
        let Self {
            row_request,
            source,
            source_state,
            render_policy,
        } = self;
        let role = row_request.role();
        let mut renderer = DisplayRowRenderer::new(state.font_metrics);
        let mut scratch_row = state.builder.current_row()?.clone();
        let mut context = DisplayRowRenderContext::new(
            state.face_resolver,
            state.evaluator.display_host.as_deref(),
            state.face_ids,
        );
        let result = DisplayRowItemSourceRenderRequest::new(row_request)
            .render_fragment_step_into_row_with_policy(
                &mut renderer,
                &mut scratch_row,
                source,
                source_state,
                &mut context,
                render_policy,
            )?;
        let row_height_px = scratch_row.height_px;
        let row_ascent_px = scratch_row.ascent_px;
        Some(DisplayRowCurrentTextSourceStepResult {
            role,
            result,
            row_height_px,
            row_ascent_px,
        })
    }
}

impl DisplayRowCurrentTextSourceStepResult {
    fn finish_and_emit(
        self,
        state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
        output: TextRowOutput,
    ) -> CurrentTextRowRenderOutcome {
        finish_current_text_row_render(
            state,
            output,
            self.role,
            self.result,
            self.row_height_px,
            self.row_ascent_px,
        )
    }

    fn into_measure_outcome(self) -> CurrentTextRowRenderOutcome {
        let end = display_row_output_end_position(self.result.progress);
        let source_slots = self.result.source_slots;
        CurrentTextRowRenderOutcome {
            stop: self.result.stop,
            source_slots,
            end,
            row_height_px: self.row_height_px,
            row_ascent_px: self.row_ascent_px,
        }
    }
}

pub(crate) fn render_display_item_source_into_current_text_row_and_emit<
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
>(
    state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    request: DisplayRowSourceRenderRequest<'_>,
    output: TextRowOutput,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome> {
    DisplayRowCurrentTextSourceRenderRequest::new(request, source, source_state, render_policy)
        .render_into_current_row(state)
        .map(|result| result.finish_and_emit(state, output))
}

pub(crate) fn measure_display_item_source_against_current_text_row<
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
>(
    state: &mut DisplayRowCurrentTextMeasureState<'_, '_>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    request: DisplayRowSourceRenderRequest<'_>,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome> {
    DisplayRowCurrentTextSourceRenderRequest::new(request, source, source_state, render_policy)
        .measure_against_current_row(state)
        .map(DisplayRowCurrentTextSourceStepResult::into_measure_outcome)
}

fn finish_current_text_row_render(
    state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
    output: TextRowOutput,
    role: GlyphRowRole,
    result: DisplayRowRenderIntoRowResult,
    row_height_px: f32,
    row_ascent_px: f32,
) -> CurrentTextRowRenderOutcome {
    let end = display_row_output_end_position(result.progress);
    RenderedDisplayRowAssetsInstall::fragment(role, output.row, &result.faces, &result.media)
        .install(state.builder);
    DisplayRowCurrentSourceSlotBoundsMergeRequest::new(&result.source_slots).install(state.builder);
    let source_slots = result.source_slots;
    state
        .output_emitter
        .emit_text_source_slots(state.evaluator, output, &source_slots, end);
    CurrentTextRowRenderOutcome {
        stop: result.stop,
        source_slots,
        end,
        row_height_px,
        row_ascent_px,
    }
}

pub(crate) fn install_display_row_in_matrix_row(
    builder: &mut GlyphMatrixBuilder,
    matrix_row: usize,
    row: &GlyphRow,
) {
    let context = builder.current_window_row_install_context();
    let mut row = row.clone();
    row.pixel_y -= context.pixel_bounds.y;
    builder.install_prebuilt_row(
        MatrixRowBeginRequest {
            row: matrix_row,
            role: row.role,
            mode_line: row.mode_line,
        },
        row,
    );
}

pub(crate) fn install_measured_window_display_row(
    builder: &mut GlyphMatrixBuilder,
    measured: &MeasuredDisplayRow,
) {
    let DisplayRowOwner::WindowChrome { window_id, kind } = measured.owner else {
        panic!("frame chrome rows must use install_measured_frame_chrome_row");
    };
    debug_assert!(window_id > 0);
    debug_assert_eq!(
        builder.current_window_media_install_context().window_id,
        window_id as i64
    );
    debug_assert!(matches!(
        kind,
        WindowChromeKind::TabLine | WindowChromeKind::HeaderLine | WindowChromeKind::ModeLine
    ));
    let matrix_row = measured.row_index as usize;
    RenderedDisplayRowAssetsInstall::from_rendered(
        &measured.rendered,
        RenderedDisplayRowAssetInstallTarget::WindowRow {
            row_index: measured.row_index,
            bounds: measured.bounds,
        },
    )
    .install(builder);
    let mut row = rendered_row_with_source_bounds(&measured.rendered);
    row.pixel_y = measured.bounds.y;
    row.height_px = measured.row_height();
    row.ascent_px = measured.row_ascent();
    install_display_row_in_matrix_row(builder, matrix_row, &row);
}

pub(crate) fn install_measured_frame_chrome_row(
    builder: &mut GlyphMatrixBuilder,
    frame_chrome_rows: &mut Vec<FrameChromeRow>,
    measured: &MeasuredDisplayRow,
) {
    let DisplayRowOwner::FrameChrome { kind } = measured.owner else {
        panic!("window-owned rows must use install_measured_window_display_row");
    };
    debug_assert!(matches!(kind, FrameChromeKind::TabBar));
    RenderedDisplayRowAssetsInstall::from_rendered(
        &measured.rendered,
        RenderedDisplayRowAssetInstallTarget::FrameChrome {
            row_index: measured.row_index,
            bounds: measured.bounds,
        },
    )
    .install(builder);
    let mut row = measured.rendered.row.clone();
    apply_display_row_source_slot_bounds(&mut row, &measured.rendered.source_slots);
    // Frame chrome (tab-bar) rows are stored in a side vector and never pass
    // through the matrix-row lifecycle, so they reorder here. Render no longer
    // reorders (the single matrix-row finalizer is `end_current_row`), so the
    // cloned row is un-reordered and must be finalized once on the clone.
    let _ = crate::glyph_row_writer::reorder_row_bidi(&mut row, None);
    row.pixel_y = measured.bounds.y;
    row.height_px = measured.row_height();
    row.ascent_px = measured.row_ascent();
    frame_chrome_rows.push(FrameChromeRow {
        row_index: measured.row_index,
        pixel_bounds: measured.bounds,
        row,
    });
}

#[derive(Clone, Copy)]
enum RenderedDisplayRowAssetInstallTarget {
    MatrixRow(usize),
    WindowRow { row_index: u32, bounds: Rect },
    FrameChrome { row_index: u32, bounds: Rect },
}

struct RenderedDisplayRowAssetsInstall<'a> {
    role: GlyphRowRole,
    faces: &'a [Face],
    media: &'a [RenderedDisplayRowMedia],
    target: RenderedDisplayRowAssetInstallTarget,
}

impl<'a> RenderedDisplayRowAssetsInstall<'a> {
    fn fragment(
        role: GlyphRowRole,
        matrix_row: usize,
        faces: &'a [Face],
        media: &'a [RenderedDisplayRowMedia],
    ) -> Self {
        Self {
            role,
            faces,
            media,
            target: RenderedDisplayRowAssetInstallTarget::MatrixRow(matrix_row),
        }
    }

    fn from_rendered(
        rendered: &'a RenderedDisplayRow,
        target: RenderedDisplayRowAssetInstallTarget,
    ) -> Self {
        Self {
            role: rendered.row.role,
            faces: &rendered.faces,
            media: &rendered.media,
            target,
        }
    }

    fn install(self, builder: &mut GlyphMatrixBuilder) {
        for face in self.faces {
            builder.install_frame_state(MatrixFrameStateInstallRequest::Face {
                id: face.id,
                face: face.clone(),
            });
        }
        for media in self.media {
            match self.target {
                RenderedDisplayRowAssetInstallTarget::MatrixRow(matrix_row) => {
                    media.install(builder, self.role, matrix_row);
                }
                RenderedDisplayRowAssetInstallTarget::WindowRow { row_index, bounds } => {
                    media.install_window_row(builder, self.role, row_index, bounds);
                }
                RenderedDisplayRowAssetInstallTarget::FrameChrome { row_index, bounds } => {
                    media.install_frame_chrome(builder, self.role, row_index, bounds);
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_current_row(
    builder: &mut GlyphMatrixBuilder,
    rendered: &RenderedDisplayRow,
    matrix_row: usize,
) -> DisplayRowPosition {
    for face in &rendered.faces {
        builder.install_frame_state(MatrixFrameStateInstallRequest::Face {
            id: face.id,
            face: face.clone(),
        });
    }
    builder.with_current_row_mut(|row| {
        row.enabled = true;
        row.role = rendered.row.role;
        row.mode_line = matches!(rendered.row.role, GlyphRowRole::ModeLine);
        row.displays_text |=
            rendered.row.displays_text || !display_row_text_is_empty(&rendered.row);
        row.glyphs[GlyphArea::Text.index()]
            .extend(rendered.row.glyphs[GlyphArea::Text.index()].iter().cloned());
        row.height_px = row.height_px.max(rendered.row.height_px);
        row.ascent_px = row
            .ascent_px
            .max(rendered.row.ascent_px)
            .min(row.height_px.max(1.0));
        merge_display_row_source_slot_bounds(row, &rendered.source_slots);
    });
    for media in &rendered.media {
        media.install(builder, rendered.row.role, matrix_row);
    }
    display_row_output_end_position(rendered.progress)
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    rendered: &RenderedDisplayRow,
    output: TextRowOutput,
) -> DisplayRowPosition {
    let end = append_rendered_display_row_fragment_to_current_row(builder, rendered, output.row);
    output_emitter.emit_text_source_slots(evaluator, output, &rendered.source_slots, end);
    end
}

impl RenderedDisplayRowMedia {
    fn install(&self, builder: &mut GlyphMatrixBuilder, role: GlyphRowRole, matrix_row: usize) {
        let context = builder.current_window_media_install_context();
        let row = matrix_row.min(u32::MAX as usize) as u32;
        let target = ResolvedMatrixMediaInstallTarget {
            window_id: context.window_id,
            role,
            clip: Some(context.text_pixel_bounds),
            slot_id: DisplaySlotId {
                window_id: context.window_id,
                row,
                col: self.col,
            },
        };
        self.install_with_target(builder, target);
    }

    fn install_window_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        role: GlyphRowRole,
        row: u32,
        clip: Rect,
    ) {
        let context = builder.current_window_media_install_context();
        let target = ResolvedMatrixMediaInstallTarget {
            window_id: context.window_id,
            role,
            clip: Some(clip),
            slot_id: DisplaySlotId {
                window_id: context.window_id,
                row,
                col: self.col,
            },
        };
        self.install_with_target(builder, target);
    }

    fn install_frame_chrome(
        &self,
        builder: &mut GlyphMatrixBuilder,
        role: GlyphRowRole,
        row: u32,
        clip: Rect,
    ) {
        let target = ResolvedMatrixMediaInstallTarget {
            window_id: FRAME_CHROME_WINDOW_ID,
            role,
            clip: Some(clip),
            slot_id: DisplaySlotId {
                window_id: FRAME_CHROME_WINDOW_ID,
                row,
                col: self.col,
            },
        };
        self.install_with_target(builder, target);
    }

    fn install_with_target(
        &self,
        builder: &mut GlyphMatrixBuilder,
        target: ResolvedMatrixMediaInstallTarget,
    ) {
        builder.install_media(MatrixMediaInstallRequest::new(
            target,
            self.matrix_media_kind(),
            self.x,
            self.y,
            self.width,
            self.height,
        ));
    }

    fn matrix_media_kind(&self) -> MatrixMediaInstallKind {
        match self.kind {
            RenderedDisplayRowMediaKind::Image { image_id } => {
                MatrixMediaInstallKind::Image { image_id }
            }
            RenderedDisplayRowMediaKind::Video {
                video_id,
                loop_count,
                autoplay,
            } => MatrixMediaInstallKind::Video {
                video_id,
                loop_count,
                autoplay,
            },
            RenderedDisplayRowMediaKind::Xwidget { xwidget_id } => {
                MatrixMediaInstallKind::Xwidget { xwidget_id }
            }
        }
    }
}

fn rendered_row_with_source_bounds(rendered: &RenderedDisplayRow) -> GlyphRow {
    let mut row = rendered.row.clone();
    apply_display_row_source_slot_bounds(&mut row, &rendered.source_slots);
    row
}

fn display_row_output_end_position(progress: DisplayRowOutputProgress) -> DisplayRowPosition {
    DisplayRowPosition {
        x_px: progress.end_x,
        col: usize::try_from(progress.end_col.max(0)).unwrap_or(usize::MAX),
    }
}

fn display_row_progress(end: DisplayRowPosition, y: f32, height: f32) -> DisplayRowOutputProgress {
    DisplayRowOutputProgress {
        end_x: end.x_px.max(0.0),
        end_col: end.col.min(i64::MAX as usize) as i64,
        y,
        height,
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
    let emitted_chars = progress.slots.len();
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
    let glyph_ascent = face.font_ascent.max(0.0);
    let glyph_height = (glyph_ascent + face.font_descent.max(0) as f32).max(1.0);
    let glyph_descent = (glyph_height - glyph_ascent).max(0.0);
    let row_descent = (layout.height_px - layout.ascent_px).max(0.0);
    layout.ascent_px = layout
        .ascent_px
        .max(glyph_ascent)
        .min(glyph_height.max(layout.height_px));
    layout.height_px = (layout.ascent_px + row_descent.max(glyph_descent)).max(glyph_height);
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
        (progress.status == DisplayRowAppendStatus::Complete && progress.metrics.width_px > 0.0)
            .then(|| descriptor.rendered_media(progress.start, y))
    }

    fn clipped_remainder(self, progress: &DisplayRowAppendProgress) -> Option<DisplayItem> {
        clipped_display_item_remainder(self.source_item, progress)
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
        fallback: DisplaySourceFallbackMetrics,
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

    pub(crate) fn render_lisp_string_request(
        &mut self,
        request: DisplayRowLispStringRenderRequest<'_>,
    ) -> Option<RenderedDisplayRow> {
        request.render_with_context(&mut self.renderer, &mut self.context)
    }

    pub(crate) fn render_item_source_fragment_into_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowItemSourceRenderRequest<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        request.render_fragment_step_into_row_with_policy(
            &mut self.renderer,
            row,
            source,
            source_state,
            &mut self.context,
            &mut NaturalDisplayRowRenderPolicy,
        )
    }
}

pub(crate) struct DisplayRowCurrentSourceFragmentRenderState<'face, 'emit> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) font_metrics: &'emit mut Option<FontMetricsService>,
    pub(crate) face_resolver: &'face FaceResolver,
    pub(crate) display_host: Option<&'emit dyn DisplayHost>,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

impl<'a> DisplayRowItemSourceRenderRequest<'a> {
    pub(crate) fn render_natural_fragment_into_current_row<S: DisplayItemSource>(
        self,
        state: &mut DisplayRowCurrentSourceFragmentRenderState<'_, '_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let DisplayRowCurrentSourceFragmentRenderState {
            builder,
            font_metrics,
            face_resolver,
            display_host,
            face_ids,
        } = state;
        let mut render_executor =
            DisplayRowRenderExecutor::new(font_metrics, face_resolver, *display_host, face_ids);
        let result = builder
            .with_current_row_mut(|row| {
                render_executor.render_item_source_fragment_into_row(
                    self,
                    row,
                    source,
                    source_state,
                )
            })
            .flatten()?;
        DisplayRowCurrentSourceSlotBoundsMergeRequest::new(&result.source_slots).install(builder);
        Some(result)
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
            .map(|result| result.rendered)
    }

    fn render_lisp_string_request_with_context(
        &mut self,
        request: DisplayRowLispStringRenderRequest<'_>,
        context: &mut DisplayRowRenderContext<'_, '_>,
    ) -> Option<RenderedDisplayRow> {
        let (plan, session_request) = request.into_render_parts();
        self.render_lisp_string_plan_with_context(plan, session_request, context)
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
        glyph_row_writer::normalize_external_row(&mut result.rendered.row);
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
            geometry.char_width,
            geometry.ascent,
            geometry.height,
        );
        let char_width = face_realizer
            .char_width(&row_face, geometry.char_width)
            .max(1.0);
        let mut row_faces = vec![row_face.clone()];

        let parsed_symbol_values = symbol_values
            .into_iter()
            .filter_map(|(name, value)| parse_display_length_expr(value).map(|expr| (name, expr)))
            .collect();
        let row_ascent = row_face
            .font_ascent
            .max(geometry.ascent)
            .min(geometry.height.max(1.0));
        let mut row_layout = geometry.to_layout(
            role,
            char_width,
            row_ascent,
            RenderFaceRef::FaceId(row_face.face_id),
            parsed_symbol_values,
        );
        let mut position = render_bounds.start;
        let mut source_slots = Vec::new();
        let mut media = Vec::new();
        let fallback_metrics =
            DisplaySourceFallbackMetrics::new(char_width, geometry.ascent, geometry.height);
        let stop = loop {
            let params =
                context.source_resolve_params(row_face.face_id, base_face, fallback_metrics);
            let resolved = state.next_resolved_item(source, params, context.face_ids());
            let item = resolved.item;
            for pending in resolved.pending_faces {
                let row_face = face_realizer.realize_face(
                    pending.face_id,
                    &pending.resolved,
                    char_width,
                    geometry.ascent,
                    geometry.height,
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
                        geometry.ascent,
                        geometry.height,
                    );
                    include_display_row_face_metrics(&mut row_layout, &realized);
                    row_faces.push(realized);
                }
            }
            let render_item = DisplayRowRenderItem::from_source_item(item);
            let item_face_id = match render_item.row_face() {
                RenderFaceRef::FaceId(face_id) => face_id,
                RenderFaceRef::Inherit => row_face.face_id,
            };
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
                        render_bounds.max_x.to_f32(),
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
                            render_bounds.max_x.to_f32(),
                            area,
                        );
                    row_writer.push_item(render_item.row_item_for_write())
                }
            };
            position = progress.end;
            source_slots.extend(progress.slots.iter().cloned());
            if let Some(rendered) =
                render_item.rendered_media_for_progress(&progress, row_layout.y_px)
            {
                media.push(rendered);
            }
            match progress.status {
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
        let progress = display_row_progress(position, geometry.y, progress_height);
        let faces = row_faces
            .into_iter()
            .map(|face| face.render_face())
            .collect();
        Some(DisplayRowRenderIntoRowResult {
            progress,
            source_slots,
            faces,
            media,
            stop,
        })
    }
}

#[cfg(test)]
#[path = "display_row_test.rs"]
mod tests;
