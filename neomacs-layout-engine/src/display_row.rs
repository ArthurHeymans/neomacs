use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayLengthExpr, DisplayMediaReplacement,
    DisplayMediaReplacementKind, DisplaySourceMappedText, DisplaySourcePosition, DisplayTextRun,
    RenderFaceRef, SourceSpan,
};
use crate::display_property::parse_display_length_expr;
use crate::display_row_builder::{
    DisplayGlyphMeasurer, DisplayRowAppendStatus, DisplayRowGlyphSlot, DisplayRowItemMeasurement,
    DisplayRowLayout, DisplayRowPosition, DisplayRowProgressWriter, DisplayTabPolicy,
};
use crate::display_source::{DisplayItemSource, LispStringSourceCursor};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_source_resolver::{
    DisplaySourceResolveParams, DisplaySourceResolveState, ResolvedDisplaySourceItem,
    resolve_next_display_source_item,
};
use crate::display_text::{DisplayTextFragment, DisplayTextStorage};
use crate::display_text_run_measurement::{
    DisplayTextRunMeasurement, DisplayTextRunMeasurementPlan,
};
use crate::engine::LayoutEngine;
use crate::font_metrics::{FontMetrics, FontMetricsService};
use crate::glyph_advance::GlyphAdvanceQuantization;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::FaceResolver;
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::face::{BoxType, Face, FaceAttributes, UnderlineStyle};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
#[cfg(test)]
use neomacs_display_protocol::glyph_matrix::GlyphArea;
use neomacs_display_protocol::glyph_matrix::{FrameChromeRow, GlyphRow};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{CharPos0, EmacsBytePos};
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

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowGlyphMeasurementFace {
    face: DisplayRowFace,
    use_font_metrics: bool,
    fallback_char_width: f32,
    quantization: GlyphAdvanceQuantization,
}

impl DisplayRowGlyphMeasurementFace {
    pub(crate) fn new(
        face: DisplayRowFace,
        use_font_metrics: bool,
        fallback_char_width: f32,
        quantization: GlyphAdvanceQuantization,
    ) -> Self {
        Self {
            face,
            use_font_metrics,
            fallback_char_width,
            quantization,
        }
    }

    pub(crate) fn from_resolved(
        face_id: u32,
        face: &ResolvedFace,
        metrics: Option<FontMetrics>,
        use_font_metrics: bool,
        fallback_char_width: f32,
    ) -> Self {
        let quantization = if use_font_metrics {
            GlyphAdvanceQuantization::PreserveLogicalPixels
        } else {
            GlyphAdvanceQuantization::SnapToIntegerPixels
        };
        Self::new(
            resolved_display_row_face(face_id, face, metrics),
            use_font_metrics,
            fallback_char_width,
            quantization,
        )
    }

    pub(crate) fn glyph_advance_px(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        columns: u8,
        fallback_advance_px: f32,
    ) -> f32 {
        let faces = [self.face.clone()];
        let font_metrics = if self.use_font_metrics {
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
        if !self.use_font_metrics {
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

    pub(crate) fn text_run_measurement(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        text: &str,
    ) -> DisplayTextRunMeasurement {
        let measurement = self.shaped_text_run_measurement(font_metrics, text);
        if measurement.measured_advances().is_some() {
            return measurement;
        }
        DisplayTextRunMeasurementPlan::from_char_advances(
            text,
            self.fallback_char_width,
            |ch, fallback_advance_px| self.advance_for_char(font_metrics, ch, fallback_advance_px),
        )
    }

    pub(crate) fn resolved_fragment_measurement(
        &self,
        text: &str,
        advance_px: f32,
    ) -> DisplayTextRunMeasurement {
        DisplayTextRunMeasurementPlan::from_resolved_fragment_advance(text, advance_px)
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
        next_face_id: &mut u32,
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
            resolve_next_display_source_item(source, params, &mut self.resolve_state, next_face_id);
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
        next_face_id: &mut u32,
        display_host: Option<&dyn DisplayHost>,
        fallback_char_width: f32,
        fallback_ascent: f32,
        fallback_row_height: f32,
    ) -> Option<DisplayRowSourceStep> {
        let resolved = self.state.next_resolved_item(
            &mut self.source,
            DisplaySourceResolveParams {
                face_resolver,
                display_host,
                base_face,
                canonical_face: face_resolver.default_face(),
                base_face_id,
                fallback_char_width,
                fallback_ascent,
                fallback_row_height,
            },
            next_face_id,
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

    fn measurement_for(&mut self, _item: &DisplayItem, _face_id: u32) -> DisplayRowItemMeasurement {
        DisplayRowItemMeasurement::Default
    }

    fn clipped_behavior(&mut self, _item: &DisplayItem) -> DisplayRowRenderClipBehavior {
        DisplayRowRenderClipBehavior::PreserveRemainderAndStop
    }
}

struct NaturalDisplayRowRenderPolicy;

impl DisplayRowRenderPolicy for NaturalDisplayRowRenderPolicy {}

pub(crate) struct DisplayRowRenderResult {
    pub(crate) rendered: RenderedDisplayRow,
    pub(crate) stop: DisplayRowRenderStop,
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
            stop: self.stop,
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

pub(crate) struct DisplayRowSpec<'a> {
    pub(crate) geometry: DisplayRowGeometry,
    pub(crate) render_bounds: DisplayRowRenderBounds,
    pub(crate) base_face_id: u32,
    pub(crate) base_face: &'a ResolvedFace,
    pub(crate) role: GlyphRowRole,
    pub(crate) symbol_values: std::collections::HashMap<String, Value>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowRenderBounds {
    pub(crate) start: DisplayRowPosition,
    pub(crate) max_x_px: f32,
}

impl DisplayRowRenderBounds {
    pub(crate) fn whole_row(width_px: f32) -> Self {
        Self {
            start: DisplayRowPosition { x_px: 0.0, col: 0 },
            max_x_px: width_px.max(0.0),
        }
    }
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
            render_bounds: DisplayRowRenderBounds::whole_row(geometry.width),
            geometry,
            base_face_id,
            base_face,
            role,
            symbol_values,
        }
    }
}

pub(crate) fn install_rendered_display_row(
    builder: &mut GlyphMatrixBuilder,
    rendered: &RenderedDisplayRow,
    matrix_row: usize,
) {
    for face in &rendered.faces {
        builder.insert_face(face.id, face.clone());
    }
    builder.begin_row(matrix_row, rendered.row.role);
    let row = rendered_row_with_source_bounds(rendered);
    builder.install_prebuilt_current_row(&row);
    builder.end_prebuilt_row();
    for media in &rendered.media {
        media.install(builder, rendered.row.role, matrix_row);
    }
}

pub(crate) fn install_measured_window_display_row(
    builder: &mut GlyphMatrixBuilder,
    measured: &MeasuredDisplayRow,
) {
    let DisplayRowOwner::WindowChrome { window_id, kind } = measured.owner else {
        panic!("frame chrome rows must use install_measured_frame_chrome_row");
    };
    debug_assert!(window_id > 0);
    debug_assert!(matches!(
        kind,
        WindowChromeKind::TabLine | WindowChromeKind::HeaderLine | WindowChromeKind::ModeLine
    ));
    for face in &measured.rendered.faces {
        builder.insert_face(face.id, face.clone());
    }
    let matrix_row = measured.row_index as usize;
    builder.begin_row(matrix_row, measured.rendered.row.role);
    let mut row = rendered_row_with_source_bounds(&measured.rendered);
    row.pixel_y = measured.bounds.y;
    row.height_px = measured.row_height();
    row.ascent_px = measured.row_ascent();
    builder.install_prebuilt_current_row(&row);
    builder.end_prebuilt_row();
    for media in &measured.rendered.media {
        media.install_window_row(
            builder,
            measured.rendered.row.role,
            measured.row_index,
            measured.bounds,
        );
    }
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
    for face in &measured.rendered.faces {
        builder.insert_face(face.id, face.clone());
    }
    for media in &measured.rendered.media {
        media.install_frame_chrome(
            builder,
            measured.rendered.row.role,
            measured.row_index,
            measured.bounds,
        );
    }
    let mut row = measured.rendered.row.clone();
    apply_source_slot_bounds_to_row(&mut row, &measured.rendered.source_slots);
    row.pixel_y = measured.bounds.y;
    row.height_px = measured.row_height();
    row.ascent_px = measured.row_ascent();
    frame_chrome_rows.push(FrameChromeRow {
        row_index: measured.row_index,
        pixel_bounds: measured.bounds,
        row,
    });
}

pub(crate) fn install_rendered_display_row_fragment_assets(
    builder: &mut GlyphMatrixBuilder,
    role: GlyphRowRole,
    matrix_row: usize,
    faces: &[Face],
    media: &[RenderedDisplayRowMedia],
) {
    for face in faces {
        builder.insert_face(face.id, face.clone());
    }
    for media in media {
        media.install(builder, role, matrix_row);
    }
}

pub(crate) fn merge_display_row_source_slot_bounds_to_current_row(
    builder: &mut GlyphMatrixBuilder,
    slots: &[DisplayRowGlyphSlot],
) {
    let Some((start, end)) = buffer_source_slot_bounds(slots) else {
        return;
    };
    builder.with_current_row_mut(|row| {
        merge_row_buffer_source_bounds(row, start, end);
    });
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_current_row(
    builder: &mut GlyphMatrixBuilder,
    rendered: &RenderedDisplayRow,
    matrix_row: usize,
) -> DisplayRowPosition {
    for face in &rendered.faces {
        builder.insert_face(face.id, face.clone());
    }
    let source_bounds = buffer_source_slot_bounds(&rendered.source_slots);
    builder.with_current_row_mut(|row| {
        row.enabled = true;
        row.role = rendered.row.role;
        row.mode_line = matches!(rendered.row.role, GlyphRowRole::ModeLine);
        row.displays_text |=
            rendered.row.displays_text || !rendered.row.glyphs[GlyphArea::Text.index()].is_empty();
        row.glyphs[GlyphArea::Text.index()]
            .extend(rendered.row.glyphs[GlyphArea::Text.index()].iter().cloned());
        row.height_px = row.height_px.max(rendered.row.height_px);
        row.ascent_px = row
            .ascent_px
            .max(rendered.row.ascent_px)
            .min(row.height_px.max(1.0));
        if let Some((start, end)) = source_bounds {
            merge_row_buffer_source_bounds(row, start, end);
        }
    });
    for media in &rendered.media {
        media.install(builder, rendered.row.role, matrix_row);
    }
    display_row_output_end_position(rendered.progress)
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

    fn install_window_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        role: GlyphRowRole,
        row: u32,
        clip: Rect,
    ) {
        match self.kind {
            RenderedDisplayRowMediaKind::Image { image_id } => builder
                .push_current_window_image_with_clip(
                    role,
                    row,
                    self.col,
                    clip,
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
            } => builder.push_current_window_video_with_clip(
                role,
                row,
                self.col,
                clip,
                video_id,
                self.x,
                self.y,
                self.width,
                self.height,
                loop_count,
                autoplay,
            ),
            RenderedDisplayRowMediaKind::Xwidget { xwidget_id } => builder
                .push_current_window_xwidget_with_clip(
                    role,
                    row,
                    self.col,
                    clip,
                    xwidget_id,
                    self.x,
                    self.y,
                    self.width,
                    self.height,
                ),
        }
    }

    fn install_frame_chrome(
        &self,
        builder: &mut GlyphMatrixBuilder,
        role: GlyphRowRole,
        row: u32,
        clip: Rect,
    ) {
        match self.kind {
            RenderedDisplayRowMediaKind::Image { image_id } => builder.push_frame_chrome_image(
                role,
                row,
                self.col,
                clip,
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
            } => builder.push_frame_chrome_video(
                role,
                row,
                self.col,
                clip,
                video_id,
                self.x,
                self.y,
                self.width,
                self.height,
                loop_count,
                autoplay,
            ),
            RenderedDisplayRowMediaKind::Xwidget { xwidget_id } => builder
                .push_frame_chrome_xwidget(
                    role,
                    row,
                    self.col,
                    clip,
                    xwidget_id,
                    self.x,
                    self.y,
                    self.width,
                    self.height,
                ),
        }
    }
}

fn rendered_row_with_source_bounds(rendered: &RenderedDisplayRow) -> GlyphRow {
    let mut row = rendered.row.clone();
    apply_source_slot_bounds_to_row(&mut row, &rendered.source_slots);
    row
}

fn apply_source_slot_bounds_to_row(row: &mut GlyphRow, slots: &[DisplayRowGlyphSlot]) {
    let Some((start, end)) = buffer_source_slot_bounds(slots) else {
        return;
    };
    set_row_buffer_source_bounds(row, start, end);
}

fn buffer_source_slot_bounds(slots: &[DisplayRowGlyphSlot]) -> Option<(usize, usize)> {
    slots.iter().fold(None::<(usize, usize)>, |bounds, slot| {
        let DisplaySourcePosition::Buffer { char_pos, .. } = slot.source else {
            return bounds;
        };
        let start = char_pos.get();
        let end = start.saturating_add(1);
        Some(match bounds {
            Some((old_start, old_end)) => (old_start.min(start), old_end.max(end)),
            None => (start, end),
        })
    })
}

fn merge_row_buffer_source_bounds(row: &mut GlyphRow, start: usize, end: usize) {
    if row.start_charpos == row.end_charpos {
        set_row_buffer_source_bounds(row, start, end);
        return;
    }
    set_row_buffer_source_bounds(row, row.start_charpos.min(start), row.end_charpos.max(end));
}

fn set_row_buffer_source_bounds(row: &mut GlyphRow, start: usize, end: usize) {
    row.start_charpos = start;
    row.end_charpos = end;
}

#[cfg(test)]
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
    pub(crate) fn render_lisp_string_row(
        &mut self,
        spec: DisplayRowSpec<'_>,
        rendered: Value,
        face_resolver: &FaceResolver,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplayRow> {
        self.render_lisp_string_row_with_display_host(
            spec,
            rendered,
            face_resolver,
            None,
            next_face_id,
        )
    }

    pub(crate) fn render_lisp_string_row_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        rendered: Value,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplayRow> {
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
    pub(crate) fn render_display_text_fragment_row(
        &mut self,
        spec: DisplayRowSpec<'_>,
        fragment: DisplayTextFragment,
        face_resolver: &FaceResolver,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplayRow> {
        self.render_display_text_fragment_row_with_display_host(
            spec,
            fragment,
            face_resolver,
            None,
            next_face_id,
        )
    }

    pub(crate) fn render_display_text_fragment_row_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        fragment: DisplayTextFragment,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplayRow> {
        let rendered = match fragment.storage {
            DisplayTextStorage::LispString(value) => value,
            DisplayTextStorage::Static(value) => Value::string(value),
            DisplayTextStorage::BufferSpan { .. } => return None,
        };
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
    ) -> Option<RenderedDisplayRow> {
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
    ) -> Option<RenderedDisplayRow> {
        let mut state = DisplayRowSourceState::default();
        self.render_display_item_source_row_step_with_display_host(
            spec,
            source,
            &mut state,
            face_resolver,
            display_host,
            next_face_id,
        )
        .map(|result| result.rendered)
    }

    pub(crate) fn render_display_item_source_row_step_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<DisplayRowRenderResult> {
        let mut result = self.render_display_item_source_row_fragment_step_with_display_host(
            spec,
            source,
            state,
            face_resolver,
            display_host,
            next_face_id,
        )?;
        GlyphMatrixBuilder::normalize_external_row(&mut result.rendered.row);
        Some(result)
    }

    pub(crate) fn render_display_item_source_row_fragment_step_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<DisplayRowRenderResult> {
        let mut row = GlyphRow::new(spec.role);
        let result = self.render_display_item_source_row_fragment_step_into_row_with_display_host(
            spec,
            &mut row,
            source,
            state,
            face_resolver,
            display_host,
            next_face_id,
        )?;
        Some(result.with_row(row))
    }

    pub(crate) fn render_display_item_source_row_fragment_step_into_row_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        row: &mut GlyphRow,
        source: &mut impl DisplayItemSource,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let mut policy = NaturalDisplayRowRenderPolicy;
        self.render_display_item_source_row_fragment_step_into_row_with_policy(
            spec,
            row,
            source,
            state,
            face_resolver,
            display_host,
            next_face_id,
            &mut policy,
        )
    }

    pub(crate) fn render_display_item_source_row_fragment_step_into_row_with_policy<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        spec: DisplayRowSpec<'_>,
        row: &mut GlyphRow,
        source: &mut S,
        state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
        policy: &mut P,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        if state.is_finished() {
            return None;
        }

        let DisplayRowSpec {
            geometry,
            render_bounds,
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
        let stop = loop {
            let resolved = state.next_resolved_item(
                source,
                DisplaySourceResolveParams {
                    face_resolver,
                    display_host,
                    base_face,
                    canonical_face: face_resolver.default_face(),
                    base_face_id: row_face.face_id,
                    fallback_char_width: char_width,
                    fallback_ascent: geometry.ascent,
                    fallback_row_height: geometry.height,
                },
                next_face_id,
            );
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
            let media_descriptor = DisplayMediaReplacement::from_item_kind(&item.kind);
            let source_item = item.clone();
            let item = media_descriptor
                .map(|descriptor| descriptor.replacement_item(item.clone()))
                .unwrap_or(item);
            let item_face_id = match item.face {
                RenderFaceRef::FaceId(face_id) => face_id,
                RenderFaceRef::Inherit => row_face.face_id,
            };
            let measurement = policy.measurement_for(&item, item_face_id);
            let progress = match measurement {
                DisplayRowItemMeasurement::Default => {
                    let mut glyph_measurer = DisplayRowGlyphMeasurer::new(
                        &row_faces,
                        face_realizer.font_metrics_service_mut(),
                        char_width,
                    );
                    let mut row_writer = DisplayRowProgressWriter::with_glyph_measurer(
                        &row_layout,
                        &mut *row,
                        &mut glyph_measurer,
                        position,
                        render_bounds.max_x_px,
                    );
                    row_writer.push_item(item)
                }
                DisplayRowItemMeasurement::TextRun(measurement) => {
                    let mut row_writer = DisplayRowProgressWriter::with_text_run_measurement(
                        &row_layout,
                        &mut *row,
                        measurement,
                        position,
                        render_bounds.max_x_px,
                    );
                    row_writer.push_item(item)
                }
            };
            position = progress.end;
            source_slots.extend(progress.slots.iter().cloned());
            if let Some(descriptor) = media_descriptor
                && progress.status == DisplayRowAppendStatus::Complete
                && progress.metrics.width_px > 0.0
            {
                media.push(descriptor.rendered_media(progress.start, row_layout.y_px));
            }
            match progress.status {
                DisplayRowAppendStatus::Complete => {}
                DisplayRowAppendStatus::Clipped => match policy.clipped_behavior(&source_item) {
                    DisplayRowRenderClipBehavior::PreserveRemainderAndStop => {
                        state.remember_pending_item(clipped_display_item_remainder(
                            source_item,
                            &progress,
                        ));
                        break DisplayRowRenderStop::Clipped;
                    }
                    DisplayRowRenderClipBehavior::Stop => {
                        break DisplayRowRenderStop::Clipped;
                    }
                    DisplayRowRenderClipBehavior::Continue => {}
                },
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

impl LayoutEngine {
    #[cfg(test)]
    pub(crate) fn render_lisp_string_row(
        &mut self,
        spec: DisplayRowSpec<'_>,
        rendered: Value,
        face_resolver: &FaceResolver,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplayRow> {
        self.render_lisp_string_row_with_display_host(
            spec,
            rendered,
            face_resolver,
            None,
            next_face_id,
        )
    }

    pub(crate) fn render_lisp_string_row_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        rendered: Value,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplayRow> {
        DisplayRowRenderer::new(&mut self.font_metrics).render_lisp_string_row_with_display_host(
            spec,
            rendered,
            face_resolver,
            display_host,
            next_face_id,
        )
    }

    pub(crate) fn render_display_text_fragment_row_with_display_host(
        &mut self,
        spec: DisplayRowSpec<'_>,
        fragment: DisplayTextFragment,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        next_face_id: &mut u32,
    ) -> Option<RenderedDisplayRow> {
        DisplayRowRenderer::new(&mut self.font_metrics)
            .render_display_text_fragment_row_with_display_host(
                spec,
                fragment,
                face_resolver,
                display_host,
                next_face_id,
            )
    }
}

#[cfg(test)]
#[path = "display_row_test.rs"]
mod tests;
