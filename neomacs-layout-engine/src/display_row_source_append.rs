use crate::display_buffer_text_render::SyntheticTextSource;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_row::{
    DisplayRowRenderBounds, DisplayRowRenderPolicy, DisplayRowSourceState,
    DisplaySourceAppendRenderPolicy, NaturalDisplayRowAppendRenderPolicy,
};
use crate::display_row_append_context::{DisplayRowAppendFrame, DisplayRowAppendKind};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_source_render::{TextRowSourceMeasureState, TextRowSourceRenderState};
use crate::display_source::DisplayItemOnceSource;
use crate::neovm_bridge::ResolvedFace;

#[derive(Clone)]
pub(crate) struct DisplayRowSingleItemAppendContext<'face> {
    base_face: &'face ResolvedFace,
    fallback_face_id: u32,
    frame: DisplayRowAppendFrame,
}

impl<'face> DisplayRowSingleItemAppendContext<'face> {
    pub(crate) fn new(
        base_face: &'face ResolvedFace,
        fallback_face_id: u32,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            base_face,
            fallback_face_id,
            frame,
        }
    }

    pub(crate) fn render_item_with_policy<P: DisplayRowRenderPolicy>(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
        render_policy: &mut P,
    ) -> Option<DisplayRowAppendProgress> {
        let kind = display_item_append_kind(&item, fallback_kind);
        let face_id = render_face_ref_id(item.face, self.fallback_face_id);
        let mut item = item;
        item.face = RenderFaceRef::FaceId(face_id);
        let (request, output) =
            self.frame
                .source_render_parts(position, face_id, self.base_face, kind);
        let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
        let mut source = DisplayItemOnceSource::new(item);
        let mut source_state = DisplayRowSourceState::default();
        let outcome = state.render_display_item_source_into_current_text_row_and_emit(
            &mut face_ids,
            &mut source,
            &mut source_state,
            request,
            output,
            render_policy,
        )?;
        Some(outcome.into_append_progress(position))
    }

    pub(crate) fn render_item_naturally(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
    ) -> Option<DisplayRowAppendProgress> {
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        self.render_item_with_policy(state, item, position, fallback_kind, &mut render_policy)
    }

    pub(crate) fn measure_item_width_with_policy<P: DisplayRowRenderPolicy>(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: &DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
        render_policy: &mut P,
    ) -> Option<f32> {
        let kind = display_item_append_kind(item, fallback_kind);
        let face_id = render_face_ref_id(item.face, self.fallback_face_id);
        let mut item = item.clone();
        item.face = RenderFaceRef::FaceId(face_id);
        let request = self
            .frame
            .source_render_request(position, face_id, self.base_face, kind)
            .with_render_bounds(DisplayRowRenderBounds::unbounded_from(position));
        let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
        let mut source = DisplayItemOnceSource::new(item);
        let mut source_state = DisplayRowSourceState::default();
        let outcome = state.measure_display_item_source_against_current_text_row(
            &mut face_ids,
            &mut source,
            &mut source_state,
            request,
            render_policy,
        )?;
        Some(outcome.into_append_progress(position).metrics.width_px)
    }

    pub(crate) fn measure_item_width_naturally(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: &DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
    ) -> Option<f32> {
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        self.measure_item_width_with_policy(
            state,
            item,
            position,
            fallback_kind,
            &mut render_policy,
        )
    }

    pub(crate) fn measure_item_width_naturally_or_fallback(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: &DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
        fallback_width_px: f32,
    ) -> f32 {
        self.measure_item_width_naturally(state, item, position, fallback_kind)
            .unwrap_or(fallback_width_px)
    }
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
    let (request, output) = frame.source_render_parts(
        position,
        face_id,
        base_face,
        DisplayRowAppendKind::SourceText,
    );
    let mut source_state = DisplayRowSourceState::default();
    let outcome = state.render_display_item_source_into_current_text_row_and_emit(
        &mut face_ids,
        &mut source,
        &mut source_state,
        request,
        output,
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
