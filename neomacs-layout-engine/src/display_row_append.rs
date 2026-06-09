use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayMediaReplacement, DisplayMediaReplacementKind,
    DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_row::{DisplayRowGeometry, insert_resolved_display_row_face};
use crate::display_row_builder::{
    DisplayGlyphMeasurer, DisplayRowAppendCursor, DisplayRowAppendProgress, DisplayRowLayout,
    DisplayRowPosition, DisplayTabPolicy, FixedGlyphAdvance,
};
use crate::display_source::{
    BufferTextItemSource, DisplayItemFaceResolver, DisplayItemSource, DisplaySourceContext,
};
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use crate::window_output::{DisplayProgressSink, TextRowOutput, WindowOutputEmitter};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::buffer::{BufferId, CharLen, CharPos0};
use neovm_core::emacs_core::{Context, Value};
use std::collections::HashMap;

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

struct LayoutDisplaySourceFaceResolver<'a> {
    face_resolver: &'a FaceResolver,
    base_face: &'a ResolvedFace,
    face_cache: &'a mut HashMap<Value, u32>,
    current_face_id: &'a mut u32,
    pending_faces: &'a mut Vec<PendingLayoutDisplayFace>,
}

#[derive(Clone, Debug)]
struct PendingLayoutDisplayFace {
    face_id: u32,
    resolved: ResolvedFace,
}

fn apply_pending_display_source_faces(
    builder: &mut GlyphMatrixBuilder,
    pending_faces: &mut Vec<PendingLayoutDisplayFace>,
) {
    for pending in pending_faces.drain(..) {
        insert_resolved_display_row_face(builder, pending.face_id, &pending.resolved, None);
    }
}

fn next_layout_display_source_item(
    builder: &mut GlyphMatrixBuilder,
    source: &mut impl DisplayItemSource,
    face_resolver: &FaceResolver,
    base_face: &ResolvedFace,
    face_cache: &mut HashMap<Value, u32>,
    current_face_id: &mut u32,
) -> Option<DisplayItem> {
    let mut pending_faces = Vec::new();
    let item = {
        let mut resolver = LayoutDisplaySourceFaceResolver {
            face_resolver,
            base_face,
            face_cache,
            current_face_id,
            pending_faces: &mut pending_faces,
        };
        let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);
        source.next_item(&mut context)
    };
    apply_pending_display_source_faces(builder, &mut pending_faces);
    item
}

pub(crate) struct DisplayItemSourceWalker<S> {
    source: S,
    face_cache: HashMap<Value, u32>,
}

impl<S> DisplayItemSourceWalker<S> {
    pub(crate) fn new(source: S) -> Self {
        Self {
            source,
            face_cache: HashMap::new(),
        }
    }
}

impl<S: DisplayItemSource> DisplayItemSourceWalker<S> {
    pub(crate) fn next_item(
        &mut self,
        builder: &mut GlyphMatrixBuilder,
        face_resolver: &FaceResolver,
        base_face: &ResolvedFace,
        current_face_id: &mut u32,
    ) -> Option<DisplayItem> {
        next_layout_display_source_item(
            builder,
            &mut self.source,
            face_resolver,
            base_face,
            &mut self.face_cache,
            current_face_id,
        )
    }
}

impl DisplayItemFaceResolver for LayoutDisplaySourceFaceResolver<'_> {
    fn resolve_face_ref(&mut self, base: RenderFaceRef, face_value: Value) -> RenderFaceRef {
        if let Some(face_id) = self.face_cache.get(&face_value) {
            return RenderFaceRef::FaceId(*face_id);
        }
        let Some(resolved) = self
            .face_resolver
            .resolve_face_value_over(self.base_face, &face_value)
        else {
            return base;
        };

        let face_id = *self.current_face_id;
        *self.current_face_id += 1;
        self.face_cache.insert(face_value, face_id);
        self.pending_faces
            .push(PendingLayoutDisplayFace { face_id, resolved });
        RenderFaceRef::FaceId(face_id)
    }
}

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

pub(crate) fn append_buffer_text_char_to_text_row<B: LayoutBufferView + ?Sized>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    buffer_id: BufferId,
    buffer: &B,
    char_pos: CharPos0,
    face_id: u32,
    ch: char,
    advance: f32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let mut source = crate::display_source::BufferTextSourceCursor::new(
        buffer_id,
        buffer,
        char_pos,
        char_pos.add_len(CharLen::new(1)),
        RenderFaceRef::FaceId(face_id),
    );
    let mut context = DisplaySourceContext::empty();
    let mut measurer = FixedGlyphAdvance::new(ch, face_id, advance);
    let mut policy = MeasuredDisplayItemAppendPolicy {
        kind: if ch == '\t' {
            DisplayRowAppendKind::Tab
        } else {
            DisplayRowAppendKind::SourceText
        },
        measurer: &mut measurer,
    };
    let result = append_display_item_stream_to_text_row(
        builder,
        output_emitter,
        evaluator,
        face_id,
        frame,
        position,
        &mut policy,
        |_builder| source.next_item(&mut context),
    );
    result
        .last_progress
        .map(|progress| (progress, result.position))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_buffer_text_item_to_text_row_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    source: BufferTextItemSource,
    face_id: u32,
    kind: DisplayItemKind,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let item = source.item(RenderFaceRef::FaceId(face_id), kind);
    append_display_item_to_text_row_and_emit(
        builder,
        output_emitter,
        evaluator,
        item,
        face_id,
        frame,
        position,
    )
}

pub(crate) enum DisplayRowAppendMeasurement<'a> {
    Default,
    Measured(&'a mut dyn DisplayGlyphMeasurer),
}

pub(crate) trait DisplayRowItemMeasurer {
    fn measurement_for<'a>(
        &'a mut self,
        item: &DisplayItem,
        face_id: u32,
    ) -> DisplayRowAppendMeasurement<'a>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowAppendClipBehavior {
    Stop,
    Continue,
}

impl DisplayRowAppendClipBehavior {
    fn stops_on(self, progress: &DisplayRowAppendProgress) -> bool {
        self == Self::Stop
            && progress.status == crate::display_row_builder::DisplayRowAppendStatus::Clipped
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowSourceAppendDecision {
    Append {
        kind: DisplayRowAppendKind,
        on_clipped: DisplayRowAppendClipBehavior,
    },
    Skip,
    Stop,
}

pub(crate) trait DisplayRowSourceAppendPolicy {
    fn decision_for(&mut self, item: &DisplayItem) -> DisplayRowSourceAppendDecision;

    fn measurement_for<'a>(
        &'a mut self,
        _item: &DisplayItem,
        _face_id: u32,
    ) -> DisplayRowAppendMeasurement<'a> {
        DisplayRowAppendMeasurement::Default
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DisplayItemSourceAppendResult {
    position: DisplayRowPosition,
    last_progress: Option<DisplayRowAppendProgress>,
}

struct NaturalDisplaySourceAppendPolicy;

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

struct MeasuredDisplayItemAppendPolicy<'a> {
    kind: DisplayRowAppendKind,
    measurer: &'a mut dyn DisplayGlyphMeasurer,
}

impl DisplayRowSourceAppendPolicy for MeasuredDisplayItemAppendPolicy<'_> {
    fn decision_for(&mut self, _item: &DisplayItem) -> DisplayRowSourceAppendDecision {
        DisplayRowSourceAppendDecision::Append {
            kind: self.kind,
            on_clipped: DisplayRowAppendClipBehavior::Stop,
        }
    }

    fn measurement_for<'a>(
        &'a mut self,
        _item: &DisplayItem,
        _face_id: u32,
    ) -> DisplayRowAppendMeasurement<'a> {
        DisplayRowAppendMeasurement::Measured(&mut *self.measurer)
    }
}

struct DisplayReplacementSourceAppendPolicy<'a, M> {
    item_measurer: &'a mut M,
}

impl<M: DisplayRowItemMeasurer> DisplayRowSourceAppendPolicy
    for DisplayReplacementSourceAppendPolicy<'_, M>
{
    fn decision_for(&mut self, item: &DisplayItem) -> DisplayRowSourceAppendDecision {
        if matches!(item.kind, DisplayItemKind::RowBreak(_)) {
            return DisplayRowSourceAppendDecision::Stop;
        }
        DisplayRowSourceAppendDecision::Append {
            kind: DisplayRowAppendKind::DisplayReplacementString,
            on_clipped: if matches!(item.kind, DisplayItemKind::SourceMappedText(_)) {
                DisplayRowAppendClipBehavior::Stop
            } else {
                DisplayRowAppendClipBehavior::Continue
            },
        }
    }

    fn measurement_for<'a>(
        &'a mut self,
        item: &DisplayItem,
        face_id: u32,
    ) -> DisplayRowAppendMeasurement<'a> {
        self.item_measurer.measurement_for(item, face_id)
    }
}

#[allow(clippy::too_many_arguments)]
fn append_display_item_stream_to_text_row<P: DisplayRowSourceAppendPolicy>(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
    mut position: DisplayRowPosition,
    policy: &mut P,
    mut next_item: impl FnMut(&mut GlyphMatrixBuilder) -> Option<DisplayItem>,
) -> DisplayItemSourceAppendResult {
    let mut last_progress = None;
    while let Some(mut item) = next_item(builder) {
        let (kind, on_clipped) = match policy.decision_for(&item) {
            DisplayRowSourceAppendDecision::Append { kind, on_clipped } => (kind, on_clipped),
            DisplayRowSourceAppendDecision::Skip => continue,
            DisplayRowSourceAppendDecision::Stop => {
                break;
            }
        };
        let face_id = render_face_ref_id(item.face, fallback_face_id);
        let measurement = policy.measurement_for(&item, face_id);
        item.face = RenderFaceRef::FaceId(face_id);
        let append_spec = frame.clone().at(position, face_id).append_spec(kind);
        let Some((progress, next_position)) = (match measurement {
            DisplayRowAppendMeasurement::Default => append_display_row_spec_item_and_emit(
                builder,
                output_emitter,
                evaluator,
                append_spec,
                item,
            ),
            DisplayRowAppendMeasurement::Measured(measurer) => {
                append_measured_display_row_spec_item_and_emit(
                    builder,
                    output_emitter,
                    evaluator,
                    append_spec,
                    item,
                    measurer,
                )
            }
        }) else {
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
    let mut source = DisplayItemSourceWalker::new(source);
    append_display_item_stream_to_text_row(
        builder,
        output_emitter,
        evaluator,
        fallback_face_id,
        frame,
        position,
        policy,
        |builder| source.next_item(builder, face_resolver, base_face, current_face_id),
    )
    .position
}

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
    append_display_row_spec_item_and_emit(builder, output_emitter, evaluator, append_spec, item)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_display_replacement_string_source_to_text_row<S: DisplayItemSource>(
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
    item_measurer: &mut impl DisplayRowItemMeasurer,
) -> DisplayRowPosition {
    let mut policy = DisplayReplacementSourceAppendPolicy { item_measurer };
    append_display_item_source_to_text_row(
        builder,
        output_emitter,
        evaluator,
        source,
        face_resolver,
        base_face,
        fallback_face_id,
        current_face_id,
        frame,
        position,
        &mut policy,
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

pub(crate) struct DisplayRowAppendSpec {
    pub(crate) layout: DisplayRowLayout,
    pub(crate) position: DisplayRowPosition,
    pub(crate) max_x: f32,
    pub(crate) output: DisplayRowAppendOutput,
}

impl DisplayRowAppendContext {
    pub(crate) fn append_spec(&self, kind: DisplayRowAppendKind) -> DisplayRowAppendSpec {
        let char_width = match kind {
            DisplayRowAppendKind::Tab | DisplayRowAppendKind::DisplayReplacementString => {
                self.face_space_width
            }
            DisplayRowAppendKind::SourceText
            | DisplayRowAppendKind::ControlChar
            | DisplayRowAppendKind::SourceMappedText
            | DisplayRowAppendKind::Glyphless
            | DisplayRowAppendKind::DisplayReplacement => self.geometry.char_width,
        };
        let max_x = match kind {
            DisplayRowAppendKind::Tab => f32::INFINITY,
            DisplayRowAppendKind::ControlChar => {
                self.content_x + (self.text_width - self.line_number_width)
            }
            DisplayRowAppendKind::SourceText
            | DisplayRowAppendKind::SourceMappedText
            | DisplayRowAppendKind::Glyphless
            | DisplayRowAppendKind::DisplayReplacement
            | DisplayRowAppendKind::DisplayReplacementString => {
                self.content_x + self.geometry.width
            }
        };
        let output_height = match kind {
            DisplayRowAppendKind::SourceText
            | DisplayRowAppendKind::Glyphless
            | DisplayRowAppendKind::DisplayReplacement
            | DisplayRowAppendKind::DisplayReplacementString => self.geometry.height,
            DisplayRowAppendKind::Tab
            | DisplayRowAppendKind::ControlChar
            | DisplayRowAppendKind::SourceMappedText => self.default_row_height,
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

fn display_slot_row(row: usize) -> u32 {
    row.min(u32::MAX as usize) as u32
}

fn display_slot_col(col: usize) -> u16 {
    col.min(usize::from(u16::MAX)) as u16
}

pub(crate) fn append_measured_display_row_item(
    builder: &mut GlyphMatrixBuilder,
    layout: &DisplayRowLayout,
    position: DisplayRowPosition,
    max_x: f32,
    item: DisplayItem,
    glyph_measurer: &mut dyn DisplayGlyphMeasurer,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let mut append_cursor = DisplayRowAppendCursor::new(position, max_x);
    let progress = append_cursor.append_measured_item_to_current_matrix_row(
        builder,
        layout,
        item,
        glyph_measurer,
    )?;
    let position = append_cursor.position();
    Some((progress, position))
}

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

pub(crate) fn append_measured_display_row_spec_item_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    spec: DisplayRowAppendSpec,
    item: DisplayItem,
    glyph_measurer: &mut dyn DisplayGlyphMeasurer,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    append_measured_display_row_item_and_emit(
        builder,
        output_emitter,
        evaluator,
        &spec.layout,
        spec.position,
        spec.max_x,
        item,
        glyph_measurer,
        spec.output,
    )
}

pub(crate) fn append_measured_display_row_item_and_emit(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    layout: &DisplayRowLayout,
    position: DisplayRowPosition,
    max_x: f32,
    item: DisplayItem,
    glyph_measurer: &mut dyn DisplayGlyphMeasurer,
    output: DisplayRowAppendOutput,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let (progress, position) =
        append_measured_display_row_item(builder, layout, position, max_x, item, glyph_measurer)?;
    emit_text_progress_slots(
        output_emitter,
        evaluator,
        &progress,
        output.row,
        output.row_y,
        output.glyph_y,
        output.height,
    );
    Some((progress, position))
}

pub(crate) fn append_synthetic_text_to_display_row(
    builder: &mut GlyphMatrixBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source_id: u64,
    text: impl Into<Box<str>>,
    face_id: u32,
    glyph_measurer: Option<&mut dyn DisplayGlyphMeasurer>,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let append_spec = frame
        .at(position, face_id)
        .append_spec(DisplayRowAppendKind::SourceText);
    let item = synthetic_display_text_item(source_id, text, face_id);
    match glyph_measurer {
        Some(measurer) => append_measured_display_row_spec_item_and_emit(
            builder,
            output_emitter,
            evaluator,
            append_spec,
            item,
            measurer,
        ),
        None => append_display_row_spec_item_and_emit(
            builder,
            output_emitter,
            evaluator,
            append_spec,
            item,
        ),
    }
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
