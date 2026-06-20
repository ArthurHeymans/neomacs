use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_row::{DisplayRowActiveFaceState, DisplayRowFallbackMetrics};
#[cfg(test)]
use crate::display_row_append_context::DisplayRowAppendFrame;
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendKind, DisplayRowAppendSurface,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_source_append::SingleDisplayItemAppendContext;
use crate::display_row_source_render::{TextRowSourceMeasureState, TextRowSourceRenderState};
#[cfg(test)]
use crate::display_source::DisplaySourceTextItemRequest;
use crate::display_source::{
    DisplaySourceItemRequest, DisplaySourceRangeItemAppendRequest, DisplaySourceSpecialDisplayKind,
    DisplaySourceTextChar, DisplaySourceTextRequest, DisplaySpecialSourceCharRequest,
};
use crate::display_source_append_plan::{
    DisplaySourceAppendRenderPlan, DisplaySourceAppendRenderPolicy,
};
use crate::display_source_item_append::{
    DisplaySourceCharAppendContext, DisplaySourceItemAppendContext,
    DisplaySourcePreparedCharAppend, DisplaySourceRowAppendState,
    DisplaySourceSpecialCharAppendPlan, DisplaySourceSpecialCharPreparedAppend,
    DisplaySourceTextCharAppendPlan, DisplaySourceTextCharPreparedAppend,
    DisplaySourceTextPositionedRenderPlanRequest,
};
use crate::neovm_bridge::LayoutBufferView;
#[cfg(test)]
use crate::neovm_bridge::ResolvedFace;
use neovm_core::buffer::BufferId;

impl DisplaySourceTextRequest {
    #[cfg(test)]
    pub(crate) fn append_request<B: LayoutBufferView + ?Sized>(
        self,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
    ) -> Option<DisplaySourceRangeItemAppendRequest> {
        buffer_source_text_item_append_request(self.source_item(), buffer_id, buffer, face_id)
    }
}

#[cfg(test)]
pub(crate) fn buffer_source_text_item_append_request<B: LayoutBufferView + ?Sized>(
    source_item: DisplaySourceTextItemRequest,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<DisplaySourceRangeItemAppendRequest> {
    let append_kind = source_item.append_kind();
    let item = source_item.into_display_item(buffer_id, buffer, RenderFaceRef::FaceId(face_id))?;
    Some(DisplaySourceRangeItemAppendRequest::new(item, append_kind))
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSourceRowAppendContext<'source, 'surface, B: LayoutBufferView + ?Sized> {
    buffer: &'source B,
    buffer_id: BufferId,
    append_surface: &'surface DisplayRowAppendSurface,
    active_face: &'source DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
}

impl<'source, 'surface, B: LayoutBufferView + ?Sized>
    BufferSourceRowAppendContext<'source, 'surface, B>
{
    pub(crate) fn new(
        buffer: &'source B,
        buffer_id: BufferId,
        append_surface: &'surface DisplayRowAppendSurface,
        active_face: &'source DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            buffer,
            buffer_id,
            append_surface,
            active_face,
            glyph_y_offset,
            fallback_metrics,
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
            self.fallback_metrics,
        )
    }

    fn item_active_face(
        &self,
        geometry: &DisplayRowGeometryState,
    ) -> DisplaySourceItemAppendContext<'source> {
        let frame = self.active_face_context(geometry).active_face_frame();
        DisplaySourceItemAppendContext::new(
            self.active_face.face_id(),
            self.active_face.resolved_face(),
            frame,
        )
    }

    fn source_display_item_for_special_source_char(
        &self,
        request: &DisplaySpecialSourceCharRequest,
        source_item: &DisplayItem,
    ) -> DisplayItem {
        if let Some(source_item) = matching_special_display_item(source_item, request.kind()) {
            return source_item.clone();
        }

        buffer_source_item_append_request(
            request.source_item_request(),
            self.buffer_id,
            self.buffer,
            self.active_face.face_id(),
        )
        .map(DisplaySourceRangeItemAppendRequest::into_item)
        .unwrap_or_else(|| source_item.clone())
    }

    pub(crate) fn prepare_special_source_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        request: DisplaySpecialSourceCharRequest,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
    ) -> DisplaySourceSpecialCharPreparedAppend {
        let display_item = self.source_display_item_for_special_source_char(&request, source_item);
        let measured_width_px = request.requires_overflow_measurement().then(|| {
            self.item_active_face(geometry)
                .measure_source_display_item_width_to_text_row(
                    state,
                    &display_item,
                    request.source_item_request(),
                    position,
                )
        });
        request.prepared_append_at(position, measured_width_px, display_item)
    }

    fn append_special_source_char_plan_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: DisplaySourceSpecialCharAppendPlan,
    ) -> Option<DisplayRowAppendProgress> {
        let position = plan.position();
        let fallback_kind = plan.source_item().append_kind();
        self.item_active_face(geometry)
            .append_display_item_to_text_row_and_emit(
                state,
                plan.display_item,
                position,
                fallback_kind,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_source_render_plan_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut DisplaySourceRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        request: DisplaySourceTextPositionedRenderPlanRequest<'_, '_>,
    ) -> DisplaySourceAppendRenderPlan {
        let frame = self.active_face_context(geometry).active_face_frame();
        append_state.resolve_source_render_plan_request_to_text_row(
            measure_state,
            self.active_face,
            frame,
            request,
        )
    }

    fn prepare_source_char_append_plan(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut DisplaySourceRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        request: DisplaySourceTextPositionedRenderPlanRequest<'_, '_>,
    ) -> DisplaySourceTextCharAppendPlan {
        let render_plan = self.resolve_source_render_plan_request_to_text_row(
            geometry,
            append_state,
            measure_state,
            request,
        );
        request.append_plan(render_plan)
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_text_source_item_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut DisplaySourceRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        source_char: &DisplaySourceTextChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
        cluster_tail: Option<(char, bool)>,
    ) -> DisplaySourceTextCharPreparedAppend {
        let request = source_char.render_plan_request_for_item_at(
            text,
            byte_idx,
            position,
            source_item,
            cluster_tail,
        );
        DisplaySourceTextCharPreparedAppend {
            plan: self.prepare_source_char_append_plan(
                geometry,
                append_state,
                measure_state,
                request,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_source_item_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut DisplaySourceRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        source_char: &DisplaySourceTextChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
        cluster_tail: Option<(char, bool)>,
    ) -> DisplaySourcePreparedCharAppend {
        if let Some(request) = source_char.special_request(cluster_tail) {
            return DisplaySourcePreparedCharAppend::Special(self.prepare_special_source_char_at(
                geometry,
                measure_state,
                request,
                position,
                source_item,
            ));
        }
        DisplaySourcePreparedCharAppend::Text(self.prepare_text_source_item_char_at(
            geometry,
            append_state,
            measure_state,
            source_char,
            text,
            byte_idx,
            position,
            source_item,
            cluster_tail,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_source_item_for_current_text_row(
        &self,
        geometry: DisplayRowGeometryState,
        append_state: &mut DisplaySourceRowAppendState,
        source_render: &mut TextRowSourceRenderState<'_>,
        source_char: &DisplaySourceTextChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
    ) -> DisplaySourcePreparedCharAppend {
        let mut measure = source_render.measure_state();
        let cluster_tail = measure.current_cluster_tail();
        self.prepare_source_item_char_at(
            &geometry,
            append_state,
            &mut measure,
            source_char,
            text,
            byte_idx,
            position,
            source_item,
            cluster_tail,
        )
    }

    #[cfg(test)]
    pub(crate) fn append_source_text_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        source_text: DisplaySourceTextRequest,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        let frame = self.active_face_context(geometry).active_face_frame();
        let face_id = self.active_face.face_id();
        let append_item = source_text.append_request(self.buffer_id, self.buffer, face_id)?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = source_text.append_render_policy();
        SingleDisplayItemAppendContext::new(self.active_face.resolved_face(), face_id, frame)
            .render_with_policy(state, item, position, kind, &mut render_policy)
    }

    pub(crate) fn append_source_display_item_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
        render_policy: &mut DisplaySourceAppendRenderPolicy,
    ) -> Option<DisplayRowAppendProgress> {
        let frame = self.active_face_context(geometry).active_face_frame();
        let face_id = self.active_face.face_id();
        SingleDisplayItemAppendContext::new(self.active_face.resolved_face(), face_id, frame)
            .render_with_policy(state, item, position, fallback_kind, render_policy)
    }

    fn append_source_char_plan_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: DisplaySourceTextCharAppendPlan,
    ) -> Option<DisplayRowAppendProgress> {
        let position = plan.position();
        let source_text = plan.source_text();
        let fallback_kind = source_text.source_item().append_kind();
        let mut render_policy = source_text.append_render_policy();
        self.append_source_display_item_to_text_row(
            geometry,
            state,
            plan.source_item,
            position,
            fallback_kind,
            &mut render_policy,
        )
    }
}

impl<'source, 'surface, B: LayoutBufferView + ?Sized> DisplaySourceCharAppendContext
    for BufferSourceRowAppendContext<'source, 'surface, B>
{
    fn append_source_char_plan_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: DisplaySourceTextCharAppendPlan,
    ) -> Option<DisplayRowAppendProgress> {
        BufferSourceRowAppendContext::append_source_char_plan_to_text_row(
            self, geometry, state, plan,
        )
    }

    fn append_special_source_char_plan_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: DisplaySourceSpecialCharAppendPlan,
    ) -> Option<DisplayRowAppendProgress> {
        BufferSourceRowAppendContext::append_special_source_char_plan_to_text_row_and_emit(
            self, geometry, state, plan,
        )
    }
}

fn matching_special_display_item(
    source_item: &DisplayItem,
    kind: DisplaySourceSpecialDisplayKind,
) -> Option<&DisplayItem> {
    match (&source_item.kind, kind) {
        (DisplayItemKind::ControlChar { .. }, DisplaySourceSpecialDisplayKind::Control)
        | (DisplayItemKind::Glyphless(_), DisplaySourceSpecialDisplayKind::Glyphless)
        | (DisplayItemKind::SourceMappedText(_), DisplaySourceSpecialDisplayKind::Nobreak) => {
            Some(source_item)
        }
        _ => None,
    }
}

pub(crate) fn buffer_source_item_append_request<B: LayoutBufferView + ?Sized>(
    source_item: DisplaySourceItemRequest,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<DisplaySourceRangeItemAppendRequest> {
    let append_kind = source_item.append_kind();
    let item = source_item.into_display_item(buffer_id, buffer, RenderFaceRef::FaceId(face_id))?;
    Some(DisplaySourceRangeItemAppendRequest::new(item, append_kind))
}

#[cfg(test)]
pub(crate) struct BufferSourceRequestAppendContext<'a, B: LayoutBufferView + ?Sized> {
    buffer: &'a B,
    buffer_id: BufferId,
    item_context: DisplaySourceItemAppendContext<'a>,
}

#[cfg(test)]
impl<'a, B: LayoutBufferView + ?Sized> BufferSourceRequestAppendContext<'a, B> {
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
            item_context: DisplaySourceItemAppendContext::new(face_id, base_face, frame),
        }
    }

    pub(crate) fn append_source_request_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        source_item: DisplaySourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        let append_item = buffer_source_item_append_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.item_context.face_id(),
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        self.item_context
            .single_item
            .render_naturally(state, item, position, kind)
    }

    pub(crate) fn try_measure_source_request_width_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: DisplaySourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        let append_item = buffer_source_item_append_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.item_context.face_id(),
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        self.item_context
            .single_item
            .measure_width_naturally(state, item, position, kind)
    }

    pub(crate) fn measure_source_request_width_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: DisplaySourceItemRequest,
        position: DisplayRowPosition,
    ) -> f32 {
        let fallback_width = source_item.fallback_width();
        let Some(append_item) = buffer_source_item_append_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.item_context.face_id(),
        ) else {
            return fallback_width.resolve_to_text_row(self.item_context.frame());
        };
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        self.item_context
            .single_item
            .measure_width_with_source_fallback(state, item, position, kind, fallback_width)
    }
}
