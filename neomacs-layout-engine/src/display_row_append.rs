use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayMediaReplacement,
    DisplaySourceMappedText, DisplaySourcePosition, DisplayTextRun, GlyphlessJoinerPolicy,
    GlyphlessMethod, RenderFaceRef, SourceSpan, glyphless_method_for_char,
};
use crate::display_origin::{DisplayOrigin, DisplayPropertySource};
use crate::display_property::{DisplayMediaReplacementProperty, DisplayPropertyClassification};
#[cfg(test)]
use crate::display_row::DisplayRowRenderStop;
#[cfg(test)]
use crate::display_row::RenderedDisplayRow;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_current_row;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy,
    DisplayRowGeometry, DisplayRowMeasuredFaceMetrics, DisplayRowRenderBounds,
    DisplayRowRenderClipBehavior, DisplayRowRenderPolicy, DisplayRowSourceAppendRequest,
    DisplayRowSourceGeometry, DisplayRowSourceState, NaturalDisplayRowAppendRenderPolicy,
    measure_display_source_append_request_against_current_text_row,
    render_display_source_append_request_into_current_text_row_and_emit,
    render_natural_display_source_append_request_into_current_text_row_and_emit,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowItemMeasurement,
    DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_source::{
    BufferDisplayReplacementSource, BufferDisplayReplacementStringSource, BufferTextItemSource,
    DisplayItemSource, DisplayReplacementBox, DisplaySourceContext, LispStringSourceCursor,
};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_source_resolver::{ResolvedDisplayReplacement, resolve_display_replacement};
use crate::display_space::{DisplaySpaceKey, display_space_positive_number};
use crate::display_text_run_measurement::{
    ComplexTextRunAdvanceResolver, DisplayTextRunMeasurementPlan,
};
use crate::font_metrics::FontMetricsService;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use crate::types::WindowParams;
use crate::unicode::decode_utf8;
use crate::window_output::{TextRowOutput, WindowOutputEmitter};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::buffer::{BufferId, CharLen, CharPos0, EmacsByteRange};
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::{Context, Value};

struct SingleDisplayItemSource {
    source_position: DisplaySourcePosition,
    item: Option<DisplayItem>,
}

impl SingleDisplayItemSource {
    fn new(item: DisplayItem) -> Self {
        Self {
            source_position: item.span.start.clone(),
            item: Some(item),
        }
    }
}

impl DisplayItemSource for SingleDisplayItemSource {
    fn next_item(&mut self, _context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem> {
        self.item.take()
    }

    fn source_position(&self) -> DisplaySourcePosition {
        self.source_position.clone()
    }
}

struct ResolvedSourceAdvanceRenderPolicy {
    advance_px: f32,
}

impl ResolvedSourceAdvanceRenderPolicy {
    fn new(advance_px: f32) -> Self {
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum BufferTextSourceAppendMeasurement {
    Natural,
    ResolvedAdvance { advance_px: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedBufferTextSourceAdvance {
    advance_px: f32,
    append_measurement: BufferTextSourceAppendMeasurement,
}

impl ResolvedBufferTextSourceAdvance {
    fn natural(advance_px: f32) -> Self {
        Self {
            advance_px,
            append_measurement: BufferTextSourceAppendMeasurement::Natural,
        }
    }

    fn resolved(advance_px: f32) -> Self {
        Self {
            advance_px,
            append_measurement: BufferTextSourceAppendMeasurement::ResolvedAdvance { advance_px },
        }
    }

    pub(crate) fn advance_px(self) -> f32 {
        self.advance_px
    }

    fn append_measurement(self) -> BufferTextSourceAppendMeasurement {
        self.append_measurement
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BufferTextSourceAdvanceResolver {
    complex_run: ComplexTextRunAdvanceResolver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextSourceAdvancePath {
    NaturalRenderedSource,
    ResolvedComplexRun,
}

impl BufferTextSourceAdvancePath {
    fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Self {
        if crate::composition::needs_complex_shaping(cluster.ch()) {
            Self::ResolvedComplexRun
        } else {
            Self::NaturalRenderedSource
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextSourceNaturalFallbackAdvance {
    Tab,
    ClusterContinuation,
    FaceColumns { columns: usize },
}

impl BufferTextSourceNaturalFallbackAdvance {
    fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Self {
        let ch = cluster.ch();
        if ch == '\t' {
            Self::Tab
        } else if cluster.is_cluster_continuation() {
            Self::ClusterContinuation
        } else {
            Self::FaceColumns {
                columns: crate::composition::base_width_cols(ch) as usize,
            }
        }
    }

    fn resolve_to_text_row(
        self,
        font_metrics: &mut Option<FontMetricsService>,
        active_face_state: &DisplayRowActiveFaceState,
        frame: &DisplayRowAppendFrame,
        position: DisplayRowPosition,
        ch: char,
    ) -> f32 {
        match self {
            Self::Tab => {
                frame
                    .geometry
                    .tab_policy
                    .advance_from(position, frame.face_space_width)
                    .pixel_width
            }
            Self::ClusterContinuation => 0.0,
            Self::FaceColumns { columns } => {
                active_face_state.advance_for_columns(font_metrics, ch, columns)
            }
        }
    }
}

enum BufferTextSourceRenderPolicy {
    Natural(NaturalDisplayRowAppendRenderPolicy),
    Resolved(ResolvedSourceAdvanceRenderPolicy),
}

impl BufferTextSourceRenderPolicy {
    fn new(measurement: BufferTextSourceAppendMeasurement) -> Self {
        match measurement {
            BufferTextSourceAppendMeasurement::Natural => {
                Self::Natural(NaturalDisplayRowAppendRenderPolicy)
            }
            BufferTextSourceAppendMeasurement::ResolvedAdvance { advance_px } => {
                Self::Resolved(ResolvedSourceAdvanceRenderPolicy::new(advance_px))
            }
        }
    }
}

impl DisplayRowRenderPolicy for BufferTextSourceRenderPolicy {
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

#[allow(clippy::too_many_arguments)]
fn append_single_display_item_fragment_to_text_row_and_emit<P: DisplayRowRenderPolicy>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    mut item: DisplayItem,
    face_resolver: &FaceResolver,
    request: DisplayRowSourceAppendRequest<'_>,
    render_policy: &mut P,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let base_face_id = request.base_face_id();
    item.face = RenderFaceRef::FaceId(base_face_id);
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(base_face_id.saturating_add(1));
    let start = request.start_position();
    let outcome = render_display_source_append_request_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        &mut face_ids,
        request,
        render_policy,
    )?;
    Some(outcome.into_append_progress_and_position(start))
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
fn append_lisp_string_value_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    text_value: Value,
    source_id: u64,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> DisplayRowPosition {
    let Some(mut source) = crate::display_source::LispStringSourceCursor::new(
        source_id,
        text_value,
        RenderFaceRef::FaceId(base_face_id),
    ) else {
        return position;
    };
    let request = frame.source_append_request(
        position,
        base_face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    let mut source_state = DisplayRowSourceState::default();
    let Some(outcome) = render_natural_display_source_append_request_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        face_ids,
        request,
    ) else {
        return position;
    };
    outcome.end_position()
}

#[cfg(test)]
#[derive(Clone)]
pub(crate) struct LispStringAppendContext<'a> {
    base_face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

#[cfg(test)]
impl<'a> LispStringAppendContext<'a> {
    pub(crate) fn new(
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            base_face_id,
            base_face,
            frame,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_value_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text_value: Value,
        source_id: u64,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        append_lisp_string_value_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            text_value,
            source_id,
            face_resolver,
            self.base_face,
            self.base_face_id,
            face_ids,
            self.frame.clone(),
            position,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct LispStringRowAppendContext<'row> {
    active_face_context: DisplayRowActiveFaceAppendContext<'row, 'row>,
}

impl<'row> LispStringRowAppendContext<'row> {
    pub(crate) fn new(
        append_surface: &'row DisplayRowAppendSurface,
        geometry: &'row DisplayRowGeometryState,
        active_face: &'row DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            active_face_context: DisplayRowActiveFaceAppendContext::new(
                append_surface,
                geometry,
                active_face,
                glyph_y_offset,
                default_row_height,
            ),
        }
    }

    #[cfg(test)]
    fn active_face<'face>(
        self,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> LispStringAppendContext<'face> {
        let frame = self.active_face_context.active_face_frame();
        LispStringAppendContext::new(base_face_id, base_face, frame)
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(crate) fn append_active_face_value_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text_value: Value,
        source_id: u64,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        base_face_id: u32,
        base_face: &'row ResolvedFace,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        self.active_face(base_face_id, base_face)
            .append_value_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                text_value,
                source_id,
                face_resolver,
                face_ids,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_active_face_source_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        source: &mut LispStringSourceCursor,
        source_state: &mut DisplayRowSourceState,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        base_face_id: u32,
        base_face: &'row ResolvedFace,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        let frame = self.active_face_context.active_face_frame();
        let mut source_context =
            LispStringSourceAppendContext::new(source, source_state, base_face_id, base_face);
        source_context
            .render_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                face_ids,
                frame,
                position,
            )
            .map(|outcome| outcome.end_position())
            .unwrap_or(position)
    }
}

#[allow(clippy::too_many_arguments)]
fn render_lisp_string_source_append_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    source: &mut LispStringSourceCursor,
    source_state: &mut DisplayRowSourceState,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<CurrentTextRowRenderOutcome> {
    let request = frame.source_append_request(
        position,
        base_face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    render_natural_display_source_append_request_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        source,
        source_state,
        face_resolver,
        face_ids,
        request,
    )
}

pub(crate) struct LispStringSourceAppendContext<'a> {
    source: &'a mut LispStringSourceCursor,
    source_state: &'a mut DisplayRowSourceState,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
}

impl<'a> LispStringSourceAppendContext<'a> {
    pub(crate) fn new(
        source: &'a mut LispStringSourceCursor,
        source_state: &'a mut DisplayRowSourceState,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> Self {
        Self {
            source,
            source_state,
            base_face_id,
            base_face,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        render_lisp_string_source_append_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            self.source,
            self.source_state,
            face_resolver,
            self.base_face,
            self.base_face_id,
            face_ids,
            frame,
            position,
        )
    }

    pub(crate) fn discard_pending_until_row_break(&mut self) -> bool {
        self.source_state.discard_pending_item();
        self.source.discard_until_row_break()
    }
}

pub(crate) struct LispStringSourceRowAppendContext<'a> {
    source_context: LispStringSourceAppendContext<'a>,
    append_surface: &'a DisplayRowAppendSurface,
    glyph_y_offset: f32,
    metrics: DisplayRowAppendMetrics,
}

impl<'a> LispStringSourceRowAppendContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source: &'a mut LispStringSourceCursor,
        source_state: &'a mut DisplayRowSourceState,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        append_surface: &'a DisplayRowAppendSurface,
        glyph_y_offset: f32,
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            source_context: LispStringSourceAppendContext::new(
                source,
                source_state,
                base_face_id,
                base_face,
            ),
            append_surface,
            glyph_y_offset,
            metrics: DisplayRowAppendMetrics::text_row(
                height,
                ascent,
                char_width,
                default_row_height,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        geometry: &DisplayRowGeometryState,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let frame = DisplayRowTextAppendContext::new(
            self.append_surface,
            geometry,
            self.glyph_y_offset,
            self.metrics.default_row_height,
        )
        .text_row_frame(
            self.metrics.height,
            self.metrics.ascent,
            self.metrics.char_width,
        );
        self.source_context.render_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            face_ids,
            frame,
            position,
        )
    }

    pub(crate) fn discard_pending_until_row_break(&mut self) -> bool {
        self.source_context.discard_pending_until_row_break()
    }
}

pub(crate) fn synthetic_display_text_item(
    source_id: u64,
    text: impl Into<Box<str>>,
    face_id: u32,
) -> DisplayItem {
    let text = text.into();
    let char_len = text.chars().count();
    DisplayItem::new(
        SourceSpan::synthetic(source_id, 0, char_len),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text)),
    )
}

#[derive(Clone)]
pub(crate) struct SyntheticTextAppendContext<'a> {
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> SyntheticTextAppendContext<'a> {
    pub(crate) fn new(
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            face_id,
            base_face,
            frame,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
        source_id: u64,
        text: impl Into<Box<str>>,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_synthetic_text_to_display_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            self.base_face,
            self.frame.clone(),
            position,
            source_id,
            text,
            self.face_id,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SyntheticTextRowAppendContext<'a> {
    active_face_context: DisplayRowActiveFaceAppendContext<'a, 'a>,
}

impl<'a> SyntheticTextRowAppendContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &'a DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            active_face_context: DisplayRowActiveFaceAppendContext::new(
                append_surface,
                geometry,
                active_face,
                glyph_y_offset,
                default_row_height,
            ),
        }
    }

    fn active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> SyntheticTextAppendContext<'a> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context.active_face_frame(),
        )
    }

    fn text_row(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> SyntheticTextAppendContext<'a> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context
                .text_row_frame(height_px, ascent_px, char_width_px),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_active_face_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
        source_id: u64,
        text: impl Into<Box<str>>,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let active_face = self.active_face_context.active_face;
        self.active_face(active_face.face_id(), active_face.resolved_face())
            .append_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                position,
                source_id,
                text,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_text_row_metrics_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
        source_id: u64,
        text: impl Into<Box<str>>,
        face_id: u32,
        base_face: &'a ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.text_row(face_id, base_face, height_px, ascent_px, char_width_px)
            .append_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                position,
                source_id,
                text,
            )
    }
}

pub(crate) fn render_face_ref_id(face: RenderFaceRef, fallback: u32) -> u32 {
    match face {
        RenderFaceRef::FaceId(face_id) => face_id,
        RenderFaceRef::Inherit => fallback,
    }
}

#[cfg(test)]
pub(crate) fn apply_pending_display_source_faces(
    builder: &mut GlyphMatrixBuilder,
    pending_faces: &mut Vec<PendingDisplaySourceFace>,
) {
    for pending in pending_faces.drain(..) {
        crate::display_row::insert_resolved_display_row_face(
            builder,
            pending.face_id,
            &pending.resolved,
            None,
        );
    }
}

#[cfg(test)]
pub(crate) fn append_lisp_string_to_text_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    text_value: Value,
    source_id: u64,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> DisplayRowPosition {
    let Some(mut source) = crate::display_source::LispStringSourceCursor::new(
        source_id,
        text_value,
        RenderFaceRef::FaceId(base_face_id),
    ) else {
        return position;
    };
    let request = frame.source_append_request(
        position,
        base_face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    let mut source_state = DisplayRowSourceState::default();
    let mut font_metrics = None;
    let Some(outcome) = render_natural_display_source_append_request_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        &mut font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        face_ids,
        request,
    ) else {
        return position;
    };
    outcome.end_position()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BufferTextSourceRange {
    start: CharPos0,
    end: CharPos0,
}

impl BufferTextSourceRange {
    pub(crate) fn new(start: CharPos0, end: CharPos0) -> Self {
        Self { start, end }
    }

    fn start(self) -> CharPos0 {
        self.start
    }

    fn end(self) -> CharPos0 {
        self.end
    }

    fn is_single_char(self) -> bool {
        self.end == self.start.add_len(CharLen::new(1))
    }

    fn is_empty_or_reversed(self) -> bool {
        self.end <= self.start
    }
}

fn buffer_text_source_range_item<B: LayoutBufferView + ?Sized>(
    range: BufferTextSourceRange,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<(DisplayItem, DisplayRowAppendKind)> {
    if !range.is_single_char() {
        return None;
    }

    let start = range.start();
    let end = range.end();
    let byte_start = buffer.layout_char_pos_to_emacs_byte_pos(start);
    let byte_end = buffer.layout_char_pos_to_emacs_byte_pos(end);
    let mut bytes = Vec::new();
    buffer.layout_copy_emacs_byte_range_to(EmacsByteRange::new(byte_start, byte_end), &mut bytes);
    let (ch, len) = decode_utf8(&bytes);
    if len == 0 {
        return None;
    }

    let item = BufferTextItemSource::single_char(buffer_id, start, byte_start, byte_end).item(
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(ch.to_string())),
    );
    let append_kind = if ch == '\t' {
        DisplayRowAppendKind::Tab
    } else {
        DisplayRowAppendKind::SourceText
    };
    Some((item, append_kind))
}

#[allow(clippy::too_many_arguments)]
fn append_resolved_buffer_text_source_range_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    resolved_advance: ResolvedBufferTextSourceAdvance,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let (item, append_kind) = buffer_text_source_range_item(range, buffer_id, buffer, face_id)?;
    let request = frame.source_append_request(position, face_id, base_face, append_kind);
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy =
        BufferTextSourceRenderPolicy::new(resolved_advance.append_measurement());
    let outcome = render_display_source_append_request_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        &mut face_ids,
        request,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress_and_position(position))
}

#[cfg(test)]
pub(crate) struct BufferTextSourceRangeAppendContext<'a, B: LayoutBufferView + ?Sized> {
    buffer: &'a B,
    buffer_id: BufferId,
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

#[cfg(test)]
impl<'a, B: LayoutBufferView + ?Sized> BufferTextSourceRangeAppendContext<'a, B> {
    pub(crate) fn new(
        buffer: &'a B,
        buffer_id: BufferId,
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            buffer,
            buffer_id,
            face_id,
            base_face,
            frame,
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(crate) fn resolve_source_range_advance_to_text_row(
        &self,
        resolver: &mut BufferTextSourceAdvanceResolver,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text: &[u8],
        byte_idx: usize,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowPosition,
        cluster: BufferTextSourceClusterState,
    ) -> ResolvedBufferTextSourceAdvance {
        resolver.resolve_source_range_to_text_row(
            builder,
            evaluator,
            font_metrics,
            text,
            byte_idx,
            range,
            face_resolver,
            self.buffer_id,
            self.buffer,
            active_face_state,
            self.frame.clone(),
            position,
            cluster,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(crate) fn append_resolved_source_range_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        resolved_advance: ResolvedBufferTextSourceAdvance,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_resolved_buffer_text_source_range_to_text_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            self.base_face,
            self.buffer_id,
            self.buffer,
            self.face_id,
            resolved_advance,
            self.frame.clone(),
            position,
        )
    }

    #[cfg(test)]
    pub(crate) fn measure_source_range_natural_advance_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        measure_buffer_text_source_range_natural_advance_to_text_row(
            builder,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            self.base_face,
            self.buffer_id,
            self.buffer,
            self.face_id,
            self.frame.clone(),
            position,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextRowAppendContext<'source, 'surface, B: LayoutBufferView + ?Sized> {
    buffer: &'source B,
    buffer_id: BufferId,
    append_surface: &'surface DisplayRowAppendSurface,
    active_face: &'source DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

impl<'source, 'surface, B: LayoutBufferView + ?Sized>
    BufferTextRowAppendContext<'source, 'surface, B>
{
    pub(crate) fn new(
        buffer: &'source B,
        buffer_id: BufferId,
        append_surface: &'surface DisplayRowAppendSurface,
        active_face: &'source DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            buffer,
            buffer_id,
            append_surface,
            active_face,
            glyph_y_offset,
            default_row_height,
        }
    }

    fn active_face_context<'row>(
        &self,
        geometry: &'row DisplayRowGeometryState,
    ) -> DisplayRowActiveFaceAppendContext<'row, 'source>
    where
        'surface: 'row,
    {
        DisplayRowActiveFaceAppendContext::new(
            self.append_surface,
            geometry,
            self.active_face,
            self.glyph_y_offset,
            self.default_row_height,
        )
    }

    fn item_active_face(
        &self,
        geometry: &DisplayRowGeometryState,
    ) -> BufferTextItemAppendContext<'source, B> {
        let frame = self.active_face_context(geometry).active_face_frame();
        BufferTextItemAppendContext::new(
            self.buffer,
            self.buffer_id,
            self.active_face.face_id(),
            self.active_face.resolved_face(),
            frame,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn measure_item_source_range_width_or_active_face_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> f32 {
        self.item_active_face(geometry)
            .measure_source_range_width_or_active_face_fallback_to_text_row(
                builder,
                evaluator,
                font_metrics,
                range,
                face_resolver,
                item,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_item_source_range_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.item_active_face(geometry)
            .append_source_range_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                range,
                face_resolver,
                item,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_source_range_advance_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        resolver: &mut BufferTextSourceAdvanceResolver,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text: &[u8],
        byte_idx: usize,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        position: DisplayRowPosition,
        cluster: BufferTextSourceClusterState,
    ) -> ResolvedBufferTextSourceAdvance {
        let frame = self.active_face_context(geometry).active_face_frame();
        resolver.resolve_source_range_to_text_row(
            builder,
            evaluator,
            font_metrics,
            text,
            byte_idx,
            range,
            face_resolver,
            self.buffer_id,
            self.buffer,
            self.active_face,
            frame,
            position,
            cluster,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_resolved_source_range_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        resolved_advance: ResolvedBufferTextSourceAdvance,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let frame = self.active_face_context(geometry).active_face_frame();
        append_resolved_buffer_text_source_range_to_text_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            self.active_face.resolved_face(),
            self.buffer_id,
            self.buffer,
            self.active_face.face_id(),
            resolved_advance,
            frame,
            position,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn measure_buffer_text_source_range_append_progress_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<DisplayRowAppendProgress> {
    let (item, append_kind) = buffer_text_source_range_item(range, buffer_id, buffer, face_id)?;
    let request = frame
        .source_append_request(position, face_id, base_face, append_kind)
        .with_measurement_bounds(DisplayRowRenderBounds::unbounded_from(position));
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy =
        BufferTextSourceRenderPolicy::new(BufferTextSourceAppendMeasurement::Natural);
    let outcome = measure_display_source_append_request_against_current_text_row(
        builder,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        &mut face_ids,
        request,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress(position))
}

#[allow(clippy::too_many_arguments)]
fn measure_buffer_text_source_range_natural_advance_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<f32> {
    Some(
        measure_buffer_text_source_range_append_progress_to_text_row(
            builder,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            base_face,
            buffer_id,
            buffer,
            face_id,
            frame,
            position,
        )?
        .metrics
        .width_px,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_buffer_text_source_range_natural_advance_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    face_resolver: &FaceResolver,
    buffer_id: BufferId,
    buffer: &B,
    active_face_state: &DisplayRowActiveFaceState,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    cluster: BufferTextSourceClusterState,
) -> f32 {
    if let Some(measured_width) = measure_buffer_text_source_range_natural_advance_to_text_row(
        builder,
        evaluator,
        font_metrics,
        range,
        face_resolver,
        active_face_state.resolved_face(),
        buffer_id,
        buffer,
        active_face_state.face_id(),
        frame.clone(),
        position,
    ) {
        return measured_width;
    }

    fallback_buffer_text_source_range_natural_advance_to_text_row(
        font_metrics,
        active_face_state,
        &frame,
        position,
        cluster,
    )
}

fn fallback_buffer_text_source_range_natural_advance_to_text_row(
    font_metrics: &mut Option<FontMetricsService>,
    active_face_state: &DisplayRowActiveFaceState,
    frame: &DisplayRowAppendFrame,
    position: DisplayRowPosition,
    cluster: BufferTextSourceClusterState,
) -> f32 {
    BufferTextSourceNaturalFallbackAdvance::for_cluster_state(cluster).resolve_to_text_row(
        font_metrics,
        active_face_state,
        frame,
        position,
        cluster.ch(),
    )
}

impl BufferTextSourceAdvanceResolver {
    #[allow(clippy::too_many_arguments)]
    fn resolve_source_range_to_text_row<B: LayoutBufferView + ?Sized>(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text: &[u8],
        byte_idx: usize,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        buffer_id: BufferId,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        cluster: BufferTextSourceClusterState,
    ) -> ResolvedBufferTextSourceAdvance {
        let ch = cluster.ch();
        match BufferTextSourceAdvancePath::for_cluster_state(cluster) {
            BufferTextSourceAdvancePath::ResolvedComplexRun => {
                let mut policy =
                    DisplayRowComplexTextRunAdvancePolicy::new(active_face_state, font_metrics);
                let advance_px = self.complex_run.advance_for_char(
                    text,
                    byte_idx,
                    ch,
                    cluster.is_cluster_continuation(),
                    &mut policy,
                );
                ResolvedBufferTextSourceAdvance::resolved(advance_px)
            }
            BufferTextSourceAdvancePath::NaturalRenderedSource => {
                let advance_px = resolve_buffer_text_source_range_natural_advance_to_text_row(
                    builder,
                    evaluator,
                    font_metrics,
                    range,
                    face_resolver,
                    buffer_id,
                    buffer,
                    active_face_state,
                    frame,
                    position,
                    cluster,
                );
                ResolvedBufferTextSourceAdvance::natural(advance_px)
            }
        }
    }
}

fn buffer_display_item_source_range_item<B: LayoutBufferView + ?Sized>(
    range: BufferTextSourceRange,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    item: BufferTextSourceAppendItem,
) -> Option<(DisplayItem, DisplayRowAppendKind)> {
    if range.is_empty_or_reversed() {
        return None;
    }

    let start = range.start();
    let end = range.end();
    let source = BufferTextItemSource::new(
        buffer_id,
        start,
        buffer.layout_char_pos_to_emacs_byte_pos(start),
        end,
        buffer.layout_char_pos_to_emacs_byte_pos(end),
    );

    let append_kind = item.append_kind();
    let item = source.item(
        RenderFaceRef::FaceId(face_id),
        item.into_display_item_kind(),
    );
    Some((item, append_kind))
}

#[allow(clippy::too_many_arguments)]
fn append_buffer_display_item_source_range_to_text_row_and_emit<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    buffer: &B,
    buffer_id: BufferId,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_id: u32,
    item: BufferTextSourceAppendItem,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let (item, append_kind) =
        buffer_display_item_source_range_item(range, buffer_id, buffer, face_id, item)?;
    let request = frame.source_append_request(position, face_id, base_face, append_kind);
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let outcome = render_natural_display_source_append_request_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        &mut face_ids,
        request,
    )?;
    Some(outcome.into_append_progress_and_position(position))
}

#[allow(clippy::too_many_arguments)]
fn measure_buffer_display_item_source_range_append_progress_to_text_row<
    B: LayoutBufferView + ?Sized,
>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    range: BufferTextSourceRange,
    buffer: &B,
    buffer_id: BufferId,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_id: u32,
    item: BufferTextSourceAppendItem,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<DisplayRowAppendProgress> {
    let (item, append_kind) =
        buffer_display_item_source_range_item(range, buffer_id, buffer, face_id, item)?;
    let request = frame
        .source_append_request(position, face_id, base_face, append_kind)
        .with_measurement_bounds(DisplayRowRenderBounds::unbounded_from(position));
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    let outcome = measure_display_source_append_request_against_current_text_row(
        builder,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        &mut face_ids,
        request,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress(position))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceAppendItem {
    ControlChar { ch: char },
    SourceMappedText { text: Box<str> },
    Glyphless { ch: char, method: GlyphlessMethod },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextSourceSpecialDisplay {
    Control(BufferTextSourceAppendItem),
    Nobreak(BufferTextSourceAppendItem),
    Glyphless(BufferTextSourceAppendItem),
}

impl BufferTextSourceSpecialDisplay {
    pub(crate) fn for_precluster_char(ch: char, nobreak_display_policy: i32) -> Option<Self> {
        if Self::is_control_char(ch) {
            Some(Self::Control(BufferTextSourceAppendItem::ControlChar {
                ch,
            }))
        } else {
            BufferTextSourceAppendItem::nobreak_display(ch, nobreak_display_policy)
                .map(Self::Nobreak)
        }
    }

    pub(crate) fn for_cluster_state(cluster: BufferTextSourceClusterState) -> Option<Self> {
        BufferTextSourceAppendItem::glyphless_display(cluster).map(Self::Glyphless)
    }

    pub(crate) fn into_append_item(self) -> BufferTextSourceAppendItem {
        match self {
            Self::Control(item) | Self::Nobreak(item) | Self::Glyphless(item) => item,
        }
    }

    pub(crate) fn is_control(&self) -> bool {
        matches!(self, Self::Control(_))
    }

    pub(crate) fn is_nobreak(&self) -> bool {
        matches!(self, Self::Nobreak(_))
    }

    fn is_control_char(ch: char) -> bool {
        (ch < ' ' && ch != '\n' && ch != '\t') || ch == '\x7F'
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BufferTextSourceFallbackWidthPolicy {
    Columns(usize),
}

impl BufferTextSourceFallbackWidthPolicy {
    fn for_append_item(item: &BufferTextSourceAppendItem) -> Self {
        match item {
            BufferTextSourceAppendItem::ControlChar { .. } => Self::Columns(2),
            BufferTextSourceAppendItem::SourceMappedText { text } => {
                Self::Columns(text.chars().count().max(1))
            }
            BufferTextSourceAppendItem::Glyphless { .. } => Self::Columns(1),
        }
    }

    fn width_px(self, fallback_char_width: f32) -> f32 {
        self.columns() as f32 * fallback_char_width.max(1.0)
    }

    fn columns(self) -> usize {
        match self {
            Self::Columns(columns) => columns,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSourceClusterState {
    ch: char,
    tail: Option<(char, bool)>,
    is_cluster_continuation: bool,
}

impl BufferTextSourceClusterState {
    pub(crate) fn for_char(ch: char, tail: Option<(char, bool)>) -> Self {
        Self {
            ch,
            tail,
            is_cluster_continuation: crate::composition::continues_cluster(ch, tail),
        }
    }

    pub(crate) fn is_cluster_continuation(self) -> bool {
        self.is_cluster_continuation
    }

    fn ch(self) -> char {
        self.ch
    }

    fn has_tail(self) -> bool {
        self.tail.is_some()
    }
}

impl BufferTextSourceAppendItem {
    pub(crate) fn nobreak_display(ch: char, display_policy: i32) -> Option<Self> {
        let text = match (display_policy, ch) {
            (1, '\u{00A0}') => " ",
            (1, '\u{00AD}') => "-",
            (2, '\u{00A0}') => "\\ ",
            (2, '\u{00AD}') => "\\-",
            _ => return None,
        };
        Some(Self::SourceMappedText { text: text.into() })
    }

    pub(crate) fn glyphless_display(cluster: BufferTextSourceClusterState) -> Option<Self> {
        let ch = cluster.ch();
        if cluster.has_tail() && crate::composition::is_composition_joiner(ch) {
            return None;
        }
        let method = glyphless_method_for_char(ch, GlyphlessJoinerPolicy::ClassifyAsGlyphless)?;
        Some(Self::Glyphless { ch, method })
    }

    fn append_kind(&self) -> DisplayRowAppendKind {
        match self {
            Self::ControlChar { .. } => DisplayRowAppendKind::ControlChar,
            Self::SourceMappedText { .. } => DisplayRowAppendKind::SourceMappedText,
            Self::Glyphless { .. } => DisplayRowAppendKind::Glyphless,
        }
    }

    fn fallback_width_policy(&self) -> BufferTextSourceFallbackWidthPolicy {
        BufferTextSourceFallbackWidthPolicy::for_append_item(self)
    }

    fn into_display_item_kind(self) -> DisplayItemKind {
        match self {
            Self::ControlChar { ch } => DisplayItemKind::ControlChar { ch },
            Self::SourceMappedText { text } => {
                DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new(text))
            }
            Self::Glyphless { ch, method } => {
                DisplayItemKind::Glyphless(DisplayGlyphless { ch, method })
            }
        }
    }
}

pub(crate) struct BufferTextItemAppendContext<'a, B: LayoutBufferView + ?Sized> {
    buffer: &'a B,
    buffer_id: BufferId,
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a, B: LayoutBufferView + ?Sized> BufferTextItemAppendContext<'a, B> {
    pub(crate) fn new(
        buffer: &'a B,
        buffer_id: BufferId,
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            buffer,
            buffer_id,
            face_id,
            base_face,
            frame,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_source_range_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_buffer_display_item_source_range_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            range,
            self.buffer,
            self.buffer_id,
            face_resolver,
            self.base_face,
            self.face_id,
            item,
            self.frame.clone(),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn measure_source_range_width_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        Some(
            measure_buffer_display_item_source_range_append_progress_to_text_row(
                builder,
                evaluator,
                font_metrics,
                range,
                self.buffer,
                self.buffer_id,
                face_resolver,
                self.base_face,
                self.face_id,
                item,
                self.frame.clone(),
                position,
            )?
            .metrics
            .width_px,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn measure_source_range_width_or_active_face_fallback_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        range: BufferTextSourceRange,
        face_resolver: &FaceResolver,
        item: BufferTextSourceAppendItem,
        position: DisplayRowPosition,
    ) -> f32 {
        let fallback_width = item
            .fallback_width_policy()
            .width_px(self.frame.geometry.char_width);
        self.measure_source_range_width_to_text_row(
            builder,
            evaluator,
            font_metrics,
            range,
            face_resolver,
            item,
            position,
        )
        .unwrap_or(fallback_width)
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementActiveFaceMeasurer {
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayReplacementActiveFaceMeasurer {
    pub(crate) fn from_active_face_state(active_face_state: &DisplayRowActiveFaceState) -> Self {
        Self {
            active_face_state: active_face_state.clone(),
        }
    }

    pub(crate) fn source_char_width_px(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        fallback_advance_px: f32,
    ) -> f32 {
        self.active_face_state
            .advance_for_char(font_metrics, ch, fallback_advance_px)
    }

    fn replacement_string_cursor_slot_width_px(
        &self,
        font_metrics: &mut Option<FontMetricsService>,
        replacement: &str,
        fallback_char_width: f32,
    ) -> f32 {
        replacement
            .chars()
            .next()
            .map(|ch| self.source_char_width_px(font_metrics, ch, fallback_char_width))
            .unwrap_or_else(|| fallback_char_width.max(1.0))
    }
}

pub(crate) struct DisplayReplacementStringItemMeasurer {
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayRowRenderPolicy for DisplayReplacementStringItemMeasurer {
    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        _face_id: u32,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        let DisplayItemKind::SourceMappedText(text) = &item.kind else {
            return DisplayRowItemMeasurement::Default;
        };
        DisplayRowItemMeasurement::TextRun(
            self.active_face_state
                .text_run_measurement(font_metrics, text.text.as_ref()),
        )
    }
}

struct DisplayReplacementStringRenderPolicy<'a, M> {
    item_policy: &'a mut M,
}

impl<M: DisplayRowRenderPolicy> DisplayRowRenderPolicy
    for DisplayReplacementStringRenderPolicy<'_, M>
{
    fn stop_before_item(&mut self, item: &DisplayItem) -> bool {
        matches!(item.kind, DisplayItemKind::RowBreak(_))
    }

    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        face_id: u32,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        self.item_policy
            .measurement_for(item, face_id, font_metrics)
    }

    fn clipped_behavior(&mut self, item: &DisplayItem) -> DisplayRowRenderClipBehavior {
        if matches!(item.kind, DisplayItemKind::SourceMappedText(_)) {
            DisplayRowRenderClipBehavior::Stop
        } else {
            DisplayRowRenderClipBehavior::Continue
        }
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementStringAppendItem {
    value: Value,
    origin: DisplayOrigin,
    base_face_policy: BaseFacePolicy,
    source_id: u64,
    active_face_state: DisplayRowActiveFaceState,
    cursor_slot_width_px: f32,
    is_empty: bool,
}

impl DisplayReplacementStringAppendItem {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn display_property_string(
        value: Value,
        anchor_charpos: CharPos0,
        source: DisplayPropertySource,
        source_id: u64,
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        fallback_char_width: f32,
    ) -> Option<Self> {
        let replacement = value.as_utf8_str()?;
        let measurer =
            DisplayReplacementActiveFaceMeasurer::from_active_face_state(active_face_state);
        Some(Self {
            value,
            origin: DisplayOrigin::DisplayPropertyString {
                anchor_charpos,
                source,
            },
            base_face_policy: BaseFacePolicy::DisplayPropertyUnderlyingFace,
            source_id,
            active_face_state: active_face_state.clone(),
            cursor_slot_width_px: measurer.replacement_string_cursor_slot_width_px(
                font_metrics,
                replacement,
                fallback_char_width,
            ),
            is_empty: replacement.is_empty(),
        })
    }

    pub(crate) fn cursor_slot_width_px(&self) -> f32 {
        self.cursor_slot_width_px
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.is_empty
    }

    pub(crate) fn origin(&self) -> DisplayOrigin {
        self.origin
    }

    pub(crate) fn base_face_policy(&self) -> BaseFacePolicy {
        self.base_face_policy
    }

    fn value(&self) -> Value {
        self.value
    }

    fn source_id(&self) -> u64 {
        self.source_id
    }

    fn string_item_measurer(&self) -> DisplayReplacementStringItemMeasurer {
        DisplayReplacementStringItemMeasurer {
            active_face_state: self.active_face_state.clone(),
        }
    }
}

fn append_raw_display_replacement_item_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    item: DisplayItem,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let face_id = render_face_ref_id(item.face, fallback_face_id);
    let request = frame.source_append_request(
        position,
        face_id,
        base_face,
        DisplayRowAppendKind::DisplayReplacement,
    );
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    append_single_display_item_fragment_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        request,
        &mut render_policy,
    )
}

#[derive(Clone, Debug)]
enum DisplayReplacementAppendItem {
    Media(DisplayMediaReplacement),
    Stretch(DisplayReplacementBox),
    SourceMappedText(Box<str>),
}

impl DisplayReplacementAppendItem {
    fn into_display_item(
        self,
        replacement_source: BufferDisplayReplacementSource,
        face_id: u32,
    ) -> DisplayItem {
        match self {
            Self::Media(media) => replacement_source.media_item(face_id, media),
            Self::Stretch(geometry) => replacement_source.stretch_item(face_id, geometry),
            Self::SourceMappedText(text) => {
                replacement_source.source_mapped_text_item(face_id, text)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DisplayReplacementStretchAppendItem {
    geometry: DisplayReplacementBox,
    width_px: f32,
    height_px: f32,
    ascent_px: f32,
    cursor_slot_width_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayReplacementSpaceGeometry {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug)]
enum DisplayReplacementSpaceWidthPolicy {
    Explicit(Value),
    Relative { factor: f32 },
    AlignTo(Value),
    Default,
}

impl DisplayReplacementSpaceWidthPolicy {
    fn from_items(items: &[Value]) -> Self {
        if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Width)
            && !prop.is_nil()
        {
            Self::Explicit(prop)
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::RelativeWidth)
            && let Some(factor) = display_space_positive_number(prop)
        {
            Self::Relative { factor }
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::AlignTo)
            && !prop.is_nil()
        {
            Self::AlignTo(prop)
        } else {
            Self::Default
        }
    }

    fn zero_width_allowed(self) -> bool {
        matches!(self, Self::AlignTo(_))
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        current_x: f32,
        content_x: f32,
        display_char_width: f32,
        default_width: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Explicit(prop) => calc_pixel_width_or_height(pctx, &prop, true, None)
                .map(|pixels| pixels as f32)
                .unwrap_or(default_width),
            Self::Relative { factor } => factor * display_char_width.max(0.0),
            Self::AlignTo(prop) => {
                let mut align_to: i32 = -1;
                if let Some(pixels) =
                    calc_pixel_width_or_height(pctx, &prop, true, Some(&mut align_to))
                {
                    let target_x = if align_to >= 0 {
                        align_to as f32 + pixels as f32
                    } else {
                        content_x + pixels as f32
                    };
                    (target_x - current_x).max(0.0)
                } else {
                    default_width
                }
            }
            Self::Default => default_width,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DisplayReplacementSpaceHeightPolicy {
    Explicit(Value),
    Relative { factor: f32 },
    Default,
}

impl DisplayReplacementSpaceHeightPolicy {
    fn from_items(items: &[Value]) -> Self {
        if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Height)
            && !prop.is_nil()
        {
            Self::Explicit(prop)
        } else if let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::RelativeHeight)
            && let Some(factor) = display_space_positive_number(prop)
        {
            Self::Relative { factor }
        } else {
            Self::Default
        }
    }

    fn zero_height_allowed(self) -> bool {
        matches!(self, Self::Explicit(_))
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        default_height: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Explicit(prop) => calc_pixel_width_or_height(pctx, &prop, false, None)
                .map(|pixels| pixels as f32)
                .unwrap_or(default_height),
            Self::Relative { factor } => default_height * factor,
            Self::Default => default_height,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DisplayReplacementSpaceAscentPolicy {
    Percent { percent: f32 },
    Pixel(Value),
    Default,
}

impl DisplayReplacementSpaceAscentPolicy {
    fn from_items(items: &[Value]) -> Self {
        let Some(prop) = display_space_plist_value(items, DisplaySpaceKey::Ascent) else {
            return Self::Default;
        };
        if let Some(percent) = display_space_positive_number(prop)
            && percent <= 100.0
        {
            Self::Percent { percent }
        } else if !prop.is_nil() {
            Self::Pixel(prop)
        } else {
            Self::Default
        }
    }

    fn resolve(
        self,
        pctx: &crate::display_pixel_calc::PixelCalcContext,
        height: f32,
        default_ascent: f32,
        default_height: f32,
    ) -> f32 {
        use crate::display_pixel_calc::calc_pixel_width_or_height;

        match self {
            Self::Percent { percent } => height * percent / 100.0,
            Self::Pixel(prop) => calc_pixel_width_or_height(pctx, &prop, false, None)
                .map(|pixels| (pixels as f32).max(0.0).min(height))
                .unwrap_or_else(|| Self::default_ascent(height, default_ascent, default_height)),
            Self::Default => Self::default_ascent(height, default_ascent, default_height),
        }
    }

    fn default_ascent(height: f32, default_ascent: f32, default_height: f32) -> f32 {
        height * default_ascent / default_height
    }
}

fn display_space_plist_value(items: &[Value], wanted: DisplaySpaceKey) -> Option<Value> {
    let mut i = 1;
    while i + 1 < items.len() {
        if DisplaySpaceKey::from_lisp_value(items[i]) == Some(wanted) {
            return Some(items[i + 1]);
        }
        i += 2;
    }
    None
}

impl DisplayReplacementStretchAppendItem {
    pub(crate) fn from_extents(width_px: f32, height_px: f32, ascent_px: f32) -> Self {
        let width_px = width_px.max(0.0);
        let height_px = height_px.max(0.0);
        let ascent_px = ascent_px.max(0.0);
        Self {
            geometry: DisplayReplacementBox::new(width_px, height_px, ascent_px),
            width_px,
            height_px,
            ascent_px,
            cursor_slot_width_px: width_px,
        }
    }

    pub(crate) fn from_space_extents(
        width_px: f32,
        height_px: f32,
        ascent_px: f32,
        fallback_cursor_width_px: f32,
    ) -> Self {
        let mut item = Self::from_extents(width_px, height_px, ascent_px);
        item.cursor_slot_width_px = item.width_px.max(fallback_cursor_width_px);
        item
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn display_space_geometry(
        spec: &Value,
        current_x: f32,
        content_x: f32,
        face_char_w: f32,
        display_char_width: f32,
        default_height: f32,
        default_ascent: f32,
        params: &WindowParams,
    ) -> DisplayReplacementSpaceGeometry {
        use crate::display_pixel_calc::PixelCalcContext;

        let default_width = params.char_width.max(1.0);
        let default_height = if params.window_system {
            default_height.max(1.0)
        } else {
            params.char_height.max(1.0)
        };
        let default_ascent = if params.window_system {
            default_ascent.max(0.0).min(default_height)
        } else {
            default_height
        };
        let Some(items) = neovm_core::emacs_core::value::list_to_vec(spec) else {
            return DisplayReplacementSpaceGeometry {
                width: default_width,
                height: default_height,
                ascent: default_ascent,
            };
        };

        let pctx = PixelCalcContext {
            frame_column_width: params.char_width.max(1.0) as f64,
            frame_line_height: params.char_height.max(1.0) as f64,
            frame_res_x: 96.0,
            frame_res_y: 96.0,
            face_font_height: default_height as f64,
            face_font_width: face_char_w.round().max(1.0) as f64,
            text_area_left: params.text_bounds.x as f64,
            text_area_right: (params.text_bounds.x + params.text_bounds.width) as f64,
            text_area_width: params.text_bounds.width as f64,
            left_margin_left: (params.text_bounds.x
                - params.left_fringe_width
                - params.left_margin_width) as f64,
            left_margin_width: params.left_margin_width as f64,
            right_margin_left: (params.text_bounds.x
                + params.text_bounds.width
                + params.right_fringe_width) as f64,
            right_margin_width: params.right_margin_width as f64,
            left_fringe_width: params.left_fringe_width as f64,
            right_fringe_width: params.right_fringe_width as f64,
            fringes_outside_margins: false,
            scroll_bar_width: 0.0,
            scroll_bar_on_left: false,
            line_number_pixel_width: 0.0,
            symbol_values: std::collections::HashMap::new(),
        };

        let width_policy = DisplayReplacementSpaceWidthPolicy::from_items(&items);
        let mut width = width_policy.resolve(
            &pctx,
            current_x,
            content_x,
            display_char_width,
            default_width,
        );
        if width <= 0.0 && (width < 0.0 || !width_policy.zero_width_allowed()) {
            width = 1.0;
        }

        let (height, ascent) = if params.window_system {
            let height_policy = DisplayReplacementSpaceHeightPolicy::from_items(&items);
            let mut height = height_policy.resolve(&pctx, default_height);
            if height <= 0.0 && (height < 0.0 || !height_policy.zero_height_allowed()) {
                height = 1.0;
            }

            let ascent = DisplayReplacementSpaceAscentPolicy::from_items(&items).resolve(
                &pctx,
                height,
                default_ascent,
                default_height,
            );
            (height, ascent)
        } else {
            (1.0, 1.0)
        };

        DisplayReplacementSpaceGeometry {
            width,
            height,
            ascent: ascent.max(0.0).min(height),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_display_space_spec(
        spec: &Value,
        current_x: f32,
        content_x: f32,
        face_char_w: f32,
        display_char_width: f32,
        default_height: f32,
        default_ascent: f32,
        fallback_cursor_width_px: f32,
        params: &WindowParams,
    ) -> Self {
        let geometry = Self::display_space_geometry(
            spec,
            current_x,
            content_x,
            face_char_w,
            display_char_width,
            default_height,
            default_ascent,
            params,
        );
        Self::from_space_extents(
            geometry.width,
            geometry.height,
            geometry.ascent,
            fallback_cursor_width_px,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_display_space_property(
        spec: &Value,
        source_text: &[u8],
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        current_x: f32,
        content_x: f32,
        default_char_width: f32,
        default_height: f32,
        default_ascent: f32,
        params: &WindowParams,
    ) -> Self {
        let (display_ch, _) = decode_utf8(source_text);
        let display_char_width = Self::source_char_width_px(
            active_face_state,
            font_metrics,
            display_ch,
            default_char_width,
        );
        Self::from_display_space_spec(
            spec,
            current_x,
            content_x,
            default_char_width,
            display_char_width,
            default_height,
            default_ascent,
            default_char_width,
            params,
        )
    }

    fn source_char_width_px(
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        ch: char,
        fallback_advance_px: f32,
    ) -> f32 {
        DisplayReplacementActiveFaceMeasurer::from_active_face_state(active_face_state)
            .source_char_width_px(font_metrics, ch, fallback_advance_px)
    }

    pub(crate) fn width_px(self) -> f32 {
        self.width_px
    }

    pub(crate) fn height_px(self) -> f32 {
        self.height_px
    }

    pub(crate) fn ascent_px(self) -> f32 {
        self.ascent_px
    }

    pub(crate) fn cursor_slot_width_px(self) -> f32 {
        self.cursor_slot_width_px
    }

    fn geometry(self) -> DisplayReplacementBox {
        self.geometry
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayReplacementMediaAppendItem {
    media: DisplayMediaReplacement,
    cursor_face_height: f32,
    cursor_face_ascent: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayReplacementMediaAppendResolution {
    Media(DisplayReplacementMediaAppendItem),
    Placeholder(DisplayReplacementSourceMappedTextAppendItem),
}

#[derive(Clone)]
pub(crate) enum DisplayPropertyReplacementAppendItem {
    String(DisplayReplacementStringAppendItem),
    Stretch(DisplayReplacementStretchAppendItem),
    Media(DisplayReplacementMediaAppendResolution),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisplayPropertyReplacementCursorPolicy {
    TextSlot {
        width_px: f32,
        stretch_like: bool,
    },
    DisplayBox {
        width_px: f32,
        cursor_face_height_px: f32,
        cursor_face_ascent_px: f32,
    },
    FaceChar,
}

impl DisplayPropertyReplacementAppendItem {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve(
        display_property: &DisplayPropertyClassification,
        value: Value,
        anchor_charpos: CharPos0,
        source_text: &[u8],
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        current_x: f32,
        content_x: f32,
        params: &WindowParams,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<Self> {
        let face_metrics = active_face_state.metrics();
        if display_property.is_string_replacement() {
            DisplayReplacementStringAppendItem::display_property_string(
                value,
                anchor_charpos,
                DisplayPropertySource::TextProperty,
                1,
                active_face_state,
                font_metrics,
                face_metrics.char_width,
            )
            .map(Self::String)
        } else if display_property.stretch_replacement().is_some() {
            Some(Self::Stretch(
                DisplayReplacementStretchAppendItem::from_display_space_property(
                    &value,
                    source_text,
                    active_face_state,
                    font_metrics,
                    current_x,
                    content_x,
                    face_metrics.char_width,
                    face_metrics.row_height,
                    face_metrics.ascent,
                    params,
                ),
            ))
        } else {
            let media_replacement = display_property.media_replacement()?;
            DisplayReplacementMediaAppendItem::resolve_display_property(
                value,
                media_replacement,
                display_host,
                active_face_state,
                face_metrics.char_width,
                face_metrics.row_height,
            )
            .map(Self::Media)
        }
    }

    pub(crate) fn cursor_policy(&self) -> DisplayPropertyReplacementCursorPolicy {
        match self {
            Self::String(item) => DisplayPropertyReplacementCursorPolicy::TextSlot {
                width_px: item.cursor_slot_width_px(),
                stretch_like: false,
            },
            Self::Stretch(item) => DisplayPropertyReplacementCursorPolicy::TextSlot {
                width_px: item.cursor_slot_width_px(),
                stretch_like: true,
            },
            Self::Media(DisplayReplacementMediaAppendResolution::Media(item)) => {
                DisplayPropertyReplacementCursorPolicy::DisplayBox {
                    width_px: item.width_px(),
                    cursor_face_height_px: item.cursor_face_height_px(),
                    cursor_face_ascent_px: item.cursor_face_ascent_px(),
                }
            }
            Self::Media(DisplayReplacementMediaAppendResolution::Placeholder(_)) => {
                DisplayPropertyReplacementCursorPolicy::FaceChar
            }
        }
    }
}

impl DisplayReplacementMediaAppendItem {
    pub(crate) fn new(
        media: DisplayMediaReplacement,
        active_face_state: &DisplayRowActiveFaceState,
        uses_xwidget_cursor_extents: bool,
    ) -> Self {
        let metrics = active_face_state.metrics();
        let (cursor_face_height, cursor_face_ascent) = if uses_xwidget_cursor_extents {
            (
                media.height.max(metrics.row_height),
                media.height.max(metrics.ascent),
            )
        } else {
            (media.height, media.height)
        };
        Self {
            media,
            cursor_face_height,
            cursor_face_ascent,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_display_property(
        display_prop: Value,
        replacement: &DisplayMediaReplacementProperty,
        display_host: Option<&dyn DisplayHost>,
        active_face_state: &DisplayRowActiveFaceState,
        fallback_char_width: f32,
        fallback_row_height: f32,
    ) -> Option<DisplayReplacementMediaAppendResolution> {
        match resolve_display_replacement(
            display_prop,
            replacement,
            display_host,
            active_face_state.resolved_face(),
            fallback_char_width,
            fallback_row_height,
        )? {
            ResolvedDisplayReplacement::Media(media) => {
                Some(DisplayReplacementMediaAppendResolution::Media(Self::new(
                    media,
                    active_face_state,
                    replacement.uses_xwidget_cursor_extents(),
                )))
            }
            ResolvedDisplayReplacement::Placeholder(placeholder) => {
                Some(DisplayReplacementMediaAppendResolution::Placeholder(
                    DisplayReplacementSourceMappedTextAppendItem::new(placeholder),
                ))
            }
        }
    }

    pub(crate) fn media(self) -> DisplayMediaReplacement {
        self.media
    }

    pub(crate) fn width_px(self) -> f32 {
        self.media.width
    }

    pub(crate) fn display_height_px(self) -> f32 {
        self.media.height
    }

    pub(crate) fn display_ascent_px(self) -> f32 {
        self.media.height
    }

    pub(crate) fn cursor_face_height_px(self) -> f32 {
        self.cursor_face_height
    }

    pub(crate) fn cursor_face_ascent_px(self) -> f32 {
        self.cursor_face_ascent
    }

    pub(crate) fn row_extents_after_append(
        self,
        progress: &DisplayRowAppendProgress,
    ) -> Option<(f32, f32)> {
        if progress.status == DisplayRowAppendStatus::Complete && progress.metrics.width_px > 0.0 {
            Some((self.display_height_px(), self.display_ascent_px()))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisplayReplacementSourceMappedTextAppendItem {
    text: Box<str>,
}

impl DisplayReplacementSourceMappedTextAppendItem {
    pub(crate) fn new(text: impl Into<Box<str>>) -> Self {
        Self { text: text.into() }
    }

    fn text(self) -> Box<str> {
        self.text
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayReplacementRowAppendContext<'a> {
    replacement_source: BufferDisplayReplacementSource,
    append_surface: &'a DisplayRowAppendSurface,
    placement: DisplayRowAppendPlacement,
    active_face: &'a DisplayRowActiveFaceState,
    default_row_height: f32,
}

impl<'a> DisplayReplacementRowAppendContext<'a> {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            replacement_source,
            append_surface,
            placement: DisplayRowAppendPlacement::from_geometry_state(geometry, glyph_y_offset),
            active_face,
            default_row_height,
        }
    }

    fn active_face_frame(self) -> DisplayRowAppendFrame {
        self.append_surface.frame(
            self.placement,
            DisplayRowAppendMetrics::from_active_face_state(
                self.active_face,
                self.default_row_height,
            ),
        )
    }

    fn full_text_width_active_face_frame(self) -> DisplayRowAppendFrame {
        self.append_surface.full_text_width_surface().frame(
            self.placement,
            DisplayRowAppendMetrics::from_active_face_state(
                self.active_face,
                self.default_row_height,
            ),
        )
    }

    fn display_box_frame(self, height_px: f32, ascent_px: f32) -> DisplayRowAppendFrame {
        self.append_surface.frame(
            self.placement,
            DisplayRowAppendMetrics::display_box_from_active_face_state(
                self.active_face,
                height_px,
                ascent_px,
                self.default_row_height,
            ),
        )
    }

    fn active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.active_face_frame(),
        )
    }

    fn full_text_width_active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.full_text_width_active_face_frame(),
        )
    }

    fn display_box(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
        height_px: f32,
        ascent_px: f32,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.display_box_frame(height_px, ascent_px),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_full_text_width_string_item_to_text_row(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        item: DisplayReplacementStringAppendItem,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        face_id: u32,
        base_face: &'a ResolvedFace,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        self.full_text_width_active_face(face_id, base_face)
            .append_string_item_to_text_row(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                item,
                face_resolver,
                face_ids,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_active_face_stretch_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementStretchAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.active_face(self.active_face.face_id(), self.active_face.resolved_face())
            .append_stretch_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                item,
                position,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_display_box_media_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementMediaAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.display_box(
            self.active_face.face_id(),
            self.active_face.resolved_face(),
            item.display_height_px(),
            item.display_ascent_px(),
        )
        .append_media_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            item,
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_active_face_source_mapped_text_to_text_row_and_emit(
        self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementSourceMappedTextAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.active_face(self.active_face.face_id(), self.active_face.resolved_face())
            .append_source_mapped_text_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                font_metrics,
                face_resolver,
                item,
                position,
            )
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementAppendContext<'a> {
    replacement_source: BufferDisplayReplacementSource,
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> DisplayReplacementAppendContext<'a> {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            replacement_source,
            face_id,
            base_face,
            frame,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn append_item_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        append_display_replacement_item_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            self.replacement_source,
            self.face_id,
            item,
            face_resolver,
            self.base_face,
            self.frame.clone(),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_stretch_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementStretchAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.append_item_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            DisplayReplacementAppendItem::Stretch(item.geometry()),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_media_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementMediaAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.append_item_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            DisplayReplacementAppendItem::Media(item.media()),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_source_mapped_text_to_text_row_and_emit(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        face_resolver: &FaceResolver,
        item: DisplayReplacementSourceMappedTextAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.append_item_to_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            face_resolver,
            DisplayReplacementAppendItem::SourceMappedText(item.text()),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_string_item_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        item: DisplayReplacementStringAppendItem,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        let value = item.value();
        let source_id = item.source_id();
        let mut item_policy = item.string_item_measurer();
        self.append_string_source_value_to_text_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            value,
            source_id,
            face_resolver,
            face_ids,
            position,
            &mut item_policy,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn append_string_source_value_to_text_row(
        &self,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        value: Value,
        source_id: u64,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
        item_policy: &mut impl DisplayRowRenderPolicy,
    ) -> DisplayRowPosition {
        append_display_replacement_string_value_to_text_row(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            value,
            self.replacement_source,
            source_id,
            face_resolver,
            self.base_face,
            self.face_id,
            face_ids,
            self.frame.clone(),
            position,
            item_policy,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn append_display_replacement_item_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    replacement_source: BufferDisplayReplacementSource,
    face_id: u32,
    item: DisplayReplacementAppendItem,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let item = item.into_display_item(replacement_source, face_id);
    append_raw_display_replacement_item_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        base_face,
        face_id,
        frame,
        position,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_display_replacement_string_source_to_text_row<S: DisplayItemSource>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    mut source: S,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    item_policy: &mut impl DisplayRowRenderPolicy,
) -> DisplayRowPosition {
    let request = frame.source_append_request(
        position,
        fallback_face_id,
        base_face,
        DisplayRowAppendKind::DisplayReplacementString,
    );
    let mut source_state = DisplayRowSourceState::default();
    let mut render_policy = DisplayReplacementStringRenderPolicy { item_policy };
    let Some(outcome) = render_display_source_append_request_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        face_ids,
        request,
        &mut render_policy,
    ) else {
        return position;
    };
    outcome.end_position()
}

#[allow(clippy::too_many_arguments)]
fn append_display_replacement_string_value_to_text_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    text_value: Value,
    replacement_source: BufferDisplayReplacementSource,
    source_id: u64,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    item_policy: &mut impl DisplayRowRenderPolicy,
) -> DisplayRowPosition {
    let Some(string_source) = LispStringSourceCursor::new(
        source_id,
        text_value,
        RenderFaceRef::FaceId(fallback_face_id),
    ) else {
        return position;
    };
    let source = BufferDisplayReplacementStringSource::new(replacement_source, string_source);
    append_display_replacement_string_source_to_text_row(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        source,
        face_resolver,
        base_face,
        fallback_face_id,
        face_ids,
        frame,
        position,
        item_policy,
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DisplayRowAppendPlacement {
    row: usize,
    y: f32,
    glyph_y: f32,
}

impl DisplayRowAppendPlacement {
    fn new(row: usize, y: f32, glyph_y: f32) -> Self {
        Self { row, y, glyph_y }
    }

    fn from_geometry_state(geometry: &DisplayRowGeometryState, glyph_y_offset: f32) -> Self {
        Self::new(
            geometry.row(),
            geometry.y(),
            geometry.glyph_y(glyph_y_offset),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendArea {
    content_x: f32,
    width: f32,
    text_width: f32,
    line_number_width: f32,
}

impl DisplayRowAppendArea {
    pub(crate) fn new(content_x: f32, width: f32, text_width: f32, line_number_width: f32) -> Self {
        Self {
            content_x,
            width,
            text_width,
            line_number_width,
        }
    }

    pub(crate) fn content_x(self) -> f32 {
        self.content_x
    }

    pub(crate) fn right_edge(self) -> f32 {
        self.content_x + self.width
    }

    fn full_text_width(self) -> Self {
        Self {
            width: (self.text_width - self.line_number_width).max(0.0),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendSurface {
    area: DisplayRowAppendArea,
    tab_policy: DisplayTabPolicy,
}

impl DisplayRowAppendSurface {
    pub(crate) fn new(area: DisplayRowAppendArea, tab_policy: DisplayTabPolicy) -> Self {
        Self { area, tab_policy }
    }

    pub(crate) fn content_x(&self) -> f32 {
        self.area.content_x()
    }

    pub(crate) fn right_edge(&self) -> f32 {
        self.area.right_edge()
    }

    pub(crate) fn full_text_right_edge(&self) -> f32 {
        self.area.full_text_width().right_edge()
    }

    pub(crate) fn full_text_width_surface(&self) -> Self {
        Self {
            area: self.area.full_text_width(),
            tab_policy: self.tab_policy.clone(),
        }
    }

    fn frame(
        &self,
        placement: DisplayRowAppendPlacement,
        metrics: DisplayRowAppendMetrics,
    ) -> DisplayRowAppendFrame {
        DisplayRowAppendFrame::from_parts(placement, self.area, metrics, self.tab_policy.clone())
    }

    fn frame_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        metrics: DisplayRowAppendMetrics,
    ) -> DisplayRowAppendFrame {
        self.frame(
            DisplayRowAppendPlacement::from_geometry_state(geometry, glyph_y_offset),
            metrics,
        )
    }

    fn text_row_frame_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> DisplayRowAppendFrame {
        self.frame_from_geometry_state(
            geometry,
            glyph_y_offset,
            DisplayRowAppendMetrics::text_row(height, ascent, char_width, default_row_height),
        )
    }

    fn frame_for_active_face_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        active_face: &DisplayRowActiveFaceState,
        default_row_height: f32,
    ) -> DisplayRowAppendFrame {
        self.frame_from_geometry_state(
            geometry,
            glyph_y_offset,
            DisplayRowAppendMetrics::from_active_face_state(active_face, default_row_height),
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowTextAppendContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    geometry: &'a DisplayRowGeometryState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

impl<'a> DisplayRowTextAppendContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &'a DisplayRowGeometryState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            append_surface,
            geometry,
            glyph_y_offset,
            default_row_height,
        }
    }

    pub(crate) fn text_row_frame(
        self,
        height: f32,
        ascent: f32,
        char_width: f32,
    ) -> DisplayRowAppendFrame {
        self.append_surface.text_row_frame_from_geometry_state(
            self.geometry,
            self.glyph_y_offset,
            height,
            ascent,
            char_width,
            self.default_row_height,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowActiveFaceAppendContext<'row, 'face> {
    text_context: DisplayRowTextAppendContext<'row>,
    active_face: &'face DisplayRowActiveFaceState,
}

impl<'row, 'face> DisplayRowActiveFaceAppendContext<'row, 'face> {
    pub(crate) fn new(
        append_surface: &'row DisplayRowAppendSurface,
        geometry: &'row DisplayRowGeometryState,
        active_face: &'face DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            text_context: DisplayRowTextAppendContext::new(
                append_surface,
                geometry,
                glyph_y_offset,
                default_row_height,
            ),
            active_face,
        }
    }

    pub(crate) fn active_face_frame(self) -> DisplayRowAppendFrame {
        self.text_context
            .append_surface
            .frame_for_active_face_from_geometry_state(
                self.text_context.geometry,
                self.text_context.glyph_y_offset,
                self.active_face,
                self.text_context.default_row_height,
            )
    }

    #[cfg(test)]
    pub(crate) fn full_text_width_active_face_frame(self) -> DisplayRowAppendFrame {
        self.text_context
            .append_surface
            .full_text_width_surface()
            .frame_for_active_face_from_geometry_state(
                self.text_context.geometry,
                self.text_context.glyph_y_offset,
                self.active_face,
                self.text_context.default_row_height,
            )
    }

    pub(crate) fn text_row_frame(
        self,
        height: f32,
        ascent: f32,
        char_width: f32,
    ) -> DisplayRowAppendFrame {
        self.text_context.text_row_frame(height, ascent, char_width)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendMetrics {
    height: f32,
    ascent: f32,
    char_width: f32,
    space_width: f32,
    default_row_height: f32,
}

impl DisplayRowAppendMetrics {
    fn new(
        height: f32,
        ascent: f32,
        char_width: f32,
        space_width: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            height,
            ascent,
            char_width,
            space_width,
            default_row_height,
        }
    }

    pub(crate) fn text_row(
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> Self {
        Self::new(height, ascent, char_width, char_width, default_row_height)
    }

    pub(crate) fn from_active_face_state(
        active_face: &DisplayRowActiveFaceState,
        default_row_height: f32,
    ) -> Self {
        Self::from_measured_face_metrics(active_face.metrics(), default_row_height)
    }

    pub(crate) fn display_box_from_active_face_state(
        active_face: &DisplayRowActiveFaceState,
        height: f32,
        ascent: f32,
        default_row_height: f32,
    ) -> Self {
        let metrics = active_face.metrics();
        Self::new(
            height,
            ascent,
            metrics.char_width,
            metrics.space_width,
            default_row_height,
        )
    }

    pub(crate) fn from_measured_face_metrics(
        metrics: DisplayRowMeasuredFaceMetrics,
        default_row_height: f32,
    ) -> Self {
        Self::new(
            metrics.row_height,
            metrics.ascent,
            metrics.char_width,
            metrics.space_width,
            default_row_height,
        )
    }
}

#[derive(Clone)]
pub(crate) struct DisplayRowAppendFrame {
    row: usize,
    glyph_y: f32,
    geometry: DisplayRowGeometry,
    default_row_height: f32,
    content_x: f32,
    text_width: f32,
    line_number_width: f32,
    face_space_width: f32,
}

impl DisplayRowAppendFrame {
    fn from_parts(
        placement: DisplayRowAppendPlacement,
        area: DisplayRowAppendArea,
        metrics: DisplayRowAppendMetrics,
        tab_policy: DisplayTabPolicy,
    ) -> Self {
        Self {
            row: placement.row,
            glyph_y: placement.glyph_y,
            geometry: DisplayRowGeometry {
                y: placement.y,
                width: area.width,
                height: metrics.height,
                char_width: metrics.char_width,
                ascent: metrics.ascent,
                tab_policy,
            },
            default_row_height: metrics.default_row_height,
            content_x: area.content_x,
            text_width: area.text_width,
            line_number_width: area.line_number_width,
            face_space_width: metrics.space_width,
        }
    }

    fn source_append_request<'face>(
        &self,
        position: DisplayRowPosition,
        face_id: u32,
        base_face: &'face ResolvedFace,
        kind: DisplayRowAppendKind,
    ) -> DisplayRowSourceAppendRequest<'face> {
        let geometry = DisplayRowGeometry {
            char_width: kind.char_width(self),
            ..self.geometry.clone()
        };
        let request = DisplayRowSourceGeometry::from_display_row_geometry(geometry)
            .source_request_for_base_face_id(face_id, base_face, GlyphRowRole::Text)
            .with_render_bounds(DisplayRowRenderBounds {
                start: position,
                max_x_px: kind.max_x(self),
            });
        let output = TextRowOutput {
            row: self.row,
            row_y: self.geometry.y,
            glyph_y: self.glyph_y,
            height: kind.output_height(self),
        };
        DisplayRowSourceAppendRequest::new(request, output, position, face_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DisplayRowAppendKind {
    SourceText,
    Tab,
    ControlChar,
    SourceMappedText,
    Glyphless,
    DisplayReplacement,
    DisplayReplacementString,
}

impl DisplayRowAppendKind {
    fn char_width(self, frame: &DisplayRowAppendFrame) -> f32 {
        match self {
            Self::Tab | Self::DisplayReplacementString => frame.face_space_width,
            Self::SourceText
            | Self::ControlChar
            | Self::SourceMappedText
            | Self::Glyphless
            | Self::DisplayReplacement => frame.geometry.char_width,
        }
    }

    fn max_x(self, frame: &DisplayRowAppendFrame) -> f32 {
        match self {
            Self::Tab => f32::INFINITY,
            Self::ControlChar => frame.content_x + (frame.text_width - frame.line_number_width),
            Self::SourceText
            | Self::SourceMappedText
            | Self::Glyphless
            | Self::DisplayReplacement
            | Self::DisplayReplacementString => frame.content_x + frame.geometry.width,
        }
    }

    fn output_height(self, frame: &DisplayRowAppendFrame) -> f32 {
        match self {
            Self::SourceText
            | Self::Glyphless
            | Self::DisplayReplacement
            | Self::DisplayReplacementString => frame.geometry.height,
            Self::Tab | Self::ControlChar | Self::SourceMappedText => frame.default_row_height,
        }
    }
}

fn append_synthetic_text_to_display_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source_id: u64,
    text: impl Into<Box<str>>,
    face_id: u32,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let item = synthetic_display_text_item(source_id, text, face_id);
    let request = frame.source_append_request(
        position,
        face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    append_single_display_item_fragment_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        item,
        face_resolver,
        request,
        &mut render_policy,
    )
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
