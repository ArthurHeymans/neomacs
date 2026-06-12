use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplaySourcePosition, DisplayTextRun, RenderFaceRef, SourceSpan,
};
#[cfg(test)]
use crate::display_item::{DisplayMediaReplacement, DisplayMediaReplacementKind};
use crate::display_origin::DisplayOrigin;
#[cfg(test)]
use crate::display_row::DisplayRowSourceWalker;
#[cfg(test)]
use crate::display_row::RenderedDisplayRow;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_current_row;
use crate::display_row::{
    DisplayRowGeometry, DisplayRowOutputProgress, DisplayRowRenderBounds,
    DisplayRowRenderClipBehavior, DisplayRowRenderPolicy, DisplayRowRenderStop, DisplayRowRenderer,
    DisplayRowSourceState, DisplayRowSpec, install_rendered_display_row_fragment_assets,
    merge_display_row_source_slot_bounds_to_current_row,
};
use crate::display_row_builder::{
    DisplayGlyphMeasurer, DisplayRowAppendProgress, DisplayRowAppendStatus,
    DisplayRowItemMeasurement, DisplayRowItemMeasurer, DisplayRowPosition, DisplayRowWriteMetrics,
    DisplayTabPolicy, FixedGlyphAdvance,
};
#[cfg(test)]
use crate::display_row_builder::{DisplayRowAppendCursor, DisplayRowLayout};
use crate::display_source::{BufferTextItemSource, DisplayItemSource, DisplaySourceContext};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_text::{DisplayTextFragment, DisplayTextStorage};
use crate::font_metrics::FontMetricsService;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use crate::unicode::decode_utf8;
#[cfg(test)]
use crate::window_output::DisplayProgressSink;
use crate::window_output::{TextRowOutput, WindowOutputEmitter};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::buffer::{BufferId, CharLen, CharPos0, EmacsByteRange};
use neovm_core::emacs_core::Context;
#[cfg(test)]
use neovm_core::emacs_core::Value;
#[cfg(test)]
use neovm_core::emacs_core::eval::DisplayHost;
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

struct MeasuredDisplayRowRenderPolicy<'a> {
    glyph_measurer: &'a mut dyn DisplayGlyphMeasurer,
}

impl DisplayRowRenderPolicy for MeasuredDisplayRowRenderPolicy<'_> {
    fn measurement_for<'a>(
        &'a mut self,
        _item: &DisplayItem,
        _face_id: u32,
    ) -> DisplayRowItemMeasurement<'a> {
        DisplayRowItemMeasurement::Measured(&mut *self.glyph_measurer)
    }
}

struct NaturalDisplayRowAppendRenderPolicy;

impl DisplayRowRenderPolicy for NaturalDisplayRowAppendRenderPolicy {}

#[cfg(test)]
pub(crate) fn emit_text_progress_slots(
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    progress: &DisplayRowAppendProgress,
    row: usize,
    row_y: f32,
    glyph_y: f32,
    height: f32,
) {
    output_emitter.emit_text_progress(
        evaluator,
        TextRowOutput {
            row,
            row_y,
            glyph_y,
            height,
        },
        progress,
    );
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
pub(crate) fn render_display_item_source_into_current_text_row_and_emit<
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
    next_face_id: &mut u32,
    row_spec: DisplayRowSpec<'_>,
    output: TextRowOutput,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome> {
    let role = row_spec.role;
    let mut renderer = DisplayRowRenderer::new(font_metrics);
    let (result, row_height_px, row_ascent_px) = builder.with_current_row_mut(|row| {
        let result = renderer.render_display_item_source_row_fragment_step_into_row_with_policy(
            row_spec,
            row,
            source,
            source_state,
            face_resolver,
            evaluator.display_host.as_deref(),
            next_face_id,
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
pub(crate) fn render_natural_display_item_source_into_current_text_row_and_emit<
    S: DisplayItemSource,
>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    face_resolver: &FaceResolver,
    next_face_id: &mut u32,
    row_spec: DisplayRowSpec<'_>,
    output: TextRowOutput,
) -> Option<CurrentTextRowRenderOutcome> {
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    render_display_item_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        source,
        source_state,
        face_resolver,
        next_face_id,
        row_spec,
        output,
        &mut render_policy,
    )
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

struct DisplayRowFragmentAppendRequest<'face> {
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    max_x_px: f32,
    output_height: f32,
    base_face_id: u32,
    base_face: &'face ResolvedFace,
}

impl<'face> DisplayRowFragmentAppendRequest<'face> {
    fn for_frame(
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        let max_x_px = frame.content_x + frame.geometry.width;
        let output_height = frame.geometry.height;
        Self {
            frame,
            position,
            max_x_px,
            output_height,
            base_face_id,
            base_face,
        }
    }

    fn with_max_x_px(mut self, max_x_px: f32) -> Self {
        self.max_x_px = max_x_px;
        self
    }

    fn with_output_height(mut self, output_height: f32) -> Self {
        self.output_height = output_height;
        self
    }
}

#[allow(clippy::too_many_arguments)]
fn append_single_display_item_fragment_to_text_row_and_emit<P: DisplayRowRenderPolicy>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    mut item: DisplayItem,
    face_resolver: &FaceResolver,
    request: DisplayRowFragmentAppendRequest<'_>,
    render_policy: &mut P,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    item.face = RenderFaceRef::FaceId(request.base_face_id);
    let mut source = SingleDisplayItemSource::new(item);
    let row_spec = DisplayRowSpec {
        geometry: request.frame.geometry.clone(),
        render_bounds: DisplayRowRenderBounds {
            start: request.position,
            max_x_px: request.max_x_px,
        },
        base_face_id: request.base_face_id,
        base_face: request.base_face,
        role: GlyphRowRole::Text,
        symbol_values: HashMap::new(),
    };
    let mut source_state = DisplayRowSourceState::default();
    let mut font_metrics = None;
    let mut next_face_id = request.base_face_id.saturating_add(1);
    let output = TextRowOutput {
        row: request.frame.row,
        row_y: request.frame.geometry.y,
        glyph_y: request.frame.glyph_y,
        height: request.output_height,
    };
    let outcome = render_display_item_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        &mut font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        &mut next_face_id,
        row_spec,
        output,
        render_policy,
    )?;
    let slots = outcome.source_slots;
    let end = outcome.end;
    let progress =
        display_row_append_progress_from_render_result(request.position, end, outcome.stop, slots);
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
    current_face_id: &mut u32,
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
    let row_spec = DisplayRowSpec {
        geometry: frame.geometry.clone(),
        render_bounds: DisplayRowRenderBounds {
            start: position,
            max_x_px: frame.content_x + frame.geometry.width,
        },
        base_face_id,
        base_face,
        role: GlyphRowRole::Text,
        symbol_values: HashMap::new(),
    };
    let mut source_state = DisplayRowSourceState::default();
    let Some(outcome) = render_natural_display_item_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        current_face_id,
        row_spec,
        TextRowOutput {
            row: frame.row,
            row_y: frame.geometry.y,
            glyph_y: frame.glyph_y,
            height: frame.geometry.height,
        },
    ) else {
        return position;
    };
    outcome.end
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
    current_face_id: &mut u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> DisplayRowPosition {
    let Some(source) = crate::display_source::LispStringSourceCursor::new(
        source_id,
        text_value,
        RenderFaceRef::FaceId(base_face_id),
    ) else {
        return position;
    };
    let mut policy = NaturalDisplaySourceAppendPolicy;
    append_display_item_source_to_text_row(
        builder,
        output_emitter,
        evaluator,
        source,
        face_resolver,
        base_face,
        base_face_id,
        current_face_id,
        frame,
        position,
        &mut policy,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_buffer_text_fragment_to_text_row<B: LayoutBufferView + ?Sized>(
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
    advance: f32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
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

    append_buffer_text_char_to_text_row(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        face_resolver,
        base_face,
        buffer_id,
        buffer,
        start,
        face_id,
        ch,
        advance,
        frame,
        position,
    )
}

pub(crate) fn append_buffer_text_char_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    font_metrics: &mut Option<FontMetricsService>,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    buffer_id: BufferId,
    buffer: &B,
    char_pos: CharPos0,
    face_id: u32,
    ch: char,
    advance: f32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let byte_start = buffer.layout_char_pos_to_emacs_byte_pos(char_pos);
    let byte_end = buffer.layout_char_pos_to_emacs_byte_pos(char_pos.add_len(CharLen::new(1)));
    let item = BufferTextItemSource::single_char(buffer_id, char_pos, byte_start, byte_end).item(
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(ch.to_string())),
    );
    let mut geometry = frame.geometry.clone();
    let mut max_x_px = frame.content_x + frame.geometry.width;
    let mut output_height = frame.geometry.height;
    if ch == '\t' {
        geometry.char_width = frame.face_space_width;
        max_x_px = f32::INFINITY;
        output_height = frame.default_row_height;
    }
    let row_spec = DisplayRowSpec {
        geometry,
        render_bounds: DisplayRowRenderBounds {
            start: position,
            max_x_px,
        },
        base_face_id: face_id,
        base_face,
        role: GlyphRowRole::Text,
        symbol_values: HashMap::new(),
    };
    let mut source = SingleDisplayItemSource::new(item);
    let mut source_state = DisplayRowSourceState::default();
    let mut next_face_id = face_id.saturating_add(1);
    let mut measurer = FixedGlyphAdvance::new(ch, face_id, advance);
    let mut render_policy = MeasuredDisplayRowRenderPolicy {
        glyph_measurer: &mut measurer,
    };
    let output = TextRowOutput {
        row: frame.row,
        row_y: frame.geometry.y,
        glyph_y: frame.glyph_y,
        height: output_height,
    };
    let outcome = render_display_item_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        &mut next_face_id,
        row_spec,
        output,
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
pub(crate) fn append_buffer_text_item_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    source: BufferTextItemSource,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_id: u32,
    kind: DisplayItemKind,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let max_x_px = if matches!(kind, DisplayItemKind::ControlChar { .. }) {
        frame.content_x + (frame.text_width - frame.line_number_width)
    } else {
        frame.content_x + frame.geometry.width
    };
    let output_height = if matches!(
        kind,
        DisplayItemKind::ControlChar { .. } | DisplayItemKind::SourceMappedText(_)
    ) {
        frame.default_row_height
    } else {
        frame.geometry.height
    };
    let item = source.item(RenderFaceRef::FaceId(face_id), kind);
    let request = DisplayRowFragmentAppendRequest::for_frame(frame, position, face_id, base_face)
        .with_max_x_px(max_x_px)
        .with_output_height(output_height);
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    append_single_display_item_fragment_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        item,
        face_resolver,
        request,
        &mut render_policy,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_buffer_text_item_fragment_to_text_row_and_emit<
    B: LayoutBufferView + ?Sized,
>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    fragment: DisplayTextFragment,
    buffer: &B,
    buffer_id: BufferId,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_id: u32,
    kind: DisplayItemKind,
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

    append_buffer_text_item_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        source,
        face_resolver,
        base_face,
        face_id,
        kind,
        frame,
        position,
    )
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowAppendClipBehavior {
    Stop,
}

#[cfg(test)]
impl DisplayRowAppendClipBehavior {
    fn stops_on(self, progress: &DisplayRowAppendProgress) -> bool {
        self == Self::Stop
            && progress.status == crate::display_row_builder::DisplayRowAppendStatus::Clipped
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowSourceAppendDecision {
    Append {
        kind: DisplayRowAppendKind,
        on_clipped: DisplayRowAppendClipBehavior,
    },
    #[cfg(test)]
    Skip,
    #[cfg(test)]
    Stop,
}

#[cfg(test)]
pub(crate) trait DisplayRowSourceAppendPolicy {
    fn decision_for(&mut self, item: &DisplayItem) -> DisplayRowSourceAppendDecision;
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
struct DisplayItemSourceAppendResult {
    position: DisplayRowPosition,
    last_progress: Option<DisplayRowAppendProgress>,
}

#[cfg(test)]
struct NaturalDisplaySourceAppendPolicy;

#[cfg(test)]
impl DisplayRowSourceAppendPolicy for NaturalDisplaySourceAppendPolicy {
    fn decision_for(&mut self, item: &DisplayItem) -> DisplayRowSourceAppendDecision {
        let Some(kind) = DisplayRowAppendKind::from_display_item_kind(&item.kind) else {
            return DisplayRowSourceAppendDecision::Skip;
        };
        DisplayRowSourceAppendDecision::Append {
            kind,
            on_clipped: DisplayRowAppendClipBehavior::Stop,
        }
    }
}

struct DisplayReplacementStringRenderPolicy<'a, M> {
    item_measurer: &'a mut M,
}

impl<M: DisplayRowItemMeasurer> DisplayRowRenderPolicy
    for DisplayReplacementStringRenderPolicy<'_, M>
{
    fn stop_before_item(&mut self, item: &DisplayItem) -> bool {
        matches!(item.kind, DisplayItemKind::RowBreak(_))
    }

    fn measurement_for<'a>(
        &'a mut self,
        item: &DisplayItem,
        face_id: u32,
    ) -> DisplayRowItemMeasurement<'a> {
        self.item_measurer.measurement_for(item, face_id)
    }

    fn clipped_behavior(&mut self, item: &DisplayItem) -> DisplayRowRenderClipBehavior {
        if matches!(item.kind, DisplayItemKind::SourceMappedText(_)) {
            DisplayRowRenderClipBehavior::Stop
        } else {
            DisplayRowRenderClipBehavior::Continue
        }
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn append_display_item_stream_to_text_row<P: DisplayRowSourceAppendPolicy>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    mut position: DisplayRowPosition,
    policy: &mut P,
    mut next_item: impl FnMut(&mut GlyphMatrixBuilder, Option<&dyn DisplayHost>) -> Option<DisplayItem>,
) -> DisplayItemSourceAppendResult {
    let mut last_progress = None;
    loop {
        let item = next_item(builder, evaluator.display_host.as_deref());
        let Some(mut item) = item else {
            break;
        };
        let (kind, on_clipped) = match policy.decision_for(&item) {
            DisplayRowSourceAppendDecision::Append { kind, on_clipped } => (kind, on_clipped),
            #[cfg(test)]
            DisplayRowSourceAppendDecision::Skip => continue,
            #[cfg(test)]
            DisplayRowSourceAppendDecision::Stop => {
                break;
            }
        };
        let face_id = render_face_ref_id(item.face, fallback_face_id);
        item.face = RenderFaceRef::FaceId(face_id);
        let append_spec = frame.clone().at(position, face_id).append_spec(kind);
        let Some((progress, next_position)) = append_display_row_spec_item_and_emit(
            builder,
            output_emitter,
            evaluator,
            append_spec,
            item,
        ) else {
            break;
        };
        position = next_position;
        let stop_on_clipped = on_clipped.stops_on(&progress);
        last_progress = Some(progress);
        if stop_on_clipped {
            break;
        }
    }
    DisplayItemSourceAppendResult {
        position,
        last_progress,
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn append_display_item_source_to_text_row<
    S: DisplayItemSource,
    P: DisplayRowSourceAppendPolicy,
>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    source: S,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    current_face_id: &mut u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    policy: &mut P,
) -> DisplayRowPosition {
    let mut source = DisplayRowSourceWalker::new(source);
    let fallback_char_width = frame.geometry.char_width;
    let fallback_ascent = frame.geometry.ascent;
    let fallback_row_height = frame.geometry.height;
    append_display_item_stream_to_text_row(
        builder,
        output_emitter,
        evaluator,
        fallback_face_id,
        frame,
        position,
        policy,
        |builder, display_host| {
            let mut step = source.next_step(
                face_resolver,
                base_face,
                fallback_face_id,
                current_face_id,
                display_host,
                fallback_char_width,
                fallback_ascent,
                fallback_row_height,
            )?;
            apply_pending_display_source_faces(builder, &mut step.pending_faces);
            Some(step.item)
        },
    )
    .position
}

#[cfg(test)]
pub(crate) fn append_display_item_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    mut item: DisplayItem,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let kind = DisplayRowAppendKind::from_display_item_kind(&item.kind)?;
    let face_id = render_face_ref_id(item.face, fallback_face_id);
    item.face = RenderFaceRef::FaceId(face_id);
    let append_spec = frame.at(position, face_id).append_spec(kind);
    append_display_row_spec_item_and_emit(builder, output_emitter, evaluator, append_spec, item)
}

#[cfg(test)]
pub(crate) fn append_display_replacement_item_to_text_row(
    builder: &mut GlyphMatrixBuilder,
    mut item: DisplayItem,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let face_id = render_face_ref_id(item.face, fallback_face_id);
    item.face = RenderFaceRef::FaceId(face_id);
    let append_spec = frame
        .at(position, face_id)
        .append_spec(DisplayRowAppendKind::DisplayReplacement);
    append_display_row_spec_item(builder, &append_spec, item)
}

pub(crate) fn append_display_replacement_item_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    item: DisplayItem,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let face_id = render_face_ref_id(item.face, fallback_face_id);
    let request = DisplayRowFragmentAppendRequest::for_frame(frame, position, face_id, base_face);
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    append_single_display_item_fragment_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        item,
        face_resolver,
        request,
        &mut render_policy,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_display_replacement_string_source_to_text_row<S: DisplayItemSource>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    mut source: S,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    fallback_face_id: u32,
    current_face_id: &mut u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    item_measurer: &mut impl DisplayRowItemMeasurer,
) -> DisplayRowPosition {
    let mut geometry = frame.geometry.clone();
    geometry.char_width = frame.face_space_width;
    let row_spec = DisplayRowSpec {
        geometry,
        render_bounds: DisplayRowRenderBounds {
            start: position,
            max_x_px: frame.content_x + frame.geometry.width,
        },
        base_face_id: fallback_face_id,
        base_face,
        role: GlyphRowRole::Text,
        symbol_values: HashMap::new(),
    };
    let mut source_state = DisplayRowSourceState::default();
    let mut render_policy = DisplayReplacementStringRenderPolicy { item_measurer };
    let mut font_metrics = None;
    let Some(outcome) = render_display_item_source_into_current_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        &mut font_metrics,
        &mut source,
        &mut source_state,
        face_resolver,
        current_face_id,
        row_spec,
        TextRowOutput {
            row: frame.row,
            row_y: frame.geometry.y,
            glyph_y: frame.glyph_y,
            height: frame.geometry.height,
        },
        &mut render_policy,
    ) else {
        return position;
    };
    outcome.end
}

#[cfg(test)]
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendMetrics {
    pub(crate) height: f32,
    pub(crate) ascent: f32,
    pub(crate) char_width: f32,
    pub(crate) space_width: f32,
    pub(crate) default_row_height: f32,
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

    #[cfg(test)]
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
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
impl DisplayRowAppendKind {
    pub(crate) fn from_display_item_kind(kind: &DisplayItemKind) -> Option<Self> {
        match kind {
            DisplayItemKind::TextRun(_) => Some(Self::SourceText),
            DisplayItemKind::SourceMappedText(_) => Some(Self::SourceMappedText),
            DisplayItemKind::ControlChar { .. } => Some(Self::ControlChar),
            DisplayItemKind::Glyphless(_) => Some(Self::Glyphless),
            DisplayItemKind::Stretch(_)
            | DisplayItemKind::Image(_)
            | DisplayItemKind::Video(_)
            | DisplayItemKind::Xwidget(_) => Some(Self::DisplayReplacement),
            DisplayItemKind::RowBreak(_)
            | DisplayItemKind::CursorAnchor(_)
            | DisplayItemKind::HitTestAnchor(_) => None,
        }
    }
}

#[cfg(test)]
pub(crate) struct DisplayRowAppendSpec {
    pub(crate) layout: DisplayRowLayout,
    pub(crate) position: DisplayRowPosition,
    pub(crate) max_x: f32,
    pub(crate) output: DisplayRowAppendOutput,
}

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn append_display_row_item(
    builder: &mut GlyphMatrixBuilder,
    layout: &DisplayRowLayout,
    position: DisplayRowPosition,
    max_x: f32,
    item: DisplayItem,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let mut append_cursor = DisplayRowAppendCursor::new(position, max_x);
    let progress = append_cursor.append_item_to_current_matrix_row(builder, layout, item)?;
    let position = append_cursor.position();
    Some((progress, position))
}

#[cfg(test)]
pub(crate) fn append_display_row_spec_item(
    builder: &mut GlyphMatrixBuilder,
    spec: &DisplayRowAppendSpec,
    item: DisplayItem,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    match DisplayMediaReplacement::from_item_kind(&item.kind) {
        Some(media) => append_media_display_row_spec_item(builder, spec, item, media),
        None => append_display_row_item(builder, &spec.layout, spec.position, spec.max_x, item),
    }
}

#[cfg(test)]
fn append_media_display_row_spec_item(
    builder: &mut GlyphMatrixBuilder,
    spec: &DisplayRowAppendSpec,
    item: DisplayItem,
    media: DisplayMediaReplacement,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let (progress, position) = append_display_row_item(
        builder,
        &spec.layout,
        spec.position,
        spec.max_x,
        media.replacement_item(item),
    )?;
    if progress.status == crate::display_row_builder::DisplayRowAppendStatus::Complete
        && progress.metrics.width_px > 0.0
    {
        install_media_replacement(builder, spec, &progress, media);
    }
    Some((progress, position))
}

#[cfg(test)]
fn install_media_replacement(
    builder: &mut GlyphMatrixBuilder,
    spec: &DisplayRowAppendSpec,
    progress: &DisplayRowAppendProgress,
    media: DisplayMediaReplacement,
) {
    match media.kind {
        DisplayMediaReplacementKind::Image { image_id } => builder.push_current_window_image(
            spec.layout.role,
            display_slot_row(spec.output.row),
            display_slot_col(progress.start.col),
            image_id,
            progress.start.x_px,
            spec.output.glyph_y,
            media.width,
            media.height,
        ),
        DisplayMediaReplacementKind::Video {
            video_id,
            loop_count,
            autoplay,
        } => builder.push_current_window_video(
            spec.layout.role,
            display_slot_row(spec.output.row),
            display_slot_col(progress.start.col),
            video_id,
            progress.start.x_px,
            spec.output.glyph_y,
            media.width,
            media.height,
            loop_count,
            autoplay,
        ),
        DisplayMediaReplacementKind::Xwidget { xwidget_id } => builder.push_current_window_xwidget(
            spec.layout.role,
            display_slot_row(spec.output.row),
            display_slot_col(progress.start.col),
            xwidget_id,
            progress.start.x_px,
            spec.output.glyph_y,
            media.width,
            media.height,
        ),
    }
}

#[cfg(test)]
fn display_slot_row(row: usize) -> u32 {
    row.min(u32::MAX as usize) as u32
}

#[cfg(test)]
fn display_slot_col(col: usize) -> u16 {
    col.min(usize::from(u16::MAX)) as u16
}

#[cfg(test)]
pub(crate) fn append_display_row_spec_item_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    spec: DisplayRowAppendSpec,
    item: DisplayItem,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let (progress, position) = append_display_row_spec_item(builder, &spec, item)?;
    emit_text_progress_slots(
        output_emitter,
        evaluator,
        &progress,
        spec.output.row,
        spec.output.row_y,
        spec.output.glyph_y,
        spec.output.height,
    );
    Some((progress, position))
}

pub(crate) fn append_synthetic_text_to_display_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source_id: u64,
    text: impl Into<Box<str>>,
    face_id: u32,
    glyph_measurer: Option<&mut dyn DisplayGlyphMeasurer>,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let item = synthetic_display_text_item(source_id, text, face_id);
    let request = DisplayRowFragmentAppendRequest::for_frame(frame, position, face_id, base_face);
    match glyph_measurer {
        Some(measurer) => {
            let mut render_policy = MeasuredDisplayRowRenderPolicy {
                glyph_measurer: measurer,
            };
            append_single_display_item_fragment_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                item,
                face_resolver,
                request,
                &mut render_policy,
            )
        }
        None => {
            let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
            append_single_display_item_fragment_to_text_row_and_emit(
                builder,
                output_emitter,
                evaluator,
                item,
                face_resolver,
                request,
                &mut render_policy,
            )
        }
    }
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
