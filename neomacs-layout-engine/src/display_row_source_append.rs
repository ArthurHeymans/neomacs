use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_ref::render_face_ref_id;
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowRenderPolicy,
    DisplayRowSourceState,
};
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendFrame, DisplayRowAppendKind,
    DisplayRowAppendSurface,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_source_render::{TextRowSourceMeasureState, TextRowSourceRenderState};
use crate::display_source::{DisplayItemOnceSource, DisplayItemSource, SyntheticTextItemSource};
use crate::display_source_append_plan::{
    DisplaySourceAppendRenderPolicy, DisplaySourceFallbackWidth,
    NaturalDisplayRowAppendRenderPolicy,
};
use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::face::BasicFaceId;

const SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS: u64 = 3;
const SYNTHETIC_SOURCE_HSCROLL_TRUNCATION: u64 = 4;
const SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS: u64 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyntheticTextSource {
    source_id: u64,
    text: Box<str>,
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
        metrics: DisplayRowFallbackMetrics,
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

#[derive(Clone, Copy)]
pub(crate) struct BufferSyntheticTextRenderContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    active_face: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    metrics: DisplayRowFallbackMetrics,
}

struct PreparedSingleDisplayItemSourceAppend {
    item: DisplayItem,
    face_id: u32,
    kind: DisplayRowAppendKind,
    position: DisplayRowPosition,
}

impl PreparedSingleDisplayItemSourceAppend {
    fn into_parts(self) -> (DisplayItem, u32, DisplayRowAppendKind, DisplayRowPosition) {
        (self.item, self.face_id, self.kind, self.position)
    }
}

#[derive(Clone)]
pub(crate) struct DisplayItemSourceAppendContext<'face> {
    base_face: &'face ResolvedFace,
    face_id: u32,
    frame: DisplayRowAppendFrame,
}

#[derive(Clone)]
pub(crate) struct SingleDisplayItemAppendContext<'face> {
    base_face: &'face ResolvedFace,
    face_id: u32,
    frame: DisplayRowAppendFrame,
}

impl SyntheticTextSource {
    #[cfg(test)]
    pub(crate) fn new(source_id: u64, text: impl Into<Box<str>>) -> Self {
        Self {
            source_id,
            text: text.into(),
        }
    }

    #[cfg(test)]
    pub(crate) fn source_id(&self) -> u64 {
        self.source_id
    }

    #[cfg(test)]
    pub(crate) fn text(&self) -> &str {
        &self.text
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
        metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            position,
            source,
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                metrics,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_row_metrics_marker(
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
        face_id: u32,
        base_face: &ResolvedFace,
        metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            position,
            source: SyntheticTextSource::marker(marker),
            face: SyntheticTextAppendFace::TextRowMetrics {
                face_id,
                base_face: base_face.clone(),
                metrics,
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
    fn new(
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &'a DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            active_face_context: DisplayRowActiveFaceAppendContext::new(
                append_surface,
                geometry,
                active_face,
                glyph_y_offset,
                fallback_metrics,
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
                metrics,
            } => self
                .text_row(
                    face_id,
                    &base_face,
                    metrics.row_height(),
                    metrics.ascent(),
                    metrics.char_width(),
                )
                .append_to_text_row_and_emit(state, position, source),
        }
    }
}

impl<'a> BufferSyntheticTextRenderContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            append_surface,
            active_face,
            glyph_y_offset,
            metrics,
        }
    }

    pub(crate) fn active_face(self) -> &'a DisplayRowActiveFaceState {
        self.active_face
    }

    fn row_context(
        self,
        geometry: &'a DisplayRowGeometryState,
    ) -> SyntheticTextRowAppendContext<'a> {
        SyntheticTextRowAppendContext::new(
            self.append_surface,
            geometry,
            self.active_face,
            self.glyph_y_offset,
            self.metrics,
        )
    }

    pub(crate) fn render_request_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) -> Option<DisplayRowAppendProgress> {
        self.row_context(geometry)
            .append_request_to_text_row_and_emit(state, request)
    }

    #[cfg(test)]
    pub(crate) fn render_active_marker_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        position: DisplayRowPosition,
        marker: SyntheticTextMarker,
    ) -> Option<DisplayRowPosition> {
        self.render_request_to_text_row(
            state,
            geometry,
            SyntheticTextAppendRequest::active_marker(position, marker),
        )
        .map(|progress| progress.end())
    }

    pub(crate) fn hscroll_truncation_request(
        self,
        base_face: ResolvedFace,
        content_x: f32,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::text_row_metrics_marker(
            DisplayRowPosition::new(content_x, 0),
            SyntheticTextMarker::HscrollTruncation,
            BasicFaceId::Default.into(),
            &base_face,
            self.metrics,
        )
    }

    #[cfg(test)]
    pub(crate) fn render_hscroll_truncation_marker_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        content_x: f32,
    ) -> Option<DisplayRowPosition> {
        let request = self.hscroll_truncation_request(state.default_face(), content_x);
        self.render_request_to_text_row(state, geometry, request)
            .map(|progress| progress.end())
    }
}

impl<'face> DisplayItemSourceAppendContext<'face> {
    pub(crate) fn new(
        base_face: &'face ResolvedFace,
        face_id: u32,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            base_face,
            face_id,
            frame,
        }
    }

    pub(crate) fn render_with_policy<S, P>(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        let row_request =
            self.frame
                .source_append_render_request(position, self.face_id, self.base_face, kind);
        state.render_display_item_source_into_current_text_row_and_emit(
            face_ids,
            source,
            source_state,
            row_request,
            render_policy,
        )
    }

    fn measure_with_policy<S, P>(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        let row_request =
            self.frame
                .source_append_measure_request(position, self.face_id, self.base_face, kind);
        state.measure_display_item_source_against_current_text_row(
            face_ids,
            source,
            source_state,
            row_request,
            render_policy,
        )
    }
}

impl<'face> SingleDisplayItemAppendContext<'face> {
    pub(crate) fn new(
        base_face: &'face ResolvedFace,
        face_id: u32,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            base_face,
            face_id,
            frame,
        }
    }

    pub(crate) fn face_id(&self) -> u32 {
        self.face_id
    }

    #[cfg(test)]
    pub(crate) fn frame(&self) -> &DisplayRowAppendFrame {
        &self.frame
    }

    fn source_context(&self) -> DisplayItemSourceAppendContext<'face> {
        DisplayItemSourceAppendContext::new(self.base_face, self.face_id, self.frame.clone())
    }

    pub(crate) fn render_source_with_policy<S, P>(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        self.source_context().render_with_policy(
            state,
            face_ids,
            source,
            source_state,
            position,
            kind,
            render_policy,
        )
    }

    fn source_context_for_face(&self, face_id: u32) -> DisplayItemSourceAppendContext<'face> {
        DisplayItemSourceAppendContext::new(self.base_face, face_id, self.frame.clone())
    }

    fn prepare_item(
        &self,
        item: DisplayItem,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
    ) -> PreparedSingleDisplayItemSourceAppend {
        prepare_single_display_item_source_append(item, self.face_id, position, kind)
    }

    pub(crate) fn render_with_policy<P: DisplayRowRenderPolicy>(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
        render_policy: &mut P,
    ) -> Option<DisplayRowAppendProgress> {
        let prepared = self.prepare_item(item, position, kind);
        let (item, face_id, kind, position) = prepared.into_parts();
        let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
        let mut source = DisplayItemOnceSource::new(item);
        let mut source_state = DisplayRowSourceState::default();
        let outcome = self.source_context_for_face(face_id).render_with_policy(
            state,
            &mut face_ids,
            &mut source,
            &mut source_state,
            position,
            kind,
            render_policy,
        )?;
        Some(outcome.into_append_progress(position))
    }

    pub(crate) fn render_naturally(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
    ) -> Option<DisplayRowAppendProgress> {
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        self.render_with_policy(state, item, position, kind, &mut render_policy)
    }

    pub(crate) fn measure_width_with_policy<P: DisplayRowRenderPolicy>(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
        render_policy: &mut P,
    ) -> Option<f32> {
        let prepared = self.prepare_item(item, position, kind);
        let (item, face_id, kind, position) = prepared.into_parts();
        let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
        let mut source = DisplayItemOnceSource::new(item);
        let mut source_state = DisplayRowSourceState::default();
        let outcome = self.source_context_for_face(face_id).measure_with_policy(
            state,
            &mut face_ids,
            &mut source,
            &mut source_state,
            position,
            kind,
            render_policy,
        )?;
        Some(outcome.into_append_progress(position).metrics().width_px())
    }

    pub(crate) fn measure_width_naturally(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
    ) -> Option<f32> {
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        self.measure_width_with_policy(state, item, position, kind, &mut render_policy)
    }

    pub(crate) fn measure_width_with_source_fallback(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
        fallback_width: DisplaySourceFallbackWidth,
    ) -> f32 {
        let fallback_width_px = fallback_width.resolve_to_text_row(&self.frame);
        self.measure_width_naturally(state, item, position, kind)
            .unwrap_or(fallback_width_px)
    }
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
    let context = DisplayItemSourceAppendContext::new(base_face, face_id, frame);
    let mut source_state = DisplayRowSourceState::default();
    let outcome = context.render_with_policy(
        state,
        &mut face_ids,
        &mut source,
        &mut source_state,
        position,
        DisplayRowAppendKind::SourceText,
        &mut render_policy,
    )?;
    Some(outcome.into_append_progress(start))
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
