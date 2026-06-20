use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info, update_cursor_info_for_main_char,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_row::{DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy};
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendFrame, DisplayRowAppendKind,
    DisplayRowAppendSurface,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowTextPosition};
use crate::display_row_source_append::SingleDisplayItemAppendContext;
use crate::display_row_source_render::{TextRowSourceMeasureState, TextRowSourceRenderState};
use crate::display_row_walk_state::{
    DisplayRowTextOverflowDecision, FaceScanCheckpoint, SpecialTextRowOverflowDecision,
    TrailingWhitespaceRenderState, WordWrapRenderState,
};
use crate::display_source::{
    DisplaySourceAppendContinuation, DisplaySourceAppendItem, DisplaySourceClusterState,
    DisplaySourceItemRequest, DisplaySourceNaturalMeasurementRequest,
    DisplaySourceRangeItemAppendRequest, DisplaySourceRenderPlanRequest,
    DisplaySourceSpecialDisplayKind, DisplaySourceStepChar, DisplaySourceTextChar,
    DisplaySourceTextItemRequest, DisplaySourceTextRange, DisplaySourceTextRequest,
    DisplaySpecialSourceCharRequest,
};
use crate::display_source_append_plan::{
    DisplaySourceAppendMeasurementKind, DisplaySourceAppendRenderPlan,
    DisplaySourceAppendRenderPolicy,
};
use crate::display_source_overflow::{
    DisplaySourceSpecialCharOverflowAction, DisplaySourceTextCharOverflowAction,
};
use crate::display_source_progress::DisplaySourceProgressState;
use crate::display_text_run_measurement::ComplexTextRunAdvanceResolver;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace};
use crate::types::{LineWrapMode, WindowParams};
use crate::window_output::WindowOutputEmitter;
use neovm_core::buffer::BufferId;

impl DisplaySourceTextRequest {
    fn append_render_policy(self) -> DisplaySourceAppendRenderPolicy {
        self.render_plan().render_policy()
    }

    #[cfg(test)]
    pub(crate) fn append_request<B: LayoutBufferView + ?Sized>(
        self,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
    ) -> Option<DisplaySourceRangeItemAppendRequest> {
        buffer_text_source_text_item_append_request(self.source_item(), buffer_id, buffer, face_id)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct DisplaySourceAppendRenderPlanResolver {
    complex_run: ComplexTextRunAdvanceResolver,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BufferTextRowAppendState {
    render_plan_resolver: DisplaySourceAppendRenderPlanResolver,
}

impl BufferTextRowAppendState {
    fn render_plan_resolver(&mut self) -> &mut DisplaySourceAppendRenderPlanResolver {
        &mut self.render_plan_resolver
    }
}

impl DisplaySourceNaturalMeasurementRequest {
    #[allow(clippy::too_many_arguments)]
    fn measure_to_text_row(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        base_face: &ResolvedFace,
        face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
    ) -> Option<f32> {
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        SingleDisplayItemAppendContext::new(base_face, face_id, frame).measure_width_with_policy(
            state,
            source_item.clone(),
            position,
            self.source_item().append_kind(),
            &mut render_policy,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_to_text_row(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
    ) -> f32 {
        if let Some(measured_width) = self.measure_to_text_row(
            state,
            active_face_state.resolved_face(),
            active_face_state.face_id(),
            frame.clone(),
            position,
            source_item,
        ) {
            return measured_width;
        }

        self.fallback().resolve_to_text_row(
            state.font_metrics(),
            active_face_state,
            &frame,
            position,
            self.source_item().source_char(),
        )
    }
}

#[cfg(test)]
pub(crate) fn buffer_text_source_text_item_append_request<B: LayoutBufferView + ?Sized>(
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
pub(crate) struct BufferTextRowAppendContext<'source, 'surface, B: LayoutBufferView + ?Sized> {
    buffer: &'source B,
    buffer_id: BufferId,
    append_surface: &'surface DisplayRowAppendSurface,
    active_face: &'source DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

impl<'source, 'surface, B: LayoutBufferView + ?Sized>
    BufferTextRowAppendContext<'source, 'surface, B>
{
    pub(crate) fn new(
        buffer: &'source B,
        buffer_id: BufferId,
        append_surface: &'surface DisplayRowAppendSurface,
        active_face: &'source DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            buffer,
            buffer_id,
            append_surface,
            active_face,
            glyph_y_offset,
            default_row_height,
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
            self.default_row_height,
        )
    }

    fn item_active_face(
        &self,
        geometry: &DisplayRowGeometryState,
    ) -> BufferTextItemAppendContext<'source> {
        let frame = self.active_face_context(geometry).active_face_frame();
        BufferTextItemAppendContext::new(
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

        buffer_text_source_item_append_request(
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
        plan: BufferTextSpecialSourceCharAppendPlan,
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
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        request: DisplaySourceTextPositionedRenderPlanRequest<'_, '_>,
    ) -> DisplaySourceAppendRenderPlan {
        let frame = self.active_face_context(geometry).active_face_frame();
        append_state
            .render_plan_resolver()
            .resolve_source_render_plan_request_to_text_row(
                measure_state,
                self.active_face,
                frame,
                request,
            )
    }

    fn prepare_source_char_append_plan(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        request: DisplaySourceTextPositionedRenderPlanRequest<'_, '_>,
    ) -> BufferTextSourceCharAppendPlan {
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
        append_state: &mut BufferTextRowAppendState,
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
        append_state: &mut BufferTextRowAppendState,
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
        append_state: &mut BufferTextRowAppendState,
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
        plan: BufferTextSourceCharAppendPlan,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplaySourcePreparedCharAppend {
    Special(DisplaySourceSpecialCharPreparedAppend),
    Text(DisplaySourceTextCharPreparedAppend),
}

impl DisplaySourcePreparedCharAppend {
    #[cfg(test)]
    pub(crate) fn into_text(self) -> Option<DisplaySourceTextCharPreparedAppend> {
        match self {
            Self::Text(prepared_append) => Some(prepared_append),
            Self::Special(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplaySourceTextCharPreparedAppend {
    pub(crate) plan: BufferTextSourceCharAppendPlan,
}

impl DisplaySourceTextCharPreparedAppend {
    fn advance_px(&self) -> f32 {
        self.plan.advance_px()
    }

    pub(crate) fn update_cursor_info_for_main_char(
        &self,
        target: &mut CursorCaptureState,
        byte_idx: usize,
    ) {
        update_cursor_info_for_main_char(target, byte_idx, self.advance_px());
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn capture_cursor_info_for_main_char_if_point(
        &self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        geometry: &DisplayRowGeometryState,
        x_px: f32,
        byte_idx: usize,
        col: usize,
        is_tab: bool,
        charpos: i64,
        point_charpos: i64,
    ) {
        if target.is_missing() && charpos == point_charpos {
            capture_cursor_info(
                target,
                self.cursor_info_for_main_char(
                    active_face_state,
                    geometry.text_position(x_px, byte_idx, col),
                    is_tab,
                ),
            );
        }
    }

    pub(crate) fn overflow_decision(
        &self,
        ch: char,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
        word_wrap: WordWrapRenderState,
    ) -> DisplayRowTextOverflowDecision {
        DisplayRowTextOverflowDecision::for_char(
            ch,
            self.plan.position.x_px,
            self.advance_px(),
            right_edge_px,
            wrap_mode,
            word_wrap,
        )
    }

    pub(crate) fn overflow_action(
        &self,
        ch: char,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
        word_wrap: WordWrapRenderState,
    ) -> DisplaySourceTextCharOverflowAction {
        DisplaySourceTextCharOverflowAction::for_decision(self.overflow_decision(
            ch,
            right_edge_px,
            wrap_mode,
            word_wrap,
        ))
    }

    fn cursor_slot_width(&self) -> CapturedCursorSlotWidth {
        CapturedCursorSlotWidth::Explicit(self.advance_px())
    }

    pub(crate) fn cursor_info_for_main_char(
        &self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
        is_tab: bool,
    ) -> CapturedCursorInfo {
        CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                position,
                self.cursor_slot_width(),
                is_tab,
            ),
        )
    }

    pub(crate) fn append_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
    ) -> Option<DisplaySourceTextCharAppendOutcome> {
        let progress = context.append_source_char_plan_to_text_row(geometry, state, self.plan)?;
        Some(DisplaySourceTextCharAppendOutcome { progress })
    }

    pub(crate) fn append_to_text_row_and_apply<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        ch: char,
        source_render: &mut TextRowSourceRenderState<'_>,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        word_wrap: &mut WordWrapRenderState,
        progress: &mut DisplaySourceProgressState<'_>,
    ) -> DisplaySourceAppendContinuation {
        let Some(outcome) = self.append_to_text_row(context, geometry, source_render) else {
            return DisplaySourceAppendContinuation::Stopped;
        };
        outcome.apply_rendered_char_to_walk_state(
            trailing_whitespace,
            word_wrap,
            ch,
            geometry,
            progress,
        );
        DisplaySourceAppendContinuation::Rendered
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplaySourceTextCharAppendOutcome {
    progress: DisplayRowAppendProgress,
}

impl DisplaySourceTextCharAppendOutcome {
    pub(crate) fn apply_to_text_row_state(
        &self,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        ch: char,
        geometry: &DisplayRowGeometryState,
        x: &mut f32,
        col: &mut usize,
    ) {
        trailing_whitespace
            .track_rendered_char(ch, geometry.start_marker_at_x(self.progress.start.x_px));
        *x = self.progress.end.x_px;
        *col = self.progress.end.col;
    }

    pub(crate) fn apply_rendered_char_to_walk_state(
        &self,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        word_wrap: &mut WordWrapRenderState,
        ch: char,
        geometry: &DisplayRowGeometryState,
        progress: &mut DisplaySourceProgressState<'_>,
    ) {
        self.apply_to_text_row_state(
            trailing_whitespace,
            ch,
            geometry,
            progress.row.x,
            progress.row.col,
        );
        *progress.charpos += 1;
        word_wrap.allow_after_current_char(ch);
    }
}

impl DisplaySourceAppendRenderPlanResolver {
    fn resolve_source_render_plan_request_to_text_row(
        &mut self,
        state: &mut TextRowSourceMeasureState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        request: DisplaySourceTextPositionedRenderPlanRequest<'_, '_>,
    ) -> DisplaySourceAppendRenderPlan {
        let ch = request.cluster().ch();
        match request.measurement_kind() {
            DisplaySourceAppendMeasurementKind::ResolvedComplexRun => {
                let mut policy = DisplayRowComplexTextRunAdvancePolicy::new(
                    active_face_state,
                    state.font_metrics(),
                );
                let advance_px = self.complex_run.advance_for_char(
                    request.text(),
                    request.byte_idx(),
                    ch,
                    request.cluster().is_cluster_continuation(),
                    &mut policy,
                );
                DisplaySourceAppendRenderPlan::resolved_advance(advance_px)
            }
            DisplaySourceAppendMeasurementKind::NaturalRenderedSource => {
                let advance_px = DisplaySourceNaturalMeasurementRequest::for_range_and_cluster(
                    request.range(),
                    request.cluster(),
                )
                .resolve_to_text_row(
                    state,
                    active_face_state,
                    frame,
                    request.position(),
                    request.source_item(),
                );
                DisplaySourceAppendRenderPlan::natural(advance_px)
            }
        }
    }
}

pub(crate) fn buffer_text_source_item_append_request<B: LayoutBufferView + ?Sized>(
    source_item: DisplaySourceItemRequest,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<DisplaySourceRangeItemAppendRequest> {
    let append_kind = source_item.append_kind();
    let item = source_item.into_display_item(buffer_id, buffer, RenderFaceRef::FaceId(face_id))?;
    Some(DisplaySourceRangeItemAppendRequest::new(item, append_kind))
}

impl DisplaySourceStepChar {
    pub(crate) fn record_word_wrap_candidate(
        self,
        word_wrap: &mut WordWrapRenderState,
        output_emitter: &WindowOutputEmitter,
    ) {
        if word_wrap.can_record_candidate(self.ch()) {
            word_wrap.record_candidate(
                self.ch(),
                self.start_byte_idx(),
                self.start_charpos(),
                output_emitter.display_point_len(),
                output_emitter.current_row_display_positions(),
            );
        }
    }
}

impl DisplaySourceTextChar {
    fn render_plan_request_for_item_at<'text, 'item>(
        &self,
        text: &'text [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        source_item: &'item DisplayItem,
        tail: Option<(char, bool)>,
    ) -> DisplaySourceTextPositionedRenderPlanRequest<'text, 'item> {
        DisplaySourceTextPositionedRenderPlanRequest::new(
            self.advance_request(text, byte_idx, tail),
            position,
            source_item,
        )
    }
}

impl DisplaySpecialSourceCharRequest {
    pub(crate) fn append_plan_at(
        &self,
        position: DisplayRowPosition,
        display_item: DisplayItem,
    ) -> BufferTextSpecialSourceCharAppendPlan {
        BufferTextSpecialSourceCharAppendPlan {
            source_item: self.source_item_request(),
            position,
            display_item,
        }
    }

    fn prepared_append_at(
        self,
        position: DisplayRowPosition,
        measured_width_px: Option<f32>,
        display_item: DisplayItem,
    ) -> DisplaySourceSpecialCharPreparedAppend {
        DisplaySourceSpecialCharPreparedAppend {
            kind: self.kind(),
            append_plan: self.append_plan_at(position, display_item),
            measured_width_px,
        }
    }
}

impl DisplaySourceSpecialDisplayKind {
    fn should_allocate_policy_face(self, params: &WindowParams) -> bool {
        match self {
            Self::Control => params.escape_glyph_fg != 0,
            Self::Nobreak => params.nobreak_char_fg != 0,
            Self::Glyphless => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisplaySourceSpecialCharPreparedAppend {
    pub(crate) kind: DisplaySourceSpecialDisplayKind,
    pub(crate) append_plan: BufferTextSpecialSourceCharAppendPlan,
    pub(crate) measured_width_px: Option<f32>,
}

impl DisplaySourceSpecialCharPreparedAppend {
    #[cfg(test)]
    pub(crate) fn kind(&self) -> DisplaySourceSpecialDisplayKind {
        self.kind
    }

    fn prepare_append_policy(
        &self,
        params: &WindowParams,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> BufferTextSpecialSourceCharAppendPolicy {
        if self.kind.should_allocate_policy_face(params) {
            let _ = face_ids.allocate();
        }
        BufferTextSpecialSourceCharAppendPolicy {
            invalidate_face_after_append: self.kind.invalidates_face_after_append(),
        }
    }

    fn measured_width_px(&self) -> Option<f32> {
        self.measured_width_px
    }

    pub(crate) fn overflow_decision(
        &self,
        x_px: f32,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
    ) -> Option<SpecialTextRowOverflowDecision> {
        Some(SpecialTextRowOverflowDecision::for_width(
            x_px,
            self.measured_width_px()?,
            right_edge_px,
            wrap_mode,
        ))
    }

    pub(crate) fn overflow_action(
        &self,
        x_px: f32,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
    ) -> Option<DisplaySourceSpecialCharOverflowAction> {
        Some(DisplaySourceSpecialCharOverflowAction::for_decision(
            self.overflow_decision(x_px, right_edge_px, wrap_mode)?,
        ))
    }

    pub(crate) fn append_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        params: &WindowParams,
        face_ids: &mut FrameFaceIdAllocator,
        state: &mut TextRowSourceRenderState<'_>,
    ) -> Option<BufferTextSpecialSourceCharAppendOutcome> {
        let append_policy = self.prepare_append_policy(params, face_ids);
        let progress = context.append_special_source_char_plan_to_text_row_and_emit(
            geometry,
            state,
            self.append_plan,
        )?;
        Some(BufferTextSpecialSourceCharAppendOutcome {
            progress,
            append_policy,
        })
    }

    pub(crate) fn append_to_text_row_and_apply<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        params: &WindowParams,
        face_ids: &mut FrameFaceIdAllocator,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_scan: &mut FaceScanCheckpoint,
        word_wrap: &mut WordWrapRenderState,
        progress: &mut DisplaySourceProgressState<'_>,
    ) -> DisplaySourceAppendContinuation {
        let Some(outcome) =
            self.append_to_text_row(context, geometry, params, face_ids, source_render)
        else {
            return DisplaySourceAppendContinuation::Stopped;
        };
        outcome.apply_rendered_special_char_to_walk_state(face_scan, word_wrap, progress);
        DisplaySourceAppendContinuation::Rendered
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferTextSpecialSourceCharAppendPolicy {
    invalidate_face_after_append: bool,
}

impl BufferTextSpecialSourceCharAppendPolicy {
    fn invalidates_face_after_append(self) -> bool {
        self.invalidate_face_after_append
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharAppendOutcome {
    progress: DisplayRowAppendProgress,
    append_policy: BufferTextSpecialSourceCharAppendPolicy,
}

impl BufferTextSpecialSourceCharAppendOutcome {
    pub(crate) fn apply_to_text_row_state(
        &self,
        face_scan: &mut FaceScanCheckpoint,
        x: &mut f32,
        col: &mut usize,
    ) {
        if self.append_policy.invalidates_face_after_append() {
            face_scan.invalidate();
        }
        *x = self.progress.end.x_px;
        *col = self.progress.end.col;
    }

    pub(crate) fn apply_rendered_special_char_to_walk_state(
        &self,
        face_scan: &mut FaceScanCheckpoint,
        word_wrap: &mut WordWrapRenderState,
        progress: &mut DisplaySourceProgressState<'_>,
    ) {
        self.apply_to_text_row_state(face_scan, progress.row.x, progress.row.col);
        *progress.charpos += 1;
        word_wrap.disallow_after_current_char();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharAppendPlan {
    pub(crate) source_item: DisplaySourceItemRequest,
    pub(crate) position: DisplayRowPosition,
    pub(crate) display_item: DisplayItem,
}

impl BufferTextSpecialSourceCharAppendPlan {
    fn position(&self) -> DisplayRowPosition {
        self.position
    }

    fn source_item(&self) -> DisplaySourceItemRequest {
        self.source_item.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharAppendPlan {
    pub(crate) source_text: DisplaySourceTextRequest,
    pub(crate) position: DisplayRowPosition,
    pub(crate) source_item: DisplayItem,
}

impl BufferTextSourceCharAppendPlan {
    fn source_text(&self) -> DisplaySourceTextRequest {
        self.source_text
    }

    fn position(&self) -> DisplayRowPosition {
        self.position
    }

    fn advance_px(&self) -> f32 {
        self.source_text.advance_px()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplaySourceTextPositionedRenderPlanRequest<'text, 'item> {
    source: DisplaySourceRenderPlanRequest<'text>,
    position: DisplayRowPosition,
    source_item: &'item DisplayItem,
}

impl<'text, 'item> DisplaySourceTextPositionedRenderPlanRequest<'text, 'item> {
    pub(crate) fn new(
        source: DisplaySourceRenderPlanRequest<'text>,
        position: DisplayRowPosition,
        source_item: &'item DisplayItem,
    ) -> Self {
        Self {
            source,
            position,
            source_item,
        }
    }

    fn text(self) -> &'text [u8] {
        self.source.text()
    }

    fn byte_idx(self) -> usize {
        self.source.byte_idx()
    }

    fn range(self) -> DisplaySourceTextRange {
        self.source.range()
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn cluster(self) -> DisplaySourceClusterState {
        self.source.cluster()
    }

    fn measurement_kind(self) -> DisplaySourceAppendMeasurementKind {
        self.source.measurement_kind()
    }

    fn source_item(self) -> &'item DisplayItem {
        self.source_item
    }

    fn append_plan(
        self,
        render_plan: DisplaySourceAppendRenderPlan,
    ) -> BufferTextSourceCharAppendPlan {
        BufferTextSourceCharAppendPlan {
            source_text: self.source.into_text_request(render_plan),
            position: self.position,
            source_item: self.source_item.clone(),
        }
    }
}

impl DisplaySourceAppendItem {
    fn append_kind(&self) -> DisplayRowAppendKind {
        match self {
            Self::ControlChar { .. } => DisplayRowAppendKind::ControlChar,
            Self::SourceMappedText { .. } => DisplayRowAppendKind::SourceMappedText,
            Self::Glyphless { .. } => DisplayRowAppendKind::Glyphless,
        }
    }
}

impl DisplaySourceTextItemRequest {
    fn append_kind(self) -> DisplayRowAppendKind {
        if self.source_char() == '\t' {
            DisplayRowAppendKind::Tab
        } else {
            DisplayRowAppendKind::SourceText
        }
    }
}

impl DisplaySourceItemRequest {
    fn append_kind(&self) -> DisplayRowAppendKind {
        self.item().append_kind()
    }
}

pub(crate) struct BufferTextItemAppendContext<'a> {
    single_item: SingleDisplayItemAppendContext<'a>,
}

impl<'a> BufferTextItemAppendContext<'a> {
    pub(crate) fn new(
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            single_item: SingleDisplayItemAppendContext::new(base_face, face_id, frame),
        }
    }

    #[cfg(test)]
    fn face_id(&self) -> u32 {
        self.single_item.face_id()
    }

    #[cfg(test)]
    fn frame(&self) -> &DisplayRowAppendFrame {
        self.single_item.frame()
    }

    pub(crate) fn append_display_item_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
    ) -> Option<DisplayRowAppendProgress> {
        self.single_item
            .render_naturally(state, item, position, fallback_kind)
    }

    pub(crate) fn measure_source_display_item_width_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: &DisplayItem,
        source_item: DisplaySourceItemRequest,
        position: DisplayRowPosition,
    ) -> f32 {
        self.single_item.measure_width_with_source_fallback(
            state,
            item.clone(),
            position,
            source_item.append_kind(),
            source_item.fallback_width(),
        )
    }
}

#[cfg(test)]
pub(crate) struct BufferTextSourceRequestAppendContext<'a, B: LayoutBufferView + ?Sized> {
    buffer: &'a B,
    buffer_id: BufferId,
    item_context: BufferTextItemAppendContext<'a>,
}

#[cfg(test)]
impl<'a, B: LayoutBufferView + ?Sized> BufferTextSourceRequestAppendContext<'a, B> {
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
            item_context: BufferTextItemAppendContext::new(face_id, base_face, frame),
        }
    }

    pub(crate) fn append_source_request_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        source_item: DisplaySourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        let append_item = buffer_text_source_item_append_request(
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
        let append_item = buffer_text_source_item_append_request(
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
        let Some(append_item) = buffer_text_source_item_append_request(
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
