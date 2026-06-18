use crate::display_buffer_text_render::{
    BufferTextSourceAppendContinuation, BufferTextSourceCharOverflowAction,
    BufferTextSourceCharRenderState, BufferTextSpecialSourceCharOverflowAction,
    BufferTextSpecialSourceCharRenderState,
};
use crate::display_buffer_text_source::BufferTextSourceStepChar;
use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    capture_cursor_info, update_cursor_info_for_main_char,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy,
    DisplaySourceAppendRenderPolicy,
};
use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendFrame, DisplayRowAppendKind,
    DisplayRowAppendSurface,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowTextPosition};
use crate::display_row_source_append::DisplayRowSingleItemAppendContext;
use crate::display_row_source_render::{TextRowSourceMeasureState, TextRowSourceRenderState};
use crate::display_row_walk_state::{
    BufferTextRowOverflowDecision, FaceScanCheckpoint, SpecialTextRowOverflowDecision,
    TrailingWhitespaceRenderState, WordWrapRenderState,
};
use crate::display_source::{
    BufferTextSourceAdvancePath, BufferTextSourceAdvanceRequest, BufferTextSourceAppendItem,
    BufferTextSourceChar, BufferTextSourceClusterState, BufferTextSourceItemRequest,
    BufferTextSourceNaturalAdvanceRequest, BufferTextSourceNaturalFallbackAdvance,
    BufferTextSourceRange, BufferTextSourceSpecialDisplayKind, BufferTextSourceTextItemRequest,
    BufferTextSourceTextRequest, BufferTextSpecialSourceCharRequest,
    ResolvedBufferTextSourceAdvance,
};
use crate::display_text_run_measurement::ComplexTextRunAdvanceResolver;
use crate::font_metrics::FontMetricsService;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace};
use crate::types::{LineWrapMode, WindowParams};
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

    #[cfg(test)]
    pub(crate) fn append_request<B: LayoutBufferView + ?Sized>(
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
    pub(crate) fn resolve_to_text_row(
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
    fn measure_to_text_row(
        self,
        state: &mut TextRowSourceMeasureState<'_>,
        base_face: &ResolvedFace,
        face_id: u32,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
    ) -> Option<f32> {
        let append_context = DisplayRowSingleItemAppendContext::new(base_face, face_id, frame);
        let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
        append_context.measure_item_width_with_policy(
            state,
            source_item,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceRangeItemAppendRequest {
    item: DisplayItem,
    append_kind: DisplayRowAppendKind,
}

impl BufferTextSourceRangeItemAppendRequest {
    fn new(item: DisplayItem, append_kind: DisplayRowAppendKind) -> Self {
        Self { item, append_kind }
    }

    pub(crate) fn append_kind(&self) -> DisplayRowAppendKind {
        self.append_kind
    }

    pub(crate) fn into_item(self) -> DisplayItem {
        self.item
    }
}

#[cfg(test)]
pub(crate) fn buffer_text_source_text_item_append_request<B: LayoutBufferView + ?Sized>(
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
        source_item: Option<&DisplayItem>,
    ) -> BufferTextSpecialSourceCharPreparedAppend {
        let measured_width_px = request.requires_overflow_measurement().then(|| {
            if let Some(source_item) = source_item {
                self.item_active_face(geometry)
                    .measure_source_display_item_width_or_item_fallback_to_text_row(
                        state,
                        source_item,
                        request.source_item_request(),
                        position,
                    )
            } else {
                self.measure_special_source_char_request_width_or_item_fallback_to_text_row(
                    geometry,
                    state,
                    request.measure_at(position),
                )
            }
        });
        request.prepared_append_at(position, measured_width_px, source_item.cloned())
    }

    fn append_item_source_request_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        self.item_active_face(geometry)
            .append_source_request_to_text_row_and_emit(state, source_item, position)
    }

    fn append_special_source_char_plan_to_text_row_and_emit(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        plan: BufferTextSpecialSourceCharAppendPlan,
    ) -> Option<DisplayRowAppendProgress> {
        let position = plan.position();
        let fallback_kind = plan.source_item().append_kind();
        if let Some(item) = plan.display_item {
            return self
                .item_active_face(geometry)
                .append_display_item_to_text_row_and_emit(state, item, position, fallback_kind);
        }
        self.append_item_source_request_to_text_row_and_emit(
            geometry,
            state,
            plan.source_item(),
            position,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_source_advance_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        request: BufferTextSourcePositionedAdvanceRequest<'_, '_>,
    ) -> ResolvedBufferTextSourceAdvance {
        let frame = self.active_face_context(geometry).active_face_frame();
        append_state
            .advance_resolver()
            .resolve_source_advance_request_to_text_row(
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
        request: BufferTextSourcePositionedAdvanceRequest<'_, '_>,
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
    fn prepare_text_source_item_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        source_char: &BufferTextSourceChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
        cluster_tail: Option<(char, bool)>,
    ) -> BufferTextSourceCharPreparedAppend {
        let request = source_char.advance_request_for_item_at(
            text,
            byte_idx,
            position,
            source_item,
            cluster_tail,
        );
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
    pub(crate) fn prepare_source_item_char_at(
        &self,
        geometry: &DisplayRowGeometryState,
        append_state: &mut BufferTextRowAppendState,
        measure_state: &mut TextRowSourceMeasureState<'_>,
        source_char: &BufferTextSourceChar,
        text: &[u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        source_item: &DisplayItem,
        cluster_tail: Option<(char, bool)>,
    ) -> BufferTextPreparedSourceCharAppend {
        if let Some(request) = source_char.special_request(cluster_tail) {
            let source_item = matching_special_display_item(source_item, request.kind());
            return BufferTextPreparedSourceCharAppend::Special(
                self.prepare_special_source_char_at(
                    geometry,
                    measure_state,
                    request,
                    position,
                    source_item,
                ),
            );
        }
        BufferTextPreparedSourceCharAppend::Text(self.prepare_text_source_item_char_at(
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

    pub(crate) fn prepare_source_item_for_current_text_row(
        &self,
        request: BufferTextSourceDisplayItemPreparationRequest<'_>,
        state: &mut BufferTextSourceCharPreparationState<'_>,
    ) -> BufferTextPreparedSourceCharAppend {
        let cluster_tail = current_text_window_cluster_tail(state.measure.builder);
        self.prepare_source_item_char_at(
            &request.geometry,
            state.append_state,
            &mut state.measure,
            request.source_char,
            request.text,
            request.byte_idx,
            request.position,
            request.source_item,
            cluster_tail,
        )
    }

    #[cfg(test)]
    pub(crate) fn append_source_text_request_to_text_row(
        &self,
        geometry: &DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        source_text: BufferTextSourceTextRequest,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        let frame = self.active_face_context(geometry).active_face_frame();
        let face_id = self.active_face.face_id();
        let append_item = source_text.append_request(self.buffer_id, self.buffer, face_id)?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        let mut render_policy = source_text.append_render_policy();
        DisplayRowSingleItemAppendContext::new(self.active_face.resolved_face(), face_id, frame)
            .render_item_with_policy(state, item, position, kind, &mut render_policy)
    }

    fn append_source_display_item_to_text_row(
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
        DisplayRowSingleItemAppendContext::new(self.active_face.resolved_face(), face_id, frame)
            .render_item_with_policy(state, item, position, fallback_kind, render_policy)
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
    kind: BufferTextSourceSpecialDisplayKind,
) -> Option<&DisplayItem> {
    match (&source_item.kind, kind) {
        (DisplayItemKind::ControlChar { .. }, BufferTextSourceSpecialDisplayKind::Control)
        | (DisplayItemKind::Glyphless(_), BufferTextSourceSpecialDisplayKind::Glyphless)
        | (DisplayItemKind::SourceMappedText(_), BufferTextSourceSpecialDisplayKind::Nobreak) => {
            Some(source_item)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSourceDisplayItemPreparationRequest<'a> {
    geometry: DisplayRowGeometryState,
    source_char: &'a BufferTextSourceChar,
    text: &'a [u8],
    byte_idx: usize,
    position: DisplayRowPosition,
    source_item: &'a DisplayItem,
}

impl<'a> BufferTextSourceDisplayItemPreparationRequest<'a> {
    pub(crate) fn new(
        geometry: DisplayRowGeometryState,
        source_char: &'a BufferTextSourceChar,
        text: &'a [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        source_item: &'a DisplayItem,
    ) -> Self {
        Self {
            geometry,
            source_char,
            text,
            byte_idx,
            position,
            source_item,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharPreparedAppend {
    pub(crate) plan: BufferTextSourceCharAppendPlan,
}

impl BufferTextSourceCharPreparedAppend {
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
        &self,
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
    ) -> Option<BufferTextSourceCharAppendOutcome> {
        let progress = context.append_source_char_plan_to_text_row(geometry, state, self.plan)?;
        Some(BufferTextSourceCharAppendOutcome { progress })
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
    fn resolve_source_advance_request_to_text_row(
        &mut self,
        state: &mut TextRowSourceMeasureState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        frame: DisplayRowAppendFrame,
        request: BufferTextSourcePositionedAdvanceRequest<'_, '_>,
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
                    active_face_state,
                    frame,
                    request.position(),
                    request.source_item(),
                );
                ResolvedBufferTextSourceAdvance::natural(advance_px)
            }
        }
    }
}

pub(crate) fn buffer_text_source_item_append_request<B: LayoutBufferView + ?Sized>(
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

impl BufferTextSourceStepChar {
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
    fn advance_request_for_item_at<'text, 'item>(
        &self,
        text: &'text [u8],
        byte_idx: usize,
        position: DisplayRowPosition,
        source_item: &'item DisplayItem,
        tail: Option<(char, bool)>,
    ) -> BufferTextSourcePositionedAdvanceRequest<'text, 'item> {
        BufferTextSourcePositionedAdvanceRequest::new(
            self.advance_request(text, byte_idx, tail),
            position,
            source_item,
        )
    }
}

impl BufferTextSpecialSourceCharRequest {
    pub(crate) fn append_plan_at(
        &self,
        position: DisplayRowPosition,
    ) -> BufferTextSpecialSourceCharAppendPlan {
        BufferTextSpecialSourceCharAppendPlan {
            source_item: self.source_item_request(),
            position,
            display_item: None,
        }
    }

    fn prepared_append_at(
        self,
        position: DisplayRowPosition,
        measured_width_px: Option<f32>,
        display_item: Option<DisplayItem>,
    ) -> BufferTextSpecialSourceCharPreparedAppend {
        BufferTextSpecialSourceCharPreparedAppend {
            kind: self.kind(),
            append_plan: self
                .append_plan_at(position)
                .with_display_item(display_item),
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
    pub(crate) kind: BufferTextSourceSpecialDisplayKind,
    pub(crate) append_plan: BufferTextSpecialSourceCharAppendPlan,
    pub(crate) measured_width_px: Option<f32>,
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
    pub(crate) source_item: BufferTextSourceItemRequest,
    pub(crate) position: DisplayRowPosition,
    pub(crate) display_item: Option<DisplayItem>,
}

impl BufferTextSpecialSourceCharAppendPlan {
    fn with_display_item(mut self, display_item: Option<DisplayItem>) -> Self {
        self.display_item = display_item;
        self
    }

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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharAppendPlan {
    pub(crate) source_text: BufferTextSourceTextRequest,
    pub(crate) position: DisplayRowPosition,
    pub(crate) source_item: DisplayItem,
}

impl BufferTextSourceCharAppendPlan {
    fn source_text(&self) -> BufferTextSourceTextRequest {
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
pub(crate) struct BufferTextSourcePositionedAdvanceRequest<'text, 'item> {
    source: BufferTextSourceAdvanceRequest<'text>,
    position: DisplayRowPosition,
    source_item: &'item DisplayItem,
}

impl<'text, 'item> BufferTextSourcePositionedAdvanceRequest<'text, 'item> {
    pub(crate) fn new(
        source: BufferTextSourceAdvanceRequest<'text>,
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

    fn range(self) -> BufferTextSourceRange {
        self.source.range()
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn cluster(self) -> BufferTextSourceClusterState {
        self.source.cluster()
    }

    fn source_item(self) -> &'item DisplayItem {
        self.source_item
    }

    fn append_plan(
        self,
        resolved_advance: ResolvedBufferTextSourceAdvance,
    ) -> BufferTextSourceCharAppendPlan {
        BufferTextSourceCharAppendPlan {
            source_text: self.source.into_text_request(resolved_advance),
            position: self.position,
            source_item: self.source_item.clone(),
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

    fn single_item_context(&self) -> DisplayRowSingleItemAppendContext<'a> {
        DisplayRowSingleItemAppendContext::new(self.base_face, self.face_id, self.frame.clone())
    }

    pub(crate) fn append_source_request_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        let append_item = buffer_text_source_item_append_request(
            source_item,
            self.buffer_id,
            self.buffer,
            self.face_id,
        )?;
        let kind = append_item.append_kind();
        let item = append_item.into_item();
        self.single_item_context()
            .render_item_naturally(state, item, position, kind)
    }

    pub(crate) fn append_display_item_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
    ) -> Option<DisplayRowAppendProgress> {
        self.single_item_context()
            .render_item_naturally(state, item, position, fallback_kind)
    }

    pub(crate) fn measure_source_request_width_to_text_row(
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
        self.single_item_context()
            .measure_item_width_naturally(state, &item, position, kind)
    }

    pub(crate) fn measure_display_item_width_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: &DisplayItem,
        position: DisplayRowPosition,
        fallback_kind: DisplayRowAppendKind,
    ) -> Option<f32> {
        self.single_item_context().measure_item_width_naturally(
            state,
            item,
            position,
            fallback_kind,
        )
    }

    pub(crate) fn measure_source_display_item_width_or_item_fallback_to_text_row(
        &self,
        state: &mut TextRowSourceMeasureState<'_>,
        item: &DisplayItem,
        source_item: BufferTextSourceItemRequest,
        position: DisplayRowPosition,
    ) -> f32 {
        let fallback_width = source_item.fallback_width_px(self.frame.geometry.char_width);
        self.measure_display_item_width_to_text_row(
            state,
            item,
            position,
            source_item.append_kind(),
        )
        .unwrap_or(fallback_width)
    }

    pub(crate) fn measure_source_request_width_or_item_fallback_to_text_row(
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
