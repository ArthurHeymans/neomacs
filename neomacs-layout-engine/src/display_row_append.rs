use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info, update_cursor_info_for_main_char,
};
use crate::display_face_id::FrameFaceIdAllocator;

use crate::display_buffer_text_render::{
    BufferTextSourceAppendContinuation, BufferTextSourceCharOverflowAction,
    BufferTextSourceCharRenderState, BufferTextSpecialSourceCharOverflowAction,
    BufferTextSpecialSourceCharRenderState, SyntheticTextSource,
};
use crate::display_buffer_text_source::BufferTextDecodedSourceChar;
use crate::display_item::{DisplayItem, RenderFaceRef};
#[cfg(test)]
use crate::display_row::DisplayRowRenderStop;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_text_row_and_emit;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy,
    DisplayRowGeometry, DisplayRowMeasuredFaceMetrics, DisplayRowRenderBounds,
    DisplayRowRenderPolicy, DisplayRowSourceAppendRequest, DisplayRowSourceAppendRequestPolicy,
    DisplayRowSourceState, DisplaySourceAppendRenderPolicy, NaturalDisplayRowAppendRenderPolicy,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition, DisplayTabPolicy};
use crate::display_row_geometry::{
    DisplayRowGeometryState, DisplayRowMaxX, DisplayRowTextPosition,
};
use crate::display_row_lisp_string::render_face_ref_id;
use crate::display_row_source_render::{
    TextRowSourceMeasureState, TextRowSourceRenderState, current_text_measure_state,
    current_text_render_state,
};
use crate::display_row_walk_state::{
    BufferTextRowOverflowDecision, FaceScanCheckpoint, SpecialTextRowOverflowDecision,
    TrailingWhitespaceRenderState, WordWrapRenderState,
};
use crate::display_source::{
    BufferTextSourceAdvancePath, BufferTextSourceAdvanceRequest, BufferTextSourceAppendItem,
    BufferTextSourceChar, BufferTextSourceClusterState, BufferTextSourceItemRequest,
    BufferTextSourceNaturalAdvanceRequest, BufferTextSourceNaturalFallbackAdvance,
    BufferTextSourceRange, BufferTextSourceSpecialDisplayKind, BufferTextSourceTextItemRequest,
    BufferTextSourceTextRequest, BufferTextSpecialSourceCharRequest, DisplayItemSource,
    ResolvedBufferTextSourceAdvance,
};
use crate::display_text_run_measurement::ComplexTextRunAdvanceResolver;
use crate::font_metrics::FontMetricsService;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace};
use crate::types::LineWrapMode;
use crate::types::WindowParams;
use crate::window_output::{WindowOutputEmitter, current_text_window_cluster_tail};
use neovm_core::buffer::BufferId;

impl ResolvedBufferTextSourceAdvance {
    fn append_render_policy(self) -> DisplaySourceAppendRenderPolicy {
        match self {
            Self::Natural { .. } => DisplaySourceAppendRenderPolicy::natural(),
            Self::Resolved { advance_px } => {
                DisplaySourceAppendRenderPolicy::resolved_advance(advance_px)
            }
        }
    }
}

impl BufferTextSourceTextRequest {
    fn append_render_policy(self) -> DisplaySourceAppendRenderPolicy {
        self.resolved_advance().append_render_policy()
    }

    fn append_request<B: LayoutBufferView + ?Sized>(
        self,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
    ) -> Option<BufferTextSourceRangeItemAppendRequest> {
        buffer_text_source_text_item_append_request(self.source_item(), buffer_id, buffer, face_id)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct BufferTextSourceAdvanceResolver {
    complex_run: ComplexTextRunAdvanceResolver,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BufferTextRowAppendState {
    advance_resolver: BufferTextSourceAdvanceResolver,
}

impl BufferTextRowAppendState {
    fn advance_resolver(&mut self) -> &mut BufferTextSourceAdvanceResolver {
        &mut self.advance_resolver
    }
}

impl BufferTextSourceNaturalFallbackAdvance {
    fn resolve_to_text_row(
        self,
        font_metrics: &mut Option<FontMetricsService>,
        active_face_state: &DisplayRowActiveFaceState,
        frame: &DisplayRowAppendFrame,
        position: DisplayRowPosition,
        ch: char,
    ) -> f32 {
        match self {
            Self::Tab => {
                frame
                    .geometry
                    .tab_policy
                    .advance_from(position, frame.face_space_width)
                    .pixel_width
            }
            Self::ClusterContinuation => 0.0,
            Self::FaceColumns { columns } => {
                active_face_state.advance_for_columns(font_metrics, ch, columns)
            }
        }
    }
}

impl BufferTextSourceNaturalAdvanceRequest {
    #[allow(clippy::too_many_arguments)]
    fn measure_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        base_face: &ResolvedFace,
        buffer_id: BufferId,
        buffer: &B,
        face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        let append_item = buffer_text_source_text_item_append_request(
            self.source_item(),
            buffer_id,
            buffer,
            face_id,
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        Some(
            DisplayRowSourceAppendOperation::for_single_item(
                &item, base_face, face_id, frame, position, kind,
            )
            .measure_single_item_to_text_row(state, item, &mut render_policy)?
            .metrics
            .width_px,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_to_text_row<B: LayoutBufferView + ?Sized>(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        buffer_id: BufferId,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> f32 {
        if let Some(measured_width) = self.measure_to_text_row(
            state,
            active_face_state.resolved_face(),
            buffer_id,
            buffer,
            active_face_state.face_id(),
            frame.clone(),
            position,
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

#[derive(Clone, Debug, PartialEq)]
struct BufferTextSourceRangeItemAppendRequest {
    item: DisplayItem,
    append_kind: DisplayRowAppendKind,
}

impl BufferTextSourceRangeItemAppendRequest {
    fn new(item: DisplayItem, append_kind: DisplayRowAppendKind) -> Self {
        Self { item, append_kind }
    }

    fn append_kind(&self) -> DisplayRowAppendKind {
        self.append_kind
    }

    fn into_item(self) -> DisplayItem {
        self.item
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

    fn measure_single_item_to_text_row<P: DisplayRowRenderPolicy>(
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

fn buffer_text_source_text_item_append_request<B: LayoutBufferView + ?Sized>(
    source_item: BufferTextSourceTextItemRequest,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<BufferTextSourceRangeItemAppendRequest> {
    let append_kind = source_item.append_kind();
    let item = source_item.into_display_item(buffer_id, buffer, RenderFaceRef::FaceId(face_id))?;
    Some(BufferTextSourceRangeItemAppendRequest::new(
        item,
        append_kind,
    ))
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
    ) -> BufferTextItemAppendContext<'source, B> {
        let frame = self.active_face_context(geometry).active_face_frame();
        BufferTextItemAppendContext::new(
            self.buffer,
            self.buffer_id,
            self.active_face.face_id(),
            self.active_face.resolved_face(),
            frame,
        )
    }

    fn measure_item_source_request_width_or_item_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> f32 {
        self.item_active_face(geometry)
            .measure_source_request_width_or_item_fallback_to_text_row(state, source_item, position)
    }

    fn measure_special_source_char_request_width_or_item_fallback_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSpecialSourceCharMeasureRequest,
    ) -> f32 {
        let position = request.position();
        self.measure_item_source_request_width_or_item_fallback_to_text_row(
            geometry,
            state,
            request.source_item(),
            position,
        )
    }

    pub(crate) fn prepare_special_source_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSpecialSourceCharRequest,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharPreparedAppend {
        let measured_width_px = request.requires_overflow_measurement().then(|| {
            self.measure_special_source_char_request_width_or_item_fallback_to_text_row(
                geometry,
                state,
                request.measure_at(position),
            )
        });
        request.prepared_append_at(position, measured_width_px)
    }

    fn append_item_source_request_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.item_active_face(geometry)
            .append_source_request_to_text_row_and_emit(state, source_item, position)
    }

    fn append_special_source_char_plan_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: BufferTextSpecialSourceCharAppendPlan,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let position = plan.position();
        self.append_item_source_request_to_text_row_and_emit(
            geometry,
            state,
            plan.source_item(),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_source_advance_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSourcePositionedAdvanceRequest<'_>,
    ) -> ResolvedBufferTextSourceAdvance {
        let frame = self.active_face_context(geometry).active_face_frame();
        append_state
            .advance_resolver()
            .resolve_source_advance_request_to_text_row(
                measure_state,
                self.buffer_id,
                self.buffer,
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
        request: BufferTextSourcePositionedAdvanceRequest<'_>,
    ) -> BufferTextSourceCharAppendPlan {
        let resolved_advance = self.resolve_source_advance_request_to_text_row(
            geometry,
            append_state,
            measure_state,
            request,
        );
        request.append_plan(resolved_advance)
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_text_source_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        source_char: &BufferTextSourceChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        cluster_tail: Option<(char, bool)>,
    ) -> BufferTextSourceCharPreparedAppend {
        let request = source_char.advance_request_at(text, byte_idx, position, cluster_tail);
        BufferTextSourceCharPreparedAppend {
            plan: self.prepare_source_char_append_plan(
                geometry,
                append_state,
                measure_state,
                request,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_source_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        source_char: &BufferTextSourceChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        cluster_tail: Option<(char, bool)>,
    ) -> BufferTextPreparedSourceCharAppend {
        if let Some(request) = source_char.special_request(cluster_tail) {
            return BufferTextPreparedSourceCharAppend::Special(
                self.prepare_special_source_char_at(geometry, measure_state, request, position),
            );
        }
        BufferTextPreparedSourceCharAppend::Text(self.prepare_text_source_char_at(
            geometry,
            append_state,
            measure_state,
            source_char,
            text,
            byte_idx,
            position,
            cluster_tail,
        ))
    }

    pub(crate) fn prepare_source_char_for_current_text_row(
        &self,
        request: BufferTextSourceCharPreparationRequest<'_>,
        state: &mut BufferTextSourceCharPreparationState<'_>,
    ) -> BufferTextPreparedSourceCharAppend {
        let cluster_tail = current_text_window_cluster_tail(state.measure.builder);
        self.prepare_source_char_at(
            &request.geometry,
            state.append_state,
            &mut state.measure,
            request.source_char,
            request.text,
            request.byte_idx,
            request.position,
            cluster_tail,
        )
    }

    fn append_source_text_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        source_text: BufferTextSourceTextRequest,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let frame = self.active_face_context(geometry).active_face_frame();
        let face_id = self.active_face.face_id();
        let append_item = source_text.append_request(self.buffer_id, self.buffer, face_id)?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = source_text.append_render_policy();
        DisplayRowSourceAppendOperation::for_single_item(
            &item,
            self.active_face.resolved_face(),
            face_id,
            frame,
            position,
            kind,
        )
        .render_single_item_to_text_row_and_emit(state, item, &mut render_policy)
    }

    fn append_source_char_plan_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: BufferTextSourceCharAppendPlan,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        self.append_source_text_request_to_text_row(
            geometry,
            state,
            plan.source_text(),
            plan.position(),
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSourceCharPreparationRequest<'a> {
    geometry: DisplayRowGeometryState,
    source_char: &'a BufferTextSourceChar,
    text: &'a [u8],
    byte_idx: usize,
    position: DisplayRowPosition,
}

impl<'a> BufferTextSourceCharPreparationRequest<'a> {
    pub(crate) fn new(
        geometry: DisplayRowGeometryState,
        source_char: &'a BufferTextSourceChar,
        text: &'a [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
    ) -> Self {
        Self {
            geometry,
            source_char,
            text,
            byte_idx,
            position,
        }
    }
}

pub(crate) struct BufferTextSourceCharPreparationState<'a> {
    append_state: &'a mut BufferTextRowAppendState,
    measure: TextRowSourceMeasureState<'a>,
}

impl<'a> BufferTextSourceCharPreparationState<'a> {
    pub(crate) fn from_source_render(
        append_state: &'a mut BufferTextRowAppendState,
        source_render: &'a mut TextRowSourceRenderState<'_>,
    ) -> Self {
        Self {
            append_state,
            measure: source_render.measure_state(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextPreparedSourceCharAppend {
    Special(BufferTextSpecialSourceCharPreparedAppend),
    Text(BufferTextSourceCharPreparedAppend),
}

impl BufferTextPreparedSourceCharAppend {
    #[cfg(test)]
    pub(crate) fn into_text(self) -> Option<BufferTextSourceCharPreparedAppend> {
        match self {
            Self::Text(prepared_append) => Some(prepared_append),
            Self::Special(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharPreparedAppend {
    plan: BufferTextSourceCharAppendPlan,
}

impl BufferTextSourceCharPreparedAppend {
    fn advance_px(self) -> f32 {
        self.plan.advance_px()
    }

    pub(crate) fn update_cursor_info_for_main_char(
        self,
        target: &mut CursorCaptureState,
        byte_idx: usize,
    ) {
        update_cursor_info_for_main_char(target, byte_idx, self.advance_px());
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn capture_cursor_info_for_main_char_if_point(
        self,
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
        self,
        ch: char,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
        word_wrap: WordWrapRenderState,
    ) -> BufferTextRowOverflowDecision {
        BufferTextRowOverflowDecision::for_char(
            ch,
            self.plan.position.x_px,
            self.advance_px(),
            right_edge_px,
            wrap_mode,
            word_wrap,
        )
    }

    pub(crate) fn overflow_action(
        self,
        ch: char,
        right_edge_px: f32,
        wrap_mode: LineWrapMode,
        word_wrap: WordWrapRenderState,
    ) -> BufferTextSourceCharOverflowAction {
        BufferTextSourceCharOverflowAction::for_decision(self.overflow_decision(
            ch,
            right_edge_px,
            wrap_mode,
            word_wrap,
        ))
    }

    fn cursor_slot_width(self) -> CapturedCursorSlotWidth {
        CapturedCursorSlotWidth::Explicit(self.advance_px())
    }

    pub(crate) fn cursor_info_for_main_char(
        self,
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
    ) -> Option<BufferTextSourceCharAppendOutcome> {
        let advance_px = self.advance_px();
        let (progress, position) =
            context.append_source_char_plan_to_text_row(geometry, state, self.plan)?;
        Some(BufferTextSourceCharAppendOutcome {
            progress,
            position,
            advance_px,
        })
    }

    pub(crate) fn append_to_text_row_and_apply<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        ch: char,
        state: &mut BufferTextSourceCharRenderState<'_>,
    ) -> BufferTextSourceAppendContinuation {
        let Some(outcome) = self.append_to_text_row(context, geometry, &mut state.source_render)
        else {
            return BufferTextSourceAppendContinuation::Stopped;
        };
        outcome.apply_rendered_char_to_walk_state(
            state.trailing_whitespace,
            state.word_wrap,
            ch,
            geometry,
            state.x,
            state.col,
            state.charpos,
        );
        BufferTextSourceAppendContinuation::Rendered
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharAppendOutcome {
    progress: DisplayRowAppendProgress,
    position: DisplayRowPosition,
    advance_px: f32,
}

impl BufferTextSourceCharAppendOutcome {
    pub(crate) fn apply_to_text_row_state(
        &self,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        ch: char,
        geometry: &DisplayRowGeometryState,
        x: &mut f32,
        col: &mut usize,
    ) {
        trailing_whitespace.track_rendered_char(
            ch,
            geometry.start_marker_at_x(self.position.x_px - self.advance_px),
        );
        *x = self.position.x_px;
        *col = self.position.col;
    }

    pub(crate) fn apply_rendered_char_to_walk_state(
        &self,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        word_wrap: &mut WordWrapRenderState,
        ch: char,
        geometry: &DisplayRowGeometryState,
        x: &mut f32,
        col: &mut usize,
        charpos: &mut i64,
    ) {
        self.apply_to_text_row_state(trailing_whitespace, ch, geometry, x, col);
        *charpos += 1;
        word_wrap.allow_after_current_char(ch);
    }
}

impl BufferTextSourceAdvanceResolver {
    #[allow(clippy::too_many_arguments)]
    fn resolve_source_advance_request_to_text_row<B: LayoutBufferView + ?Sized>(
        &mut self,
        state: &mut TextRowSourceMeasureState<'_>,
        buffer_id: BufferId,
        buffer: &B,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        request: BufferTextSourcePositionedAdvanceRequest<'_>,
    ) -> ResolvedBufferTextSourceAdvance {
        let ch = request.cluster().ch();
        match BufferTextSourceAdvancePath::for_cluster_state(request.cluster()) {
            BufferTextSourceAdvancePath::ResolvedComplexRun => {
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
                ResolvedBufferTextSourceAdvance::resolved(advance_px)
            }
            BufferTextSourceAdvancePath::NaturalRenderedSource => {
                let advance_px = BufferTextSourceNaturalAdvanceRequest::for_range_and_cluster(
                    request.range(),
                    request.cluster(),
                )
                .resolve_to_text_row(
                    state,
                    buffer_id,
                    buffer,
                    active_face_state,
                    frame,
                    request.position(),
                );
                ResolvedBufferTextSourceAdvance::natural(advance_px)
            }
        }
    }
}

fn buffer_text_source_item_append_request<B: LayoutBufferView + ?Sized>(
    source_item: BufferTextSourceItemRequest,
    buffer_id: BufferId,
    buffer: &B,
    face_id: u32,
) -> Option<BufferTextSourceRangeItemAppendRequest> {
    let append_kind = source_item.append_kind();
    let item = source_item.into_display_item(buffer_id, buffer, RenderFaceRef::FaceId(face_id))?;
    Some(BufferTextSourceRangeItemAppendRequest::new(
        item,
        append_kind,
    ))
}

impl BufferTextDecodedSourceChar {
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

impl BufferTextSourceChar {
    fn advance_request_at<'text>(
        &self,
        text: &'text [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        tail: Option<(char, bool)>,
    ) -> BufferTextSourcePositionedAdvanceRequest<'text> {
        BufferTextSourcePositionedAdvanceRequest::new(
            self.advance_request(text, byte_idx, tail),
            position,
        )
    }
}

impl BufferTextSpecialSourceCharRequest {
    fn append_plan_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharAppendPlan {
        BufferTextSpecialSourceCharAppendPlan {
            source_item: self.source_item_request(),
            position,
        }
    }

    fn prepared_append_at(
        self,
        position: DisplayRowPosition,
        measured_width_px: Option<f32>,
    ) -> BufferTextSpecialSourceCharPreparedAppend {
        BufferTextSpecialSourceCharPreparedAppend {
            kind: self.kind(),
            append_plan: self.append_plan_at(position),
            measured_width_px,
        }
    }

    fn measure_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharMeasureRequest {
        BufferTextSpecialSourceCharMeasureRequest {
            source_item: self.source_item_request(),
            position,
        }
    }
}

impl BufferTextSourceSpecialDisplayKind {
    fn should_allocate_policy_face(self, params: &WindowParams) -> bool {
        match self {
            Self::Control => params.escape_glyph_fg != 0,
            Self::Nobreak => params.nobreak_char_fg != 0,
            Self::Glyphless => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharPreparedAppend {
    kind: BufferTextSourceSpecialDisplayKind,
    append_plan: BufferTextSpecialSourceCharAppendPlan,
    measured_width_px: Option<f32>,
}

impl BufferTextSpecialSourceCharPreparedAppend {
    #[cfg(test)]
    pub(crate) fn kind(&self) -> BufferTextSourceSpecialDisplayKind {
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
    ) -> Option<BufferTextSpecialSourceCharOverflowAction> {
        Some(BufferTextSpecialSourceCharOverflowAction::for_decision(
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
        let (progress, position) = context.append_special_source_char_plan_to_text_row_and_emit(
            geometry,
            state,
            self.append_plan,
        )?;
        Some(BufferTextSpecialSourceCharAppendOutcome {
            progress,
            position,
            append_policy,
        })
    }

    pub(crate) fn append_to_text_row_and_apply<B: LayoutBufferView + ?Sized>(
        self,
        context: &BufferTextRowAppendContext<'_, '_, B>,
        geometry: &DisplayRowGeometryState,
        params: &WindowParams,
        state: &mut BufferTextSpecialSourceCharRenderState<'_>,
    ) -> BufferTextSourceAppendContinuation {
        let Some(outcome) = self.append_to_text_row(
            context,
            geometry,
            params,
            state.face_ids,
            &mut state.source_render,
        ) else {
            return BufferTextSourceAppendContinuation::Stopped;
        };
        outcome.apply_rendered_special_char_to_walk_state(
            state.face_scan,
            state.word_wrap,
            state.x,
            state.col,
            state.charpos,
        );
        BufferTextSourceAppendContinuation::Rendered
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
    position: DisplayRowPosition,
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
        *x = self.position.x_px;
        *col = self.position.col;
    }

    pub(crate) fn apply_rendered_special_char_to_walk_state(
        &self,
        face_scan: &mut FaceScanCheckpoint,
        word_wrap: &mut WordWrapRenderState,
        x: &mut f32,
        col: &mut usize,
        charpos: &mut i64,
    ) {
        self.apply_to_text_row_state(face_scan, x, col);
        *charpos += 1;
        word_wrap.disallow_after_current_char();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharAppendPlan {
    source_item: BufferTextSourceItemRequest,
    position: DisplayRowPosition,
}

impl BufferTextSpecialSourceCharAppendPlan {
    fn position(&self) -> DisplayRowPosition {
        self.position
    }

    fn source_item(&self) -> BufferTextSourceItemRequest {
        self.source_item.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSpecialSourceCharMeasureRequest {
    source_item: BufferTextSourceItemRequest,
    position: DisplayRowPosition,
}

impl BufferTextSpecialSourceCharMeasureRequest {
    fn position(&self) -> DisplayRowPosition {
        self.position
    }

    fn source_item(&self) -> BufferTextSourceItemRequest {
        self.source_item.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourceCharAppendPlan {
    source_text: BufferTextSourceTextRequest,
    position: DisplayRowPosition,
}

impl BufferTextSourceCharAppendPlan {
    fn source_text(self) -> BufferTextSourceTextRequest {
        self.source_text
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn advance_px(self) -> f32 {
        self.source_text.advance_px()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BufferTextSourcePositionedAdvanceRequest<'text> {
    source: BufferTextSourceAdvanceRequest<'text>,
    position: DisplayRowPosition,
}

impl<'text> BufferTextSourcePositionedAdvanceRequest<'text> {
    fn new(source: BufferTextSourceAdvanceRequest<'text>, position: DisplayRowPosition) -> Self {
        Self { source, position }
    }

    fn text(self) -> &'text [u8] {
        self.source.text()
    }

    fn byte_idx(self) -> usize {
        self.source.byte_idx()
    }

    fn range(self) -> BufferTextSourceRange {
        self.source.range()
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn cluster(self) -> BufferTextSourceClusterState {
        self.source.cluster()
    }

    fn append_plan(
        self,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> BufferTextSourceCharAppendPlan {
        BufferTextSourceCharAppendPlan {
            source_text: self.source.into_text_request(resolved_advance),
            position: self.position,
        }
    }
}

impl BufferTextSourceAppendItem {
    fn append_kind(&self) -> DisplayRowAppendKind {
        match self {
            Self::ControlChar { .. } => DisplayRowAppendKind::ControlChar,
            Self::SourceMappedText { .. } => DisplayRowAppendKind::SourceMappedText,
            Self::Glyphless { .. } => DisplayRowAppendKind::Glyphless,
        }
    }
}

impl BufferTextSourceTextItemRequest {
    fn append_kind(self) -> DisplayRowAppendKind {
        if self.source_char() == '\t' {
            DisplayRowAppendKind::Tab
        } else {
            DisplayRowAppendKind::SourceText
        }
    }
}

impl BufferTextSourceItemRequest {
    fn append_kind(&self) -> DisplayRowAppendKind {
        self.item().append_kind()
    }
}

pub(crate) struct BufferTextItemAppendContext<'a, B: LayoutBufferView + ?Sized> {
    buffer: &'a B,
    buffer_id: BufferId,
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a, B: LayoutBufferView + ?Sized> BufferTextItemAppendContext<'a, B> {
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
            face_id,
            base_face,
            frame,
        }
    }

    fn append_source_request_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let append_item = buffer_text_source_item_append_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.face_id,
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        DisplayRowSourceAppendOperation::for_single_item(
            &item,
            self.base_face,
            self.face_id,
            self.frame.clone(),
            position,
            kind,
        )
        .render_single_item_to_text_row_and_emit(state, item, &mut render_policy)
    }

    fn measure_source_request_width_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<f32> {
        let append_item = buffer_text_source_item_append_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.face_id,
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        Some(
            DisplayRowSourceAppendOperation::for_single_item(
                &item,
                self.base_face,
                self.face_id,
                self.frame.clone(),
                position,
                kind,
            )
            .measure_single_item_to_text_row(state, item, &mut render_policy)?
            .metrics
            .width_px,
        )
    }

    fn measure_source_request_width_or_item_fallback_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> f32 {
        let fallback_width = source_item.fallback_width_px(self.frame.geometry.char_width);
        self.measure_source_request_width_to_text_row(state, source_item, position)
            .unwrap_or(fallback_width)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendPlacement {
    row: usize,
    y: f32,
    glyph_y: f32,
}

impl DisplayRowAppendPlacement {
    fn new(row: usize, y: f32, glyph_y: f32) -> Self {
        Self { row, y, glyph_y }
    }

    pub(crate) fn from_geometry_state(
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
    ) -> Self {
        Self::new(
            geometry.row(),
            geometry.y(),
            geometry.glyph_y(glyph_y_offset),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendArea {
    content_x: f32,
    width: f32,
    text_width: f32,
    line_number_width: f32,
}

impl DisplayRowAppendArea {
    pub(crate) fn new(content_x: f32, width: f32, text_width: f32, line_number_width: f32) -> Self {
        Self {
            content_x,
            width,
            text_width,
            line_number_width,
        }
    }

    pub(crate) fn content_x(self) -> f32 {
        self.content_x
    }

    pub(crate) fn right_edge(self) -> f32 {
        self.content_x + self.width
    }

    fn full_text_width(self) -> Self {
        Self {
            width: (self.text_width - self.line_number_width).max(0.0),
            ..self
        }
    }
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

    pub(crate) fn content_x(&self) -> f32 {
        self.area.content_x()
    }

    pub(crate) fn right_edge(&self) -> f32 {
        self.area.right_edge()
    }

    pub(crate) fn full_text_right_edge(&self) -> f32 {
        self.area.full_text_width().right_edge()
    }

    pub(crate) fn full_text_width_surface(&self) -> Self {
        Self {
            area: self.area.full_text_width(),
            tab_policy: self.tab_policy.clone(),
        }
    }

    pub(crate) fn frame(
        &self,
        placement: DisplayRowAppendPlacement,
        metrics: DisplayRowAppendMetrics,
    ) -> DisplayRowAppendFrame {
        DisplayRowAppendFrame::from_parts(placement, self.area, metrics, self.tab_policy.clone())
    }

    fn frame_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        metrics: DisplayRowAppendMetrics,
    ) -> DisplayRowAppendFrame {
        self.frame(
            DisplayRowAppendPlacement::from_geometry_state(geometry, glyph_y_offset),
            metrics,
        )
    }

    fn text_row_frame_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> DisplayRowAppendFrame {
        self.frame_from_geometry_state(
            geometry,
            glyph_y_offset,
            DisplayRowAppendMetrics::text_row(height, ascent, char_width, default_row_height),
        )
    }

    fn frame_for_active_face_from_geometry_state(
        &self,
        geometry: &DisplayRowGeometryState,
        glyph_y_offset: f32,
        active_face: &DisplayRowActiveFaceState,
        default_row_height: f32,
    ) -> DisplayRowAppendFrame {
        self.frame_from_geometry_state(
            geometry,
            glyph_y_offset,
            DisplayRowAppendMetrics::from_active_face_state(active_face, default_row_height),
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowTextAppendContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    geometry: &'a DisplayRowGeometryState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

impl<'a> DisplayRowTextAppendContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &'a DisplayRowGeometryState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            append_surface,
            geometry,
            glyph_y_offset,
            default_row_height,
        }
    }

    pub(crate) fn text_row_frame(
        self,
        height: f32,
        ascent: f32,
        char_width: f32,
    ) -> DisplayRowAppendFrame {
        self.append_surface.text_row_frame_from_geometry_state(
            self.geometry,
            self.glyph_y_offset,
            height,
            ascent,
            char_width,
            self.default_row_height,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowActiveFaceAppendContext<'row, 'face> {
    text_context: DisplayRowTextAppendContext<'row>,
    active_face: &'face DisplayRowActiveFaceState,
}

impl<'row, 'face> DisplayRowActiveFaceAppendContext<'row, 'face> {
    pub(crate) fn new(
        append_surface: &'row DisplayRowAppendSurface,
        geometry: &'row DisplayRowGeometryState,
        active_face: &'face DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            text_context: DisplayRowTextAppendContext::new(
                append_surface,
                geometry,
                glyph_y_offset,
                default_row_height,
            ),
            active_face,
        }
    }

    pub(crate) fn active_face_frame(self) -> DisplayRowAppendFrame {
        self.text_context
            .append_surface
            .frame_for_active_face_from_geometry_state(
                self.text_context.geometry,
                self.text_context.glyph_y_offset,
                self.active_face,
                self.text_context.default_row_height,
            )
    }

    pub(crate) fn active_face(self) -> &'face DisplayRowActiveFaceState {
        self.active_face
    }

    #[cfg(test)]
    pub(crate) fn full_text_width_active_face_frame(self) -> DisplayRowAppendFrame {
        self.text_context
            .append_surface
            .full_text_width_surface()
            .frame_for_active_face_from_geometry_state(
                self.text_context.geometry,
                self.text_context.glyph_y_offset,
                self.active_face,
                self.text_context.default_row_height,
            )
    }

    pub(crate) fn text_row_frame(
        self,
        height: f32,
        ascent: f32,
        char_width: f32,
    ) -> DisplayRowAppendFrame {
        self.text_context.text_row_frame(height, ascent, char_width)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAppendMetrics {
    pub(crate) height: f32,
    pub(crate) ascent: f32,
    pub(crate) char_width: f32,
    space_width: f32,
    pub(crate) default_row_height: f32,
}

impl DisplayRowAppendMetrics {
    fn new(
        height: f32,
        ascent: f32,
        char_width: f32,
        space_width: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            height,
            ascent,
            char_width,
            space_width,
            default_row_height,
        }
    }

    pub(crate) fn text_row(
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> Self {
        Self::new(height, ascent, char_width, char_width, default_row_height)
    }

    pub(crate) fn from_active_face_state(
        active_face: &DisplayRowActiveFaceState,
        default_row_height: f32,
    ) -> Self {
        Self::from_measured_face_metrics(active_face.metrics(), default_row_height)
    }

    pub(crate) fn display_box_from_active_face_state(
        active_face: &DisplayRowActiveFaceState,
        height: f32,
        ascent: f32,
        default_row_height: f32,
    ) -> Self {
        let metrics = active_face.metrics();
        Self::new(
            height,
            ascent,
            metrics.char_width,
            metrics.space_width,
            default_row_height,
        )
    }

    pub(crate) fn from_measured_face_metrics(
        metrics: DisplayRowMeasuredFaceMetrics,
        default_row_height: f32,
    ) -> Self {
        Self::new(
            metrics.row_height,
            metrics.ascent,
            metrics.char_width,
            metrics.space_width,
            default_row_height,
        )
    }
}

#[derive(Clone)]
pub(crate) struct DisplayRowAppendFrame {
    row: usize,
    glyph_y: f32,
    geometry: DisplayRowGeometry,
    default_row_height: f32,
    content_x: f32,
    text_width: f32,
    line_number_width: f32,
    face_space_width: f32,
}

impl DisplayRowAppendFrame {
    fn right_edge(&self) -> f32 {
        self.content_x + self.geometry.width
    }

    fn text_right_edge_excluding_line_number(&self) -> f32 {
        self.content_x + (self.text_width - self.line_number_width).max(0.0)
    }

    fn from_parts(
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

    fn source_append_request<'face>(
        &self,
        position: DisplayRowPosition,
        face_id: u32,
        base_face: &'face ResolvedFace,
        kind: DisplayRowAppendKind,
    ) -> DisplayRowSourceAppendRequest<'face> {
        let geometry = DisplayRowGeometry {
            char_width: kind.char_width(self),
            ..self.geometry.clone()
        };
        DisplayRowSourceAppendRequest::from_text_row_policy(
            position,
            face_id,
            base_face,
            DisplayRowSourceAppendRequestPolicy::new(
                self.row,
                self.geometry.y,
                self.glyph_y,
                kind.output_height(self),
                geometry,
                kind.max_x(self),
            ),
        )
    }
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
    fn char_width(self, frame: &DisplayRowAppendFrame) -> f32 {
        match self {
            Self::Tab | Self::DisplayReplacementString => frame.face_space_width,
            Self::SourceText
            | Self::ControlChar
            | Self::SourceMappedText
            | Self::Glyphless
            | Self::DisplayReplacement => frame.geometry.char_width,
        }
    }

    fn max_x(self, frame: &DisplayRowAppendFrame) -> DisplayRowMaxX {
        match self {
            Self::Tab => DisplayRowMaxX::Unbounded,
            Self::ControlChar => {
                DisplayRowMaxX::Bounded(frame.text_right_edge_excluding_line_number())
            }
            Self::SourceText
            | Self::SourceMappedText
            | Self::Glyphless
            | Self::DisplayReplacement
            | Self::DisplayReplacementString => DisplayRowMaxX::Bounded(frame.right_edge()),
        }
    }

    fn output_height(self, frame: &DisplayRowAppendFrame) -> f32 {
        match self {
            Self::SourceText
            | Self::Glyphless
            | Self::DisplayReplacement
            | Self::DisplayReplacementString => frame.geometry.height,
            Self::Tab | Self::ControlChar | Self::SourceMappedText => frame.default_row_height,
        }
    }
}

pub(crate) fn append_synthetic_text_to_display_row(
    state: &mut TextRowSourceRenderState<'_>,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face_id: u32,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let source = source.into_item_source(face_id);
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    let start = position;
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let outcome = DisplayRowSourceAppendOperation::new(
        base_face,
        face_id,
        frame,
        position,
        DisplayRowAppendKind::SourceText,
    )
    .render_source_to_text_row_and_emit(state, source, &mut face_ids, &mut render_policy)?;
    Some(outcome.into_append_progress_and_position(start))
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
