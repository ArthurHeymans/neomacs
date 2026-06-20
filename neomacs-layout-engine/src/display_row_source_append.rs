use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowRenderPolicy,
    DisplayRowSourceState, DisplaySourceAppendRenderPolicy, NaturalDisplayRowAppendRenderPolicy,
};
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendFrame, DisplayRowAppendKind,
    DisplayRowAppendSurface,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_source_render::{TextRowSourceMeasureState, TextRowSourceRenderState};
use crate::display_source::{DisplayItemOnceSource, DisplayItemSource, SyntheticTextItemSource};
use crate::neovm_bridge::ResolvedFace;

const SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS: u64 = 3;
const SYNTHETIC_SOURCE_HSCROLL_TRUNCATION: u64 = 4;
const SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS: u64 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyntheticTextSource {
    pub(crate) source_id: u64,
    pub(crate) text: Box<str>,
}

#[derive(Clone, Debug)]
pub(crate) struct SyntheticTextAppendRequest {
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face: SyntheticTextAppendFace,
}

#[derive(Clone, Debug)]
enum SyntheticTextAppendFace {
    ActiveFace,
    TextRowMetrics {
        face_id: u32,
        base_face: ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    },
}

#[derive(Clone)]
struct SyntheticTextAppendContext<'a> {
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyntheticTextMarker {
    InvisibleEllipsis,
    HscrollTruncation,
    SelectiveEllipsis,
}

#[derive(Clone, Copy)]
pub(crate) struct SyntheticTextRowAppendContext<'a> {
    active_face_context: DisplayRowActiveFaceAppendContext<'a, 'a>,
}

pub(crate) struct DisplayItemSourceAppendRequest<'frame, 'face> {
    base_face: &'face ResolvedFace,
    base_face_id: u32,
    frame: &'frame DisplayRowAppendFrame,
    position: DisplayRowPosition,
    kind: DisplayRowAppendKind,
}

pub(crate) struct SingleDisplayItemSourceAppendRequest<'frame, 'face> {
    base_face: &'face ResolvedFace,
    fallback_face_id: u32,
    frame: &'frame DisplayRowAppendFrame,
    item: DisplayItem,
    position: DisplayRowPosition,
    fallback_kind: DisplayRowAppendKind,
}

pub(crate) struct SingleDisplayItemSourceMeasureRequest<'frame, 'face> {
    base_face: &'face ResolvedFace,
    fallback_face_id: u32,
    frame: &'frame DisplayRowAppendFrame,
    item: DisplayItem,
    position: DisplayRowPosition,
    fallback_kind: DisplayRowAppendKind,
}

struct PreparedSingleDisplayItemSourceAppend {
    item: DisplayItem,
    face_id: u32,
    kind: DisplayRowAppendKind,
    position: DisplayRowPosition,
}

impl SyntheticTextSource {
    #[cfg(test)]
    pub(crate) fn new(source_id: u64, text: impl Into<Box<str>>) -> Self {
        Self {
            source_id,
            text: text.into(),
        }
    }

    fn marker(marker: SyntheticTextMarker) -> Self {
        Self {
            source_id: marker.source_id(),
            text: marker.text().into(),
        }
    }

    fn into_item_source(self, face_id: u32) -> SyntheticTextItemSource {
        SyntheticTextItemSource::new(self.source_id, self.text, RenderFaceRef::FaceId(face_id), 0)
    }
}

impl SyntheticTextAppendRequest {
    #[cfg(test)]
    pub(crate) fn active_source(position: DisplayRowPosition, source: SyntheticTextSource) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::ActiveFace,
        }
    }

    pub(crate) fn active_marker(position: DisplayRowPosition, marker: SyntheticTextMarker) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::ActiveFace,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_source(
        position: DisplayRowPosition,
        source: SyntheticTextSource,
        face_id: u32,
        base_face: &ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                height_px,
                ascent_px,
                char_width_px,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_marker(
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
        face_id: u32,
        base_face: &ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                height_px,
                ascent_px,
                char_width_px,
            },
        }
    }

    fn into_parts(
        self,
    ) -> (
        DisplayRowPosition,
        SyntheticTextSource,
        SyntheticTextAppendFace,
    ) {
        (self.position, self.source, self.face)
    }
}

impl<'a> SyntheticTextAppendContext<'a> {
    fn new(face_id: u32, base_face: &'a ResolvedFace, frame: DisplayRowAppendFrame) -> Self {
        Self {
            face_id,
            base_face,
            frame,
        }
    }

    fn append_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
        source: SyntheticTextSource,
    ) -> Option<DisplayRowAppendProgress> {
        append_synthetic_text_to_display_row(
            state,
            self.base_face,
            self.frame.clone(),
            position,
            source,
            self.face_id,
        )
    }
}

impl SyntheticTextMarker {
    pub(crate) fn source_id(self) -> u64 {
        match self {
            Self::InvisibleEllipsis => SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS,
            Self::HscrollTruncation => SYNTHETIC_SOURCE_HSCROLL_TRUNCATION,
            Self::SelectiveEllipsis => SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS,
        }
    }

    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::InvisibleEllipsis | Self::SelectiveEllipsis => "...",
            Self::HscrollTruncation => "$",
        }
    }
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

    fn text_row<'face>(
        self,
        face_id: u32,
        base_face: &'face ResolvedFace,
        height_px: f32,
        ascent_px: f32,
        char_width_px: f32,
    ) -> SyntheticTextAppendContext<'face> {
        SyntheticTextAppendContext::new(
            face_id,
            base_face,
            self.active_face_context
                .text_row_frame(height_px, ascent_px, char_width_px),
        )
    }

    pub(crate) fn append_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        request: SyntheticTextAppendRequest,
    ) -> Option<DisplayRowAppendProgress> {
        let (position, source, face) = request.into_parts();
        match face {
            SyntheticTextAppendFace::ActiveFace => {
                let active_face = self.active_face_context.active_face();
                self.active_face(active_face.face_id(), active_face.resolved_face())
                    .append_to_text_row_and_emit(state, position, source)
            }
            SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face,
                height_px,
                ascent_px,
                char_width_px,
            } => self
                .text_row(face_id, &base_face, height_px, ascent_px, char_width_px)
                .append_to_text_row_and_emit(state, position, source),
        }
    }
}

impl<'frame, 'face> DisplayItemSourceAppendRequest<'frame, 'face> {
    pub(crate) fn new(
        base_face: &'face ResolvedFace,
        base_face_id: u32,
        frame: &'frame DisplayRowAppendFrame,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
    ) -> Self {
        Self {
            base_face,
            base_face_id,
            frame,
            position,
            kind,
        }
    }
}

impl<'frame, 'face> SingleDisplayItemSourceAppendRequest<'frame, 'face> {
    pub(crate) fn new(
        base_face: &'face ResolvedFace,
        fallback_face_id: u32,
        frame: &'frame DisplayRowAppendFrame,
        item: DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
    ) -> Self {
        Self {
            base_face,
            fallback_face_id,
            frame,
            item,
            position,
            fallback_kind,
        }
    }

    fn prepare(self) -> PreparedSingleDisplayItemSourceAppend {
        prepare_single_display_item_source_append(
            self.item,
            self.fallback_face_id,
            self.position,
            self.fallback_kind,
        )
    }
}

impl<'frame, 'face> SingleDisplayItemSourceMeasureRequest<'frame, 'face> {
    pub(crate) fn new(
        base_face: &'face ResolvedFace,
        fallback_face_id: u32,
        frame: &'frame DisplayRowAppendFrame,
        item: DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
    ) -> Self {
        Self {
            base_face,
            fallback_face_id,
            frame,
            item,
            position,
            fallback_kind,
        }
    }

    fn prepare(self) -> PreparedSingleDisplayItemSourceAppend {
        prepare_single_display_item_source_append(
            self.item,
            self.fallback_face_id,
            self.position,
            self.fallback_kind,
        )
    }
}

pub(crate) fn render_display_item_source_with_policy<S, P>(
    state: &mut TextRowSourceRenderState<'_>,
    face_ids: &mut FrameFaceIdAllocator,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    request: DisplayItemSourceAppendRequest<'_, '_>,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome>
where
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
{
    let row_request = request.frame.source_append_render_request(
        request.position,
        request.base_face_id,
        request.base_face,
        request.kind,
    );
    state.render_display_item_source_into_current_text_row_and_emit(
        face_ids,
        source,
        source_state,
        row_request,
        render_policy,
    )
}

pub(crate) fn render_single_display_item_with_policy<P: DisplayRowRenderPolicy>(
    state: &mut TextRowSourceRenderState<'_>,
    request: SingleDisplayItemSourceAppendRequest<'_, '_>,
    render_policy: &mut P,
) -> Option<DisplayRowAppendProgress> {
    let base_face = request.base_face;
    let frame = request.frame;
    let prepared = request.prepare();
    let mut face_ids = FrameFaceIdAllocator::new(prepared.face_id.saturating_add(1));
    let mut source = DisplayItemOnceSource::new(prepared.item);
    let mut source_state = DisplayRowSourceState::default();
    let request = DisplayItemSourceAppendRequest::new(
        base_face,
        prepared.face_id,
        frame,
        prepared.position,
        prepared.kind,
    );
    let outcome = render_display_item_source_with_policy(
        state,
        &mut face_ids,
        &mut source,
        &mut source_state,
        request,
        render_policy,
    )?;
    Some(outcome.into_append_progress(prepared.position))
}

pub(crate) fn render_single_display_item_naturally(
    state: &mut TextRowSourceRenderState<'_>,
    request: SingleDisplayItemSourceAppendRequest<'_, '_>,
) -> Option<DisplayRowAppendProgress> {
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    render_single_display_item_with_policy(state, request, &mut render_policy)
}

pub(crate) fn measure_single_display_item_width_with_policy<P: DisplayRowRenderPolicy>(
    state: &mut TextRowSourceMeasureState<'_>,
    request: SingleDisplayItemSourceMeasureRequest<'_, '_>,
    render_policy: &mut P,
) -> Option<f32> {
    let base_face = request.base_face;
    let frame = request.frame;
    let prepared = request.prepare();
    let row_request = frame.source_append_measure_request(
        prepared.position,
        prepared.face_id,
        base_face,
        prepared.kind,
    );
    let mut face_ids = FrameFaceIdAllocator::new(prepared.face_id.saturating_add(1));
    let mut source = DisplayItemOnceSource::new(prepared.item);
    let mut source_state = DisplayRowSourceState::default();
    let outcome = state.measure_display_item_source_against_current_text_row(
        &mut face_ids,
        &mut source,
        &mut source_state,
        row_request,
        render_policy,
    )?;
    Some(
        outcome
            .into_append_progress(prepared.position)
            .metrics
            .width_px,
    )
}

pub(crate) fn measure_single_display_item_width_naturally(
    state: &mut TextRowSourceMeasureState<'_>,
    request: SingleDisplayItemSourceMeasureRequest<'_, '_>,
) -> Option<f32> {
    let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
    measure_single_display_item_width_with_policy(state, request, &mut render_policy)
}

pub(crate) fn measure_single_display_item_width_naturally_or_fallback(
    state: &mut TextRowSourceMeasureState<'_>,
    request: SingleDisplayItemSourceMeasureRequest<'_, '_>,
    fallback_width_px: f32,
) -> f32 {
    measure_single_display_item_width_naturally(state, request).unwrap_or(fallback_width_px)
}

fn append_synthetic_text_to_display_row(
    state: &mut TextRowSourceRenderState<'_>,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face_id: u32,
) -> Option<DisplayRowAppendProgress> {
    let mut source = source.into_item_source(face_id);
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    let start = position;
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let request = DisplayItemSourceAppendRequest::new(
        base_face,
        face_id,
        &frame,
        position,
        DisplayRowAppendKind::SourceText,
    );
    let mut source_state = DisplayRowSourceState::default();
    let outcome = render_display_item_source_with_policy(
        state,
        &mut face_ids,
        &mut source,
        &mut source_state,
        request,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress(start))
}

fn render_face_ref_id(face: RenderFaceRef, fallback: u32) -> u32 {
    match face {
        RenderFaceRef::FaceId(face_id) => face_id,
        RenderFaceRef::Inherit => fallback,
    }
}

fn prepare_single_display_item_source_append(
    item: DisplayItem,
    fallback_face_id: u32,
    position: DisplayRowPosition,
    fallback_kind: DisplayRowAppendKind,
) -> PreparedSingleDisplayItemSourceAppend {
    let kind = display_item_append_kind(&item, fallback_kind);
    let face_id = render_face_ref_id(item.face, fallback_face_id);
    let mut item = item;
    item.face = RenderFaceRef::FaceId(face_id);
    PreparedSingleDisplayItemSourceAppend {
        item,
        face_id,
        kind,
        position,
    }
}

pub(crate) fn display_item_append_kind(
    item: &DisplayItem,
    fallback: DisplayRowAppendKind,
) -> DisplayRowAppendKind {
    match &item.kind {
        DisplayItemKind::TextRun(run) if run.text.as_ref() == "\t" => DisplayRowAppendKind::Tab,
        DisplayItemKind::TextRun(_) => DisplayRowAppendKind::SourceText,
        DisplayItemKind::SourceMappedText(_) => DisplayRowAppendKind::SourceMappedText,
        DisplayItemKind::ControlChar { .. } => DisplayRowAppendKind::ControlChar,
        DisplayItemKind::Glyphless(_) => DisplayRowAppendKind::Glyphless,
        _ => fallback,
    }
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
