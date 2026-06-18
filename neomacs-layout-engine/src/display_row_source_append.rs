use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{DisplayItem, RenderFaceRef};
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowRenderBounds, DisplayRowRenderPolicy,
    DisplayRowSourceAppendRequest, DisplayRowSourceState,
};
use crate::display_row_append::{DisplayRowAppendFrame, DisplayRowAppendKind};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_source_render::{
    TextRowSourceMeasureState, TextRowSourceRenderState, current_text_measure_state,
    current_text_render_state,
};
use crate::display_source::DisplayItemSource;
use crate::neovm_bridge::ResolvedFace;

pub(crate) struct DisplayRowSourceAppendOperation<'face> {
    base_face: &'face ResolvedFace,
    base_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    kind: DisplayRowAppendKind,
}

impl<'face> DisplayRowSourceAppendOperation<'face> {
    pub(crate) fn new(
        base_face: &'face ResolvedFace,
        base_face_id: u32,
        frame: DisplayRowAppendFrame,
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

    fn request(&self) -> DisplayRowSourceAppendRequest<'face> {
        self.frame.source_append_request(
            self.position,
            self.base_face_id,
            self.base_face,
            self.kind,
        )
    }

    pub(crate) fn for_single_item(
        item: &DisplayItem,
        base_face: &'face ResolvedFace,
        fallback_face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        kind: DisplayRowAppendKind,
    ) -> Self {
        Self::new(
            base_face,
            render_face_ref_id(item.face, fallback_face_id),
            frame,
            position,
            kind,
        )
    }

    pub(crate) fn render_single_item_to_text_row_and_emit<P: DisplayRowRenderPolicy>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        render_policy: &mut P,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let request = self.request();
        let start = request.start_position();
        let mut face_ids = FrameFaceIdAllocator::new(self.base_face_id.saturating_add(1));
        let outcome = request.render_display_item_into_current_text_row_and_emit(
            &mut current_text_render_state(state, &mut face_ids),
            item,
            render_policy,
        )?;
        Some(outcome.into_append_progress_and_position(start))
    }

    pub(crate) fn measure_single_item_to_text_row<P: DisplayRowRenderPolicy>(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: DisplayItem,
        render_policy: &mut P,
    ) -> Option<DisplayRowAppendProgress> {
        let request = self
            .request()
            .with_measurement_bounds(DisplayRowRenderBounds::unbounded_from(self.position));
        let start = request.start_position();
        let mut face_ids = FrameFaceIdAllocator::new(self.base_face_id.saturating_add(1));
        let outcome = request.measure_display_item_against_current_text_row(
            &mut current_text_measure_state(state, &mut face_ids),
            item,
            render_policy,
        )?;
        Some(outcome.into_append_progress(start))
    }

    pub(crate) fn render_source_to_text_row_and_emit<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        source: S,
        face_ids: &mut FrameFaceIdAllocator,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.request()
            .render_owned_display_source_into_current_text_row_and_emit(
                &mut current_text_render_state(state, face_ids),
                source,
                render_policy,
            )
    }

    pub(crate) fn render_source_cursor_to_text_row_and_emit<S: DisplayItemSource>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.request()
            .render_natural_display_source_into_current_text_row_and_emit(
                &mut current_text_render_state(state, face_ids),
                source,
                source_state,
            )
    }
}

fn render_face_ref_id(face: RenderFaceRef, fallback: u32) -> u32 {
    match face {
        RenderFaceRef::FaceId(face_id) => face_id,
        RenderFaceRef::Inherit => fallback,
    }
}
