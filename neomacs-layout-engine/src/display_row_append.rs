use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayMediaReplacement,
    DisplaySourceMappedText, DisplaySourcePosition, DisplayTextRun, GlyphlessMethod, RenderFaceRef,
    SourceSpan,
};
use crate::display_origin::DisplayOrigin;
#[cfg(test)]
use crate::display_row::RenderedDisplayRow;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_current_row;
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy, DisplayRowGeometry,
    DisplayRowMeasuredFaceMetrics, DisplayRowOutputProgress, DisplayRowRenderBounds,
    DisplayRowRenderClipBehavior, DisplayRowRenderContext, DisplayRowRenderPolicy,
    DisplayRowRenderStop, DisplayRowRenderer, DisplayRowSourceRenderRequest, DisplayRowSourceState,
    install_rendered_display_row_fragment_assets,
    merge_display_row_source_slot_bounds_to_current_row,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowItemMeasurement, DisplayRowLayout,
    DisplayRowPosition, DisplayRowWriteMetrics, DisplayTabPolicy,
};
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_source::{
    BufferDisplayReplacementSource, BufferDisplayReplacementStringSource, BufferTextItemSource,
    DisplayItemSource, DisplayReplacementBox, DisplaySourceContext, LispStringSourceCursor,
};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_text::{DisplayTextFragment, DisplayTextStorage};
use crate::display_text_run_measurement::{
    ComplexTextRunAdvanceResolver, DisplayTextRunMeasurementPlan,
};
use crate::font_metrics::FontMetricsService;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use crate::unicode::decode_utf8;
use crate::window_output::{TextRowOutput, WindowOutputEmitter};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::buffer::{BufferId, CharLen, EmacsByteRange};
use neovm_core::emacs_core::Context;
#[cfg(test)]
use neovm_core::emacs_core::Value;
use std::collections::HashMap;

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

struct ResolvedFragmentAdvanceRenderPolicy {
    advance_px: f32,
}

impl ResolvedFragmentAdvanceRenderPolicy {
    fn new(advance_px: f32) -> Self {
        Self { advance_px }
    }

    fn measurement_for_text(&self, text: &str) -> DisplayRowItemMeasurement {
        DisplayRowItemMeasurement::TextRun(
            DisplayTextRunMeasurementPlan::from_resolved_fragment_advance(text, self.advance_px),
        )
    }
}

impl DisplayRowRenderPolicy for ResolvedFragmentAdvanceRenderPolicy {
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

struct NaturalDisplayRowAppendRenderPolicy;

impl DisplayRowRenderPolicy for NaturalDisplayRowAppendRenderPolicy {}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BufferTextFragmentAppendMeasurement {
    Natural,
    ResolvedAdvance { advance_px: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedBufferTextFragmentAdvance {
    advance_px: f32,
    append_measurement: BufferTextFragmentAppendMeasurement,
}

impl ResolvedBufferTextFragmentAdvance {
    fn natural(advance_px: f32) -> Self {
        Self {
            advance_px,
            append_measurement: BufferTextFragmentAppendMeasurement::Natural,
        }
    }

    fn resolved(advance_px: f32) -> Self {
        Self {
            advance_px,
            append_measurement: BufferTextFragmentAppendMeasurement::ResolvedAdvance { advance_px },
        }
    }

    pub(crate) fn advance_px(self) -> f32 {
        self.advance_px
    }

    fn append_measurement(self) -> BufferTextFragmentAppendMeasurement {
        self.append_measurement
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BufferTextFragmentAdvanceResolver {
    complex_run: ComplexTextRunAdvanceResolver,
}

enum BufferTextFragmentRenderPolicy {
    Natural(NaturalDisplayRowAppendRenderPolicy),
    Resolved(ResolvedFragmentAdvanceRenderPolicy),
}

impl BufferTextFragmentRenderPolicy {
    fn new(measurement: BufferTextFragmentAppendMeasurement) -> Self {
        match measurement {
            BufferTextFragmentAppendMeasurement::Natural => {
                Self::Natural(NaturalDisplayRowAppendRenderPolicy)
            }
            BufferTextFragmentAppendMeasurement::ResolvedAdvance { advance_px } => {
                Self::Resolved(ResolvedFragmentAdvanceRenderPolicy::new(advance_px))
            }
        }
    }
}

impl DisplayRowRenderPolicy for BufferTextFragmentRenderPolicy {
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

pub(crate) struct CurrentTextRowRenderOutcome {
    pub(crate) stop: DisplayRowRenderStop,
    pub(crate) source_slots: Vec<crate::display_row_builder::DisplayRowGlyphSlot>,
    pub(crate) end: DisplayRowPosition,
    pub(crate) row_height_px: f32,
    pub(crate) row_ascent_px: f32,
}

fn display_row_position_from_output_progress(
    progress: DisplayRowOutputProgress,
) -> DisplayRowPosition {
    DisplayRowPosition {
        x_px: progress.end_x,
        col: usize::try_from(progress.end_col.max(0)).unwrap_or(usize::MAX),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_display_item_source_into_current_text_row_and_emit<
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    face_resolver: &FaceResolver,
    face_ids: &mut FrameFaceIdAllocator,
    request: DisplayRowSourceRenderRequest<'_>,
    output: TextRowOutput,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome> {
    let role = request.role();
    let mut renderer = DisplayRowRenderer::new(font_metrics);
    let (result, row_height_px, row_ascent_px) = builder.with_current_row_mut(|row| {
        let mut context = DisplayRowRenderContext::new(
            face_resolver,
            evaluator.display_host.as_deref(),
            face_ids,
        );
        let result = renderer
            .render_display_item_source_row_fragment_step_from_request_into_row_with_policy(
                request,
                row,
                source,
                source_state,
                &mut context,
                render_policy,
            )?;
        Some((result, row.height_px, row.ascent_px))
    })??;
    let end = display_row_position_from_output_progress(result.progress);
    install_rendered_display_row_fragment_assets(
        builder,
        role,
        output.row,
        &result.faces,
        &result.media,
    );
    merge_display_row_source_slot_bounds_to_current_row(builder, &result.source_slots);
    let source_slots = result.source_slots;
    output_emitter.emit_text_source_slots(evaluator, output, &source_slots, end);
    Some(CurrentTextRowRenderOutcome {
        stop: result.stop,
        source_slots,
        end,
        row_height_px,
        row_ascent_px,
    })
}

#[allow(clippy::too_many_arguments)]
fn render_display_source_append_request_into_current_text_row_and_emit<
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    face_resolver: &FaceResolver,
    face_ids: &mut FrameFaceIdAllocator,
    request: DisplayRowSourceAppendRequest<'_>,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome> {
    let parts = request.into_render_parts();
    render_display_item_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        source,
        source_state,
        face_resolver,
        face_ids,
        parts.request,
        parts.output,
        render_policy,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_natural_display_source_append_request_into_current_text_row_and_emit<
    S: DisplayItemSource,
>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    face_resolver: &FaceResolver,
    face_ids: &mut FrameFaceIdAllocator,
    request: DisplayRowSourceAppendRequest<'_>,
) -> Option<CurrentTextRowRenderOutcome> {
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    render_display_source_append_request_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        source,
        source_state,
        face_resolver,
        face_ids,
        request,
        &mut render_policy,
    )
}

#[allow(clippy::too_many_arguments)]
fn measure_display_source_append_request_against_current_text_row<
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    face_resolver: &FaceResolver,
    face_ids: &mut FrameFaceIdAllocator,
    request: DisplayRowSourceAppendRequest<'_>,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome> {
    let parts = request.into_render_parts();
    let mut renderer = DisplayRowRenderer::new(font_metrics);
    let (result, row_height_px, row_ascent_px) = builder.with_current_row_mut(|row| {
        let mut scratch_row = row.clone();
        let mut context = DisplayRowRenderContext::new(
            face_resolver,
            evaluator.display_host.as_deref(),
            face_ids,
        );
        let result = renderer
            .render_display_item_source_row_fragment_step_from_request_into_row_with_policy(
                parts.request,
                &mut scratch_row,
                source,
                source_state,
                &mut context,
                render_policy,
            )?;
        Some((result, scratch_row.height_px, scratch_row.ascent_px))
    })??;
    let end = display_row_position_from_output_progress(result.progress);
    let source_slots = result.source_slots;
    Some(CurrentTextRowRenderOutcome {
        stop: result.stop,
        source_slots,
        end,
        row_height_px,
        row_ascent_px,
    })
}

fn display_row_append_progress_from_render_result(
    start: DisplayRowPosition,
    end: DisplayRowPosition,
    stop: DisplayRowRenderStop,
    slots: Vec<crate::display_row_builder::DisplayRowGlyphSlot>,
) -> DisplayRowAppendProgress {
    DisplayRowAppendProgress {
        start,
        end,
        metrics: DisplayRowWriteMetrics {
            width_px: (end.x_px - start.x_px).max(0.0),
            width_cols: end.col.saturating_sub(start.col),
        },
        status: match stop {
            DisplayRowRenderStop::SourceExhausted => DisplayRowAppendStatus::Complete,
            DisplayRowRenderStop::Clipped => DisplayRowAppendStatus::Clipped,
            DisplayRowRenderStop::RowBreak => DisplayRowAppendStatus::RowBreak,
        },
        slots,
    }
}

pub(crate) struct DisplayRowSourceAppendRequest<'face> {
    append_spec: DisplayRowAppendSpec,
    base_face_id: u32,
    base_face: &'face ResolvedFace,
}

struct DisplayRowSourceAppendRenderParts<'face> {
    request: DisplayRowSourceRenderRequest<'face>,
    output: TextRowOutput,
}

impl<'face> DisplayRowSourceAppendRequest<'face> {
    pub(crate) fn from_append_spec(
        append_spec: DisplayRowAppendSpec,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        Self {
            append_spec,
            base_face_id,
            base_face,
        }
    }

    fn into_render_parts(self) -> DisplayRowSourceAppendRenderParts<'face> {
        let request = self
            .append_spec
            .display_row_source_render_request(self.base_face_id, self.base_face);
        let output = self.append_spec.text_row_output();
        DisplayRowSourceAppendRenderParts { request, output }
    }
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
    item.face = RenderFaceRef::FaceId(request.base_face_id);
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(request.base_face_id.saturating_add(1));
    let start = request.append_spec.position;
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
    let slots = outcome.source_slots;
    let end = outcome.end;
    let progress = display_row_append_progress_from_render_result(start, end, outcome.stop, slots);
    Some((progress, end))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_lisp_string_fragment_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    fragment: DisplayTextFragment,
    source_id: u64,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> DisplayRowPosition {
    let DisplayTextStorage::LispString(text_value) = fragment.storage else {
        return position;
    };
    let Some(mut source) = crate::display_source::LispStringSourceCursor::new(
        source_id,
        text_value,
        RenderFaceRef::FaceId(base_face_id),
    ) else {
        return position;
    };
    let append_spec = frame.append_spec(position, base_face_id, DisplayRowAppendKind::SourceText);
    let request =
        DisplayRowSourceAppendRequest::from_append_spec(append_spec, base_face_id, base_face);
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
    outcome.end
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_lisp_string_source_append_to_text_row_and_emit(
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
    let append_spec = frame.append_spec(position, base_face_id, DisplayRowAppendKind::SourceText);
    let request =
        DisplayRowSourceAppendRequest::from_append_spec(append_spec, base_face_id, base_face);
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
    let append_spec = frame.append_spec(position, base_face_id, DisplayRowAppendKind::SourceText);
    let request =
        DisplayRowSourceAppendRequest::from_append_spec(append_spec, base_face_id, base_face);
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
    outcome.end
}

#[allow(clippy::too_many_arguments)]
fn buffer_text_fragment_source_item<B: LayoutBufferView + ?Sized>(
    fragment: DisplayTextFragment,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<(DisplayItem, DisplayRowAppendKind)> {
    let DisplayTextStorage::BufferSpan { start, end } = fragment.storage else {
        return None;
    };
    let DisplayOrigin::BufferText { charpos } = fragment.origin else {
        return None;
    };
    if start != charpos || end != start.add_len(CharLen::new(1)) {
        return None;
    }

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
pub(crate) fn append_resolved_buffer_text_fragment_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    fragment: DisplayTextFragment,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    resolved_advance: ResolvedBufferTextFragmentAdvance,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let (item, append_kind) =
        buffer_text_fragment_source_item(fragment, buffer_id, buffer, face_id)?;
    let append_spec = frame.append_spec(position, face_id, append_kind);
    let request = DisplayRowSourceAppendRequest::from_append_spec(append_spec, face_id, base_face);
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy =
        BufferTextFragmentRenderPolicy::new(resolved_advance.append_measurement());
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
    let progress = display_row_append_progress_from_render_result(
        position,
        outcome.end,
        outcome.stop,
        outcome.source_slots,
    );
    Some((progress, outcome.end))
}

#[allow(clippy::too_many_arguments)]
fn measure_buffer_text_fragment_append_progress_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    fragment: DisplayTextFragment,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<DisplayRowAppendProgress> {
    let (item, append_kind) =
        buffer_text_fragment_source_item(fragment, buffer_id, buffer, face_id)?;
    let append_spec = frame.append_spec(position, face_id, append_kind);
    let request = DisplayRowSourceAppendRequest::from_append_spec(append_spec, face_id, base_face);
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let mut render_policy =
        BufferTextFragmentRenderPolicy::new(BufferTextFragmentAppendMeasurement::Natural);
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
    Some(display_row_append_progress_from_render_result(
        position,
        outcome.end,
        outcome.stop,
        outcome.source_slots,
    ))
}

#[allow(clippy::too_many_arguments)]
fn measure_buffer_text_fragment_natural_advance_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    fragment: DisplayTextFragment,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<f32> {
    Some(
        measure_buffer_text_fragment_append_progress_to_text_row(
            builder,
            evaluator,
            font_metrics,
            fragment,
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
fn resolve_buffer_text_fragment_natural_advance_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    fragment: DisplayTextFragment,
    face_resolver: &FaceResolver,
    buffer_id: BufferId,
    buffer: &B,
    active_face_state: &DisplayRowActiveFaceState,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    ch: char,
    is_cluster_continuation: bool,
) -> f32 {
    if let Some(measured_width) = measure_buffer_text_fragment_natural_advance_to_text_row(
        builder,
        evaluator,
        font_metrics,
        fragment,
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

    fallback_buffer_text_fragment_natural_advance_to_text_row(
        font_metrics,
        active_face_state,
        &frame,
        position,
        ch,
        is_cluster_continuation,
    )
}

fn fallback_buffer_text_fragment_natural_advance_to_text_row(
    font_metrics: &mut Option<FontMetricsService>,
    active_face_state: &DisplayRowActiveFaceState,
    frame: &DisplayRowAppendFrame,
    position: DisplayRowPosition,
    ch: char,
    is_cluster_continuation: bool,
) -> f32 {
    if ch == '\t' {
        return frame
            .geometry
            .tab_policy
            .advance_from(position, frame.face_space_width)
            .pixel_width;
    }
    if is_cluster_continuation {
        return 0.0;
    }
    let char_cols = crate::composition::base_width_cols(ch) as usize;
    active_face_state.advance_for_columns(font_metrics, ch, char_cols)
}

impl BufferTextFragmentAdvanceResolver {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_to_text_row<B: LayoutBufferView + ?Sized>(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        evaluator: &mut Context,
        font_metrics: &mut Option<FontMetricsService>,
        text: &[u8],
        byte_idx: usize,
        fragment: DisplayTextFragment,
        face_resolver: &FaceResolver,
        buffer_id: BufferId,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        ch: char,
        is_cluster_continuation: bool,
    ) -> ResolvedBufferTextFragmentAdvance {
        if crate::composition::needs_complex_shaping(ch) {
            let mut policy =
                DisplayRowComplexTextRunAdvancePolicy::new(active_face_state, font_metrics);
            let advance_px = self.complex_run.advance_for_char(
                text,
                byte_idx,
                ch,
                is_cluster_continuation,
                &mut policy,
            );
            return ResolvedBufferTextFragmentAdvance::resolved(advance_px);
        }

        if ch == '\t' || is_cluster_continuation || !ch.is_ascii() {
            let advance_px = resolve_buffer_text_fragment_natural_advance_to_text_row(
                builder,
                evaluator,
                font_metrics,
                fragment,
                face_resolver,
                buffer_id,
                buffer,
                active_face_state,
                frame,
                position,
                ch,
                is_cluster_continuation,
            );
            return ResolvedBufferTextFragmentAdvance::natural(advance_px);
        }

        ResolvedBufferTextFragmentAdvance::natural(active_face_state.advance_for_columns(
            font_metrics,
            ch,
            1,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn append_buffer_display_item_fragment_to_text_row_and_emit<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    fragment: DisplayTextFragment,
    buffer: &B,
    buffer_id: BufferId,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_id: u32,
    item: BufferTextFragmentAppendItem,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let DisplayTextStorage::BufferSpan { start, end } = fragment.storage else {
        return None;
    };
    let DisplayOrigin::BufferText { charpos } = fragment.origin else {
        return None;
    };
    if start != charpos || end <= start {
        return None;
    }

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
    let append_spec = frame.append_spec(position, face_id, append_kind);
    let request = DisplayRowSourceAppendRequest::from_append_spec(append_spec, face_id, base_face);
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
    let progress = display_row_append_progress_from_render_result(
        position,
        outcome.end,
        outcome.stop,
        outcome.source_slots,
    );
    Some((progress, outcome.end))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextFragmentAppendItem {
    ControlChar { ch: char },
    SourceMappedText { text: Box<str> },
    Glyphless { ch: char, method: GlyphlessMethod },
}

impl BufferTextFragmentAppendItem {
    fn append_kind(&self) -> DisplayRowAppendKind {
        match self {
            Self::ControlChar { .. } => DisplayRowAppendKind::ControlChar,
            Self::SourceMappedText { .. } => DisplayRowAppendKind::SourceMappedText,
            Self::Glyphless { .. } => DisplayRowAppendKind::Glyphless,
        }
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_buffer_text_item_fragment_to_text_row_and_emit<
    B: LayoutBufferView + ?Sized,
>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    fragment: DisplayTextFragment,
    buffer: &B,
    buffer_id: BufferId,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_id: u32,
    item: BufferTextFragmentAppendItem,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    append_buffer_display_item_fragment_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        fragment,
        buffer,
        buffer_id,
        face_resolver,
        base_face,
        face_id,
        item,
        frame,
        position,
    )
}

pub(crate) struct DisplayReplacementStringItemMeasurer {
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayReplacementStringItemMeasurer {
    pub(crate) fn from_active_face_state(active_face_state: &DisplayRowActiveFaceState) -> Self {
        Self {
            active_face_state: active_face_state.clone(),
        }
    }
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
    let append_spec =
        frame.append_spec(position, face_id, DisplayRowAppendKind::DisplayReplacement);
    let request = DisplayRowSourceAppendRequest::from_append_spec(append_spec, face_id, base_face);
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
pub(crate) enum DisplayReplacementAppendItem {
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_display_replacement_item_to_text_row_and_emit(
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
pub(crate) fn append_display_replacement_string_source_to_text_row<S: DisplayItemSource>(
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
    let append_spec = frame.append_spec(
        position,
        fallback_face_id,
        DisplayRowAppendKind::DisplayReplacementString,
    );
    let request =
        DisplayRowSourceAppendRequest::from_append_spec(append_spec, fallback_face_id, base_face);
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
    outcome.end
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_display_replacement_string_fragment_to_text_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    fragment: DisplayTextFragment,
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
    let DisplayTextStorage::LispString(text_value) = fragment.storage else {
        return position;
    };
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

pub(crate) struct DisplayRowAppendOutput {
    pub(crate) row: usize,
    pub(crate) row_y: f32,
    pub(crate) glyph_y: f32,
    pub(crate) height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendPlacement {
    pub(crate) row: usize,
    pub(crate) y: f32,
    pub(crate) glyph_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendArea {
    pub(crate) content_x: f32,
    pub(crate) width: f32,
    pub(crate) text_width: f32,
    pub(crate) line_number_width: f32,
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

    pub(crate) fn frame(
        &self,
        placement: DisplayRowAppendPlacement,
        metrics: DisplayRowAppendMetrics,
    ) -> DisplayRowAppendFrame {
        DisplayRowAppendFrame::from_parts(placement, self.area, metrics, self.tab_policy.clone())
    }

    pub(crate) fn frame_for_active_face(
        &self,
        placement: DisplayRowAppendPlacement,
        active_face: &DisplayRowActiveFaceState,
        default_row_height: f32,
    ) -> DisplayRowAppendFrame {
        self.frame(
            placement,
            DisplayRowAppendMetrics::from_active_face_state(active_face, default_row_height),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendMetrics {
    pub(crate) height: f32,
    pub(crate) ascent: f32,
    pub(crate) char_width: f32,
    pub(crate) space_width: f32,
    pub(crate) default_row_height: f32,
}

impl DisplayRowAppendMetrics {
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
        Self {
            height,
            ascent,
            char_width: metrics.char_width,
            space_width: metrics.space_width,
            default_row_height,
        }
    }

    pub(crate) fn from_measured_face_metrics(
        metrics: DisplayRowMeasuredFaceMetrics,
        default_row_height: f32,
    ) -> Self {
        Self {
            height: metrics.row_height,
            ascent: metrics.ascent,
            char_width: metrics.char_width,
            space_width: metrics.space_width,
            default_row_height,
        }
    }
}

#[derive(Clone)]
pub(crate) struct DisplayRowAppendFrame {
    pub(crate) row: usize,
    pub(crate) glyph_y: f32,
    pub(crate) geometry: DisplayRowGeometry,
    pub(crate) default_row_height: f32,
    pub(crate) content_x: f32,
    pub(crate) text_width: f32,
    pub(crate) line_number_width: f32,
    pub(crate) face_space_width: f32,
}

impl DisplayRowAppendFrame {
    pub(crate) fn from_parts(
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

    pub(crate) fn from_geometry_state(
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        area: DisplayRowAppendArea,
        metrics: DisplayRowAppendMetrics,
        tab_policy: DisplayTabPolicy,
    ) -> Self {
        Self::from_parts(
            geometry.append_placement(glyph_y_offset),
            area,
            metrics,
            tab_policy,
        )
    }

    pub(crate) fn at(self, position: DisplayRowPosition, face_id: u32) -> DisplayRowAppendContext {
        DisplayRowAppendContext {
            row: self.row,
            glyph_y: self.glyph_y,
            x: position.x_px,
            col: position.col,
            geometry: self.geometry,
            default_row_height: self.default_row_height,
            content_x: self.content_x,
            text_width: self.text_width,
            line_number_width: self.line_number_width,
            face_space_width: self.face_space_width,
            face_id,
        }
    }

    pub(crate) fn append_spec(
        &self,
        position: DisplayRowPosition,
        face_id: u32,
        kind: DisplayRowAppendKind,
    ) -> DisplayRowAppendSpec {
        self.clone().at(position, face_id).append_spec(kind)
    }
}

pub(crate) struct DisplayRowAppendContext {
    pub(crate) row: usize,
    pub(crate) glyph_y: f32,
    pub(crate) x: f32,
    pub(crate) col: usize,
    pub(crate) geometry: DisplayRowGeometry,
    pub(crate) default_row_height: f32,
    pub(crate) content_x: f32,
    pub(crate) text_width: f32,
    pub(crate) line_number_width: f32,
    pub(crate) face_space_width: f32,
    pub(crate) face_id: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowAppendKind {
    SourceText,
    Tab,
    ControlChar,
    SourceMappedText,
    Glyphless,
    DisplayReplacement,
    DisplayReplacementString,
}

pub(crate) struct DisplayRowAppendSpec {
    pub(crate) geometry: DisplayRowGeometry,
    pub(crate) layout: DisplayRowLayout,
    pub(crate) position: DisplayRowPosition,
    pub(crate) max_x: f32,
    pub(crate) output: DisplayRowAppendOutput,
}

impl DisplayRowAppendContext {
    pub(crate) fn append_spec(&self, kind: DisplayRowAppendKind) -> DisplayRowAppendSpec {
        let char_width = match kind {
            DisplayRowAppendKind::Tab => self.face_space_width,
            DisplayRowAppendKind::DisplayReplacementString => self.face_space_width,
            DisplayRowAppendKind::SourceText => self.geometry.char_width,
            DisplayRowAppendKind::ControlChar => self.geometry.char_width,
            DisplayRowAppendKind::SourceMappedText => self.geometry.char_width,
            DisplayRowAppendKind::Glyphless => self.geometry.char_width,
            DisplayRowAppendKind::DisplayReplacement => self.geometry.char_width,
        };
        let max_x = match kind {
            DisplayRowAppendKind::Tab => f32::INFINITY,
            DisplayRowAppendKind::ControlChar => {
                self.content_x + (self.text_width - self.line_number_width)
            }
            DisplayRowAppendKind::SourceText => self.content_x + self.geometry.width,
            DisplayRowAppendKind::SourceMappedText => self.content_x + self.geometry.width,
            DisplayRowAppendKind::Glyphless => self.content_x + self.geometry.width,
            DisplayRowAppendKind::DisplayReplacement => self.content_x + self.geometry.width,
            DisplayRowAppendKind::DisplayReplacementString => self.content_x + self.geometry.width,
        };
        let output_height = match kind {
            DisplayRowAppendKind::SourceText => self.geometry.height,
            DisplayRowAppendKind::Glyphless => self.geometry.height,
            DisplayRowAppendKind::DisplayReplacement => self.geometry.height,
            DisplayRowAppendKind::DisplayReplacementString => self.geometry.height,
            DisplayRowAppendKind::Tab => self.default_row_height,
            DisplayRowAppendKind::ControlChar => self.default_row_height,
            DisplayRowAppendKind::SourceMappedText => self.default_row_height,
        };

        DisplayRowAppendSpec {
            geometry: DisplayRowGeometry {
                char_width,
                ..self.geometry.clone()
            },
            layout: self.geometry.to_layout(
                GlyphRowRole::Text,
                char_width,
                self.geometry.ascent,
                RenderFaceRef::FaceId(self.face_id),
                HashMap::new(),
            ),
            position: DisplayRowPosition {
                x_px: self.x,
                col: self.col,
            },
            max_x,
            output: DisplayRowAppendOutput {
                row: self.row,
                row_y: self.geometry.y,
                glyph_y: self.glyph_y,
                height: output_height,
            },
        }
    }
}

impl DisplayRowAppendSpec {
    pub(crate) fn display_row_source_render_request<'face>(
        &self,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> DisplayRowSourceRenderRequest<'face> {
        DisplayRowSourceRenderRequest::whole_row(
            self.geometry.clone(),
            base_face_id,
            base_face,
            self.layout.role,
        )
        .with_render_bounds(DisplayRowRenderBounds {
            start: self.position,
            max_x_px: self.max_x,
        })
    }

    pub(crate) fn text_row_output(&self) -> TextRowOutput {
        TextRowOutput {
            row: self.output.row,
            row_y: self.output.row_y,
            glyph_y: self.output.glyph_y,
            height: self.output.height,
        }
    }
}

pub(crate) fn append_synthetic_text_to_display_row(
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
    let append_spec = frame.append_spec(position, face_id, DisplayRowAppendKind::SourceText);
    let request = DisplayRowSourceAppendRequest::from_append_spec(append_spec, face_id, base_face);
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
