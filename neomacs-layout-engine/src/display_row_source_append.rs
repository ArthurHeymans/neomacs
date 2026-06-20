use crate::display_buffer_text_render::SyntheticTextSource;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowRenderPolicy, DisplayRowSourceState,
    DisplaySourceAppendRenderPolicy, NaturalDisplayRowAppendRenderPolicy,
};
use crate::display_row_append_context::{DisplayRowAppendFrame, DisplayRowAppendKind};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_source_render::{TextRowSourceMeasureState, TextRowSourceRenderState};
use crate::display_source::{DisplayItemOnceSource, DisplayItemSource};
use crate::neovm_bridge::ResolvedFace;

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

pub(crate) fn append_synthetic_text_to_display_row(
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
