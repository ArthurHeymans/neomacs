use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{DisplayItem, RenderFaceRef};
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowCurrentTextMeasureState,
    DisplayRowCurrentTextRenderState, DisplayRowGeometry, DisplayRowRenderBounds,
    DisplayRowRenderPolicy, DisplayRowSourceRenderRequest, DisplayRowSourceRequestPolicy,
    DisplayRowSourceState, NaturalDisplayRowAppendRenderPolicy,
    measure_display_item_source_against_current_text_row,
    render_display_item_source_into_current_text_row_and_emit,
};
use crate::display_row_append_context::{DisplayRowAppendFrame, DisplayRowAppendKind};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_geometry::DisplayRowMaxX;
use crate::display_row_source_render::{
    TextRowSourceMeasureState, TextRowSourceRenderState, current_text_measure_state,
    current_text_render_state,
};
use crate::display_source::{DisplayItemOnceSource, DisplayItemSource};
use crate::neovm_bridge::ResolvedFace;
use crate::window_output::TextRowOutput;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplayRowSourceAppendRequestPolicy {
    matrix_row: usize,
    row_y: f32,
    glyph_y: f32,
    output_height: f32,
    geometry: DisplayRowGeometry,
    max_x: DisplayRowMaxX,
}

impl DisplayRowSourceAppendRequestPolicy {
    pub(crate) fn new(
        matrix_row: usize,
        row_y: f32,
        glyph_y: f32,
        output_height: f32,
        geometry: DisplayRowGeometry,
        max_x: DisplayRowMaxX,
    ) -> Self {
        Self {
            matrix_row,
            row_y,
            glyph_y,
            output_height,
            geometry,
            max_x,
        }
    }
}

pub(crate) struct DisplayRowSourceAppendRequest<'face> {
    request: DisplayRowSourceRenderRequest<'face>,
    output: TextRowOutput,
    start: DisplayRowPosition,
}

impl<'face> DisplayRowSourceAppendRequest<'face> {
    fn new(
        request: DisplayRowSourceRenderRequest<'face>,
        output: TextRowOutput,
        start: DisplayRowPosition,
    ) -> Self {
        Self {
            request,
            output,
            start,
        }
    }

    pub(crate) fn from_text_row_policy(
        position: DisplayRowPosition,
        base_face_id: u32,
        base_face: &'face ResolvedFace,
        policy: DisplayRowSourceAppendRequestPolicy,
    ) -> Self {
        let request = DisplayRowSourceRequestPolicy::from_display_row_geometry(
            policy.geometry,
            GlyphRowRole::Text,
        )
        .source_request_for_base_face_id(base_face_id, base_face)
        .with_render_bounds(DisplayRowRenderBounds {
            start: position,
            max_x: policy.max_x,
        });
        let output = TextRowOutput {
            row: policy.matrix_row,
            row_y: policy.row_y,
            glyph_y: policy.glyph_y,
            height: policy.output_height,
        };
        Self::new(request, output, position)
    }

    pub(crate) fn start_position(&self) -> DisplayRowPosition {
        self.start
    }

    #[cfg(test)]
    pub(crate) fn base_face_id(&self) -> u32 {
        self.request.base_face_id()
    }

    #[cfg(test)]
    pub(crate) fn render_bounds(&self) -> DisplayRowRenderBounds {
        self.request.render_bounds()
    }

    #[cfg(test)]
    pub(crate) fn role(&self) -> GlyphRowRole {
        self.request.role()
    }

    #[cfg(test)]
    pub(crate) fn base_face_ref(&self) -> RenderFaceRef {
        self.request.base_face_ref()
    }

    #[cfg(test)]
    pub(crate) fn geometry(&self) -> &DisplayRowGeometry {
        self.request.geometry()
    }

    #[cfg(test)]
    pub(crate) fn output(&self) -> TextRowOutput {
        self.output
    }

    pub(crate) fn with_measurement_bounds(mut self, render_bounds: DisplayRowRenderBounds) -> Self {
        self.request = self.request.with_render_bounds(render_bounds);
        self
    }

    pub(crate) fn render_display_source_into_current_text_row_and_emit<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let Self {
            request, output, ..
        } = self;
        render_display_item_source_into_current_text_row_and_emit(
            state,
            source,
            source_state,
            request,
            output,
            render_policy,
        )
    }

    pub(crate) fn render_natural_display_source_into_current_text_row_and_emit<
        S: DisplayItemSource,
    >(
        self,
        state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        self.render_display_source_into_current_text_row_and_emit(
            state,
            source,
            source_state,
            &mut render_policy,
        )
    }

    pub(crate) fn render_owned_display_source_into_current_text_row_and_emit<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
        mut source: S,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let mut source_state = DisplayRowSourceState::default();
        self.render_display_source_into_current_text_row_and_emit(
            state,
            &mut source,
            &mut source_state,
            render_policy,
        )
    }

    pub(crate) fn render_display_item_into_current_text_row_and_emit<P: DisplayRowRenderPolicy>(
        self,
        state: &mut DisplayRowCurrentTextRenderState<'_, '_>,
        mut item: DisplayItem,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        item.face = RenderFaceRef::FaceId(self.request.base_face_id());
        self.render_owned_display_source_into_current_text_row_and_emit(
            state,
            DisplayItemOnceSource::new(item),
            render_policy,
        )
    }

    pub(crate) fn measure_display_source_against_current_text_row<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        state: &mut DisplayRowCurrentTextMeasureState<'_, '_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let Self { request, .. } = self;
        measure_display_item_source_against_current_text_row(
            state,
            source,
            source_state,
            request,
            render_policy,
        )
    }

    pub(crate) fn measure_owned_display_source_against_current_text_row<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        self,
        state: &mut DisplayRowCurrentTextMeasureState<'_, '_>,
        mut source: S,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let mut source_state = DisplayRowSourceState::default();
        self.measure_display_source_against_current_text_row(
            state,
            &mut source,
            &mut source_state,
            render_policy,
        )
    }

    pub(crate) fn measure_display_item_against_current_text_row<P: DisplayRowRenderPolicy>(
        self,
        state: &mut DisplayRowCurrentTextMeasureState<'_, '_>,
        mut item: DisplayItem,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        item.face = RenderFaceRef::FaceId(self.request.base_face_id());
        self.measure_owned_display_source_against_current_text_row(
            state,
            DisplayItemOnceSource::new(item),
            render_policy,
        )
    }
}

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
