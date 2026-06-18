use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth,
    CapturedCursorVisualState, CursorCaptureState, capture_cursor_info,
    display_property_replacement_cursor_info, update_cursor_info_for_main_char,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};

use crate::display_buffer_text_source::{
    BufferTextDecodedSourceChar, BufferTextLineBreakSourceEvent, BufferTextSourceTextEvent,
};
#[cfg(test)]
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{DisplayItem, DisplayItemKind, RenderFaceRef};
use crate::display_origin::{DisplayOrigin, OverlayStringKind};
use crate::display_property::{
    DisplayMediaReplacementProperty, DisplayPropertyClassification, DisplayReplacementProperty,
    classify_display_property,
};
#[cfg(test)]
use crate::display_row::DisplayRowRenderStop;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_text_row_and_emit;
use crate::display_row::{
    CurrentTextRowRenderOutcome, DisplayRowActiveFaceState, DisplayRowComplexTextRunAdvancePolicy,
    DisplayRowFallbackMetrics, DisplayRowGeometry, DisplayRowMeasuredFaceMetrics,
    DisplayRowMeasurementPolicy, DisplayRowRenderBounds, DisplayRowRenderClipBehavior,
    DisplayRowRenderPolicy, DisplayRowSourceAppendRequest, DisplayRowSourceAppendRequestPolicy,
    DisplayRowSourceState, DisplaySourceAppendRenderPolicy, NaturalDisplayRowAppendRenderPolicy,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowItemMeasurement,
    DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::{
    DisplayRowBoundaryTarget, DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults,
    DisplayRowGeometryState, DisplayRowHitRange, DisplayRowLimit, DisplayRowMaxX,
    DisplayRowScopedValue, DisplayRowTextPosition, DisplayRowVisibilityLimit, DisplayRowYPositions,
    DisplayRowYRecording,
};
use crate::display_row_overlay_string::render_overlay_string;
use crate::display_row_source_render::{
    TextRowOutputRenderState, TextRowSourceMeasureState, TextRowSourceRenderState,
    current_text_measure_state, current_text_render_state,
};
use crate::display_row_walk_state::{
    ActiveDisplayPropertySpan, BoxFaceRowState, BufferTextRowOverflowDecision, FaceScanCheckpoint,
    HitRowRangeTracker, HorizontalScrollSkipState, LineNumberRenderState,
    SpecialTextRowOverflowDecision, TextPropertyScanCheckpoints, TextRowTransitionPrefixAction,
    TextRowTransitionStatePolicy, TrailingWhitespaceRenderState, WordWrapBreakCandidate,
    WordWrapRenderState, skip_text_to_charpos, skip_to_newline,
};
use crate::display_source::{
    BufferDisplayPropertyTextModifierAction, BufferDisplayPropertyTextSourceEvent,
    BufferDisplayReplacementSource, BufferDisplayReplacementStringRequest,
    BufferTextSourceAdvancePath, BufferTextSourceAdvanceRequest, BufferTextSourceAppendItem,
    BufferTextSourceChar, BufferTextSourceClusterState, BufferTextSourceItemRequest,
    BufferTextSourceNaturalAdvanceRequest, BufferTextSourceNaturalFallbackAdvance,
    BufferTextSourceRange, BufferTextSourceSpecialDisplayKind, BufferTextSourceTextItemRequest,
    BufferTextSourceTextRequest, BufferTextSpecialSourceCharRequest, DisplayItemSource,
    DisplayPropertyReplacementCursorPolicy, DisplayPropertyReplacementSourceInputs,
    DisplayPropertyReplacementSourceItem, DisplayPropertyReplacementSourceMetrics,
    DisplayReplacementAppendItem, DisplayReplacementMediaSourceItem,
    DisplayReplacementMediaSourceResolution, DisplayReplacementSourceMappedTextItem,
    DisplayReplacementStretchSourceItem, DisplayReplacementStringSourceItem,
    LispStringSourceCursor, ResolvedBufferTextSourceAdvance, SyntheticTextItemSource,
};
#[cfg(test)]
use crate::display_source_resolver::PendingDisplaySourceFace;
use crate::display_source_resolver::{
    DisplayStringBaseFace, ResolvedDisplayReplacement, resolve_display_replacement,
};
use crate::display_text_run_measurement::ComplexTextRunAdvanceResolver;
use crate::font_metrics::FontMetricsService;
use crate::hit_test::HitRow;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::neovm_bridge::{
    FaceResolver, LayoutBufferView, OverlayDisplayString, ResolvedFace, RustTextPropAccess,
};
use crate::types::LineWrapMode;
use crate::types::WindowParams;
use crate::unicode::decode_utf8;
use crate::window_output::{
    TextMatrixRowGeometryTransition, TextMatrixRowTransition, WindowOutputEmitter,
    current_text_window_cluster_tail, emit_text_matrix_row_transition_with_limit,
};
use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::{Context, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LispStringSourceId(u64);

impl LispStringSourceId {
    pub(crate) const OVERLAY_STRING: Self = Self(1);
    const PREFIX: Self = Self(2);

    #[cfg(test)]
    fn display_replacement(source_id: u64) -> Self {
        Self(source_id)
    }

    pub(crate) fn raw(self) -> u64 {
        self.0
    }
}

const SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS: u64 = 3;
const SYNTHETIC_SOURCE_HSCROLL_TRUNCATION: u64 = 4;
const SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS: u64 = 5;

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

pub(crate) struct DisplayRowBoundaryTransitionRequest<'a> {
    target: DisplayRowBoundaryTarget<'a>,
    max_rows: usize,
}

pub(crate) struct DisplayRowLineBreakTransitionRequest<'a> {
    hit_range: DisplayRowHitRange,
    defaults: DisplayRowGeometryDefaults,
    row_base: usize,
    col: usize,
    x: f32,
    line_spacing: f32,
    row_y_recording: DisplayRowYRecording<'a>,
    max_rows: usize,
}

pub(crate) struct DisplayRowTransitionRequestContext<'a> {
    defaults: DisplayRowGeometryDefaults,
    row_base: usize,
    row_y_recording: DisplayRowYRecording<'a>,
    max_rows: usize,
}

pub(crate) struct DisplayRowTextWindowTransitionContext<'a> {
    request_context: DisplayRowTransitionRequestContext<'a>,
}

pub(crate) struct DisplayRowTextWindowEmitContext<'a, 'emit> {
    defaults: DisplayRowGeometryDefaults,
    row_base: usize,
    row_y_positions: &'a mut DisplayRowYPositions,
    max_rows: usize,
    row_geometry: &'emit mut DisplayRowGeometryState,
    row_flags: &'emit mut DisplayRowFlags,
    row_limit: DisplayRowLimit,
    hit_rows: &'emit mut Vec<HitRow>,
    output_render: TextRowOutputRenderState<'emit>,
}

pub(crate) struct BufferTextLineBreakRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextOverflowRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextSpecialOverflowRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferTextSourceCharRenderRequest<'a> {
    source_event: BufferTextSourceTextEvent,
    context: BufferTextSourceCharRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSourceCharRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) buffer_id: BufferId,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) params: &'a WindowParams,
    pub(crate) glyph_y_offset: f32,
    pub(crate) char_h: f32,
    pub(crate) point_charpos: i64,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferTextSourceCharRenderRequestState<'a, 'emit> {
    pub(crate) append_state: &'emit mut BufferTextRowAppendState,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
    pub(crate) raise_span: &'emit mut ActiveDisplayPropertySpan<f32>,
}

pub(crate) struct BufferSelectiveDisplayTailRenderRequest<'a> {
    source_char: BufferTextDecodedSourceChar,
    context: BufferSelectiveDisplayTailRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferSelectiveDisplayTailRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) selective_display: i32,
    pub(crate) tab_width: i32,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) glyph_y_offset: f32,
    pub(crate) default_face_ascent: f32,
    pub(crate) char_h: f32,
    pub(crate) char_w: f32,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

pub(crate) struct BufferSelectiveDisplayTailRenderState<'a, 'emit> {
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) col: &'emit mut usize,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) x: &'emit mut f32,
    pub(crate) line_numbers: &'emit mut LineNumberRenderState,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) row_flags: &'emit mut DisplayRowFlags,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) prefix_request: &'emit mut DisplayRowPrefixRequest,
    pub(crate) hscroll_skip: &'emit mut HorizontalScrollSkipState,
    pub(crate) word_wrap: &'emit mut WordWrapRenderState,
    pub(crate) trailing_whitespace: &'emit mut TrailingWhitespaceRenderState,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
}

pub(crate) struct BufferInvisibleTextRenderRequest<'a> {
    context: BufferInvisibleTextRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferInvisibleTextRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) accessible_end: i64,
    pub(crate) point_charpos: i64,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) glyph_y_offset: f32,
    pub(crate) default_face_ascent: f32,
    pub(crate) char_h: f32,
    pub(crate) char_w: f32,
}

pub(crate) struct BufferInvisibleTextRenderRequestState<'a, 'emit> {
    pub(crate) checkpoints: &'emit mut TextPropertyScanCheckpoints,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) hit_rows: &'emit mut Vec<HitRow>,
    pub(crate) hit_row_range: &'emit mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
}

pub(crate) struct DisplayRowTransitionRenderState<'a> {
    prefix_request: &'a mut DisplayRowPrefixRequest,
    has_prefix: bool,
    line_numbers: &'a mut LineNumberRenderState,
    hscroll_skip: &'a mut HorizontalScrollSkipState,
    word_wrap: &'a mut WordWrapRenderState,
    trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRowTransitionContinuation {
    Continue,
    Exhausted,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowLineBreakTransitionPlan {
    state_policy: TextRowTransitionStatePolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRowOverflowTransitionKind {
    Truncation,
    VisualWrap,
}

pub(crate) struct DisplayRowOverflowTransitionRequest<'a> {
    kind: DisplayRowOverflowTransitionKind,
    hit_range: DisplayRowHitRange,
    defaults: DisplayRowGeometryDefaults,
    row_base: usize,
    col: usize,
    x: f32,
    row_y_recording: DisplayRowYRecording<'a>,
    max_rows: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowOverflowTransitionPlan {
    kind: DisplayRowOverflowTransitionKind,
    state_policy: TextRowTransitionStatePolicy,
}

impl<'a> DisplayRowBoundaryTransitionRequest<'a> {
    pub(crate) fn new(target: DisplayRowBoundaryTarget<'a>, max_rows: usize) -> Self {
        Self { target, max_rows }
    }

    pub(crate) fn emit(
        self,
        row_geometry: &mut DisplayRowGeometryState,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        let geometry_transition =
            row_geometry.finish_boundary_and_record_hit(self.target, hit_rows);
        emit_text_matrix_row_transition_with_limit(
            builder,
            output_emitter,
            evaluator,
            geometry_transition,
            self.max_rows,
        )
    }
}

impl<'a> DisplayRowTransitionRequestContext<'a> {
    pub(crate) fn new(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> Self {
        Self {
            defaults,
            row_base,
            row_y_recording,
            max_rows,
        }
    }

    pub(crate) fn line_break(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
    ) -> DisplayRowLineBreakTransitionRequest<'a> {
        plan.request(
            hit_range,
            self.defaults,
            self.row_base,
            position,
            line_spacing,
            self.row_y_recording,
            self.max_rows,
        )
    }

    pub(crate) fn overflow(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
    ) -> DisplayRowOverflowTransitionRequest<'a> {
        plan.request(
            hit_range,
            self.defaults,
            self.row_base,
            position,
            self.row_y_recording,
            self.max_rows,
        )
    }
}

impl<'a> DisplayRowTextWindowTransitionContext<'a> {
    pub(crate) fn new(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        row_y_positions: &'a mut DisplayRowYPositions,
        max_rows: usize,
    ) -> Self {
        Self {
            request_context: DisplayRowTransitionRequestContext::new(
                defaults,
                row_base,
                row_y_positions.recording(),
                max_rows,
            ),
        }
    }

    pub(crate) fn line_break(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
    ) -> DisplayRowLineBreakTransitionRequest<'a> {
        self.request_context
            .line_break(plan, hit_range, position, line_spacing)
    }

    pub(crate) fn emit_line_break(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
        row_geometry: &mut DisplayRowGeometryState,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        self.line_break(plan, hit_range, position, line_spacing)
            .emit(row_geometry, hit_rows, builder, output_emitter, evaluator)
    }

    pub(crate) fn overflow(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
    ) -> DisplayRowOverflowTransitionRequest<'a> {
        self.request_context.overflow(plan, hit_range, position)
    }

    pub(crate) fn emit_overflow(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        row_geometry: &mut DisplayRowGeometryState,
        row_flags: &mut DisplayRowFlags,
        row_limit: DisplayRowLimit,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        self.overflow(plan, hit_range, position).emit(
            row_geometry,
            row_flags,
            row_limit,
            hit_rows,
            builder,
            output_emitter,
            evaluator,
        )
    }
}

impl<'a, 'emit> DisplayRowTextWindowEmitContext<'a, 'emit> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        row_y_positions: &'a mut DisplayRowYPositions,
        max_rows: usize,
        row_geometry: &'emit mut DisplayRowGeometryState,
        row_flags: &'emit mut DisplayRowFlags,
        row_limit: DisplayRowLimit,
        hit_rows: &'emit mut Vec<HitRow>,
        output_render: TextRowOutputRenderState<'emit>,
    ) -> Self {
        Self {
            defaults,
            row_base,
            row_y_positions,
            max_rows,
            row_geometry,
            row_flags,
            row_limit,
            hit_rows,
            output_render,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_source_render(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        row_y_positions: &'a mut DisplayRowYPositions,
        max_rows: usize,
        row_geometry: &'emit mut DisplayRowGeometryState,
        row_flags: &'emit mut DisplayRowFlags,
        row_limit: DisplayRowLimit,
        hit_rows: &'emit mut Vec<HitRow>,
        source_render: &'emit mut TextRowSourceRenderState<'emit>,
    ) -> Self {
        Self::new(
            defaults,
            row_base,
            row_y_positions,
            max_rows,
            row_geometry,
            row_flags,
            row_limit,
            hit_rows,
            source_render.output_render(),
        )
    }

    pub(crate) fn emit_line_break(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
    ) -> TextMatrixRowTransition {
        let (builder, output_emitter, evaluator) = self.output_render.into_parts();
        DisplayRowTextWindowTransitionContext::new(
            self.defaults,
            self.row_base,
            self.row_y_positions,
            self.max_rows,
        )
        .emit_line_break(
            plan,
            hit_range,
            position,
            line_spacing,
            self.row_geometry,
            self.hit_rows,
            builder,
            output_emitter,
            evaluator,
        )
    }

    pub(crate) fn emit_line_break_then_row_start(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        line_spacing: f32,
        render_state: DisplayRowTransitionRenderState<'_>,
        col: &mut usize,
    ) -> TextMatrixRowTransition {
        let transition = self.emit_line_break(plan, hit_range, position, line_spacing);
        if !transition.is_exhausted() {
            render_state.apply_line_break_row_start(plan, col);
        }
        transition
    }

    pub(crate) fn emit_overflow(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
    ) -> TextMatrixRowTransition {
        let (builder, output_emitter, evaluator) = self.output_render.into_parts();
        DisplayRowTextWindowTransitionContext::new(
            self.defaults,
            self.row_base,
            self.row_y_positions,
            self.max_rows,
        )
        .emit_overflow(
            plan,
            hit_range,
            position,
            self.row_geometry,
            self.row_flags,
            self.row_limit,
            self.hit_rows,
            builder,
            output_emitter,
            evaluator,
        )
    }

    pub(crate) fn emit_overflow_then_row_start(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        hit_range: DisplayRowHitRange,
        position: DisplayRowPosition,
        render_state: DisplayRowTransitionRenderState<'_>,
        col: &mut usize,
    ) -> TextMatrixRowTransition {
        let transition = self.emit_overflow(plan, hit_range, position);
        if !transition.is_exhausted() {
            render_state.apply_overflow_row_start(plan, col);
        }
        transition
    }
}

impl<'a> DisplayRowTransitionRenderState<'a> {
    pub(crate) fn new(
        prefix_request: &'a mut DisplayRowPrefixRequest,
        has_prefix: bool,
        line_numbers: &'a mut LineNumberRenderState,
        hscroll_skip: &'a mut HorizontalScrollSkipState,
        word_wrap: &'a mut WordWrapRenderState,
        trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
    ) -> Self {
        Self {
            prefix_request,
            has_prefix,
            line_numbers,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
        }
    }

    fn apply_state_policy(&mut self, policy: TextRowTransitionStatePolicy) {
        let prefix_action = policy.apply(
            self.line_numbers,
            self.hscroll_skip,
            self.word_wrap,
            self.trailing_whitespace,
        );
        self.prefix_request
            .apply_transition_prefix_action(self.has_prefix, prefix_action);
    }

    pub(crate) fn apply_line_break_row_start(
        self,
        plan: DisplayRowLineBreakTransitionPlan,
        col: &mut usize,
    ) {
        plan.apply_row_start_prefix_state(col, self);
    }

    pub(crate) fn apply_overflow_prefix(self, plan: DisplayRowOverflowTransitionPlan) {
        plan.apply_prefix_state(self);
    }

    pub(crate) fn apply_overflow_row_start(
        self,
        plan: DisplayRowOverflowTransitionPlan,
        col: &mut usize,
    ) {
        plan.apply_row_start_prefix_state(col, self);
    }
}

impl DisplayRowTransitionContinuation {
    pub(crate) fn after_visible_row_transition(
        row_transition: TextMatrixRowTransition,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> Self {
        if row_transition.is_exhausted() {
            Self::Exhausted
        } else if row_geometry.current_row_is_visible(row_visibility_limit) {
            Self::Continue
        } else {
            Self::Hidden
        }
    }

    pub(crate) fn should_break(self) -> bool {
        !matches!(self, Self::Continue)
    }
}

impl DisplayRowLineBreakTransitionPlan {
    fn new(state_policy: TextRowTransitionStatePolicy) -> Self {
        Self { state_policy }
    }

    pub(crate) fn hscroll_line_break() -> Self {
        Self::new(TextRowTransitionStatePolicy::hscroll_line_break())
    }

    pub(crate) fn hidden_line_break() -> Self {
        Self::new(TextRowTransitionStatePolicy::hidden_line_break())
    }

    pub(crate) fn line_break() -> Self {
        Self::new(TextRowTransitionStatePolicy::line_break())
    }

    pub(crate) fn apply_prefix_state(self, mut state: DisplayRowTransitionRenderState<'_>) {
        state.apply_state_policy(self.state_policy);
    }

    pub(crate) fn apply_row_start_prefix_state(
        self,
        col: &mut usize,
        state: DisplayRowTransitionRenderState<'_>,
    ) {
        *col = 0;
        self.apply_prefix_state(state);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn request<'a>(
        self,
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        position: DisplayRowPosition,
        line_spacing: f32,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> DisplayRowLineBreakTransitionRequest<'a> {
        DisplayRowLineBreakTransitionRequest::new(
            hit_range,
            defaults,
            row_base,
            position.col,
            position.x_px,
            line_spacing,
            row_y_recording,
            max_rows,
        )
    }
}

impl<'a> DisplayRowLineBreakTransitionRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        line_spacing: f32,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> Self {
        Self {
            hit_range,
            defaults,
            row_base,
            col,
            x,
            line_spacing,
            row_y_recording,
            max_rows,
        }
    }

    fn boundary_target(self) -> DisplayRowBoundaryTarget<'a> {
        DisplayRowBoundaryTarget::line_break(
            self.hit_range,
            self.defaults,
            self.row_base,
            self.col,
            self.x,
            self.line_spacing,
            self.row_y_recording,
        )
    }

    pub(crate) fn finish_geometry(
        self,
        row_geometry: &mut DisplayRowGeometryState,
        hit_rows: &mut Vec<HitRow>,
    ) -> TextMatrixRowGeometryTransition {
        row_geometry.finish_boundary_and_record_hit(self.boundary_target(), hit_rows)
    }

    pub(crate) fn emit(
        self,
        row_geometry: &mut DisplayRowGeometryState,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        let max_rows = self.max_rows;
        DisplayRowBoundaryTransitionRequest::new(self.boundary_target(), max_rows).emit(
            row_geometry,
            hit_rows,
            builder,
            output_emitter,
            evaluator,
        )
    }
}

impl DisplayRowOverflowTransitionPlan {
    fn new(
        kind: DisplayRowOverflowTransitionKind,
        state_policy: TextRowTransitionStatePolicy,
    ) -> Self {
        Self { kind, state_policy }
    }

    fn truncation(state_policy: TextRowTransitionStatePolicy) -> Self {
        Self::new(DisplayRowOverflowTransitionKind::Truncation, state_policy)
    }

    fn visual_wrap(state_policy: TextRowTransitionStatePolicy) -> Self {
        Self::new(DisplayRowOverflowTransitionKind::VisualWrap, state_policy)
    }

    pub(crate) fn apply_prefix_state(self, mut state: DisplayRowTransitionRenderState<'_>) {
        state.apply_state_policy(self.state_policy);
    }

    pub(crate) fn apply_row_start_prefix_state(
        self,
        col: &mut usize,
        state: DisplayRowTransitionRenderState<'_>,
    ) {
        *col = 0;
        self.apply_prefix_state(state);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn request<'a>(
        self,
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        position: DisplayRowPosition,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> DisplayRowOverflowTransitionRequest<'a> {
        match self.kind {
            DisplayRowOverflowTransitionKind::Truncation => {
                DisplayRowOverflowTransitionRequest::truncation(
                    hit_range,
                    defaults,
                    row_base,
                    position.col,
                    position.x_px,
                    row_y_recording,
                    max_rows,
                )
            }
            DisplayRowOverflowTransitionKind::VisualWrap => {
                DisplayRowOverflowTransitionRequest::visual_wrap(
                    hit_range,
                    defaults,
                    row_base,
                    position.col,
                    position.x_px,
                    row_y_recording,
                    max_rows,
                )
            }
        }
    }
}

impl<'a> DisplayRowOverflowTransitionRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn truncation(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> Self {
        Self {
            kind: DisplayRowOverflowTransitionKind::Truncation,
            hit_range,
            defaults,
            row_base,
            col,
            x,
            row_y_recording,
            max_rows,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn visual_wrap(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
        max_rows: usize,
    ) -> Self {
        Self {
            kind: DisplayRowOverflowTransitionKind::VisualWrap,
            hit_range,
            defaults,
            row_base,
            col,
            x,
            row_y_recording,
            max_rows,
        }
    }

    fn boundary_target(self) -> DisplayRowBoundaryTarget<'a> {
        match self.kind {
            DisplayRowOverflowTransitionKind::Truncation => DisplayRowBoundaryTarget::truncation(
                self.hit_range,
                self.defaults,
                self.row_base,
                self.col,
                self.x,
                self.row_y_recording,
            ),
            DisplayRowOverflowTransitionKind::VisualWrap => DisplayRowBoundaryTarget::visual_wrap(
                self.hit_range,
                self.defaults,
                self.row_base,
                self.col,
                self.x,
                self.row_y_recording,
            ),
        }
    }

    pub(crate) fn emit(
        self,
        row_geometry: &mut DisplayRowGeometryState,
        row_flags: &mut DisplayRowFlags,
        row_limit: DisplayRowLimit,
        hit_rows: &mut Vec<HitRow>,
        builder: &mut GlyphMatrixBuilder,
        output_emitter: &mut WindowOutputEmitter,
        evaluator: &mut Context,
    ) -> TextMatrixRowTransition {
        match self.kind {
            DisplayRowOverflowTransitionKind::Truncation => {
                row_geometry.mark_current_row_flag_kind(
                    row_flags,
                    DisplayRowFlagKind::Truncated,
                    row_limit,
                );
            }
            DisplayRowOverflowTransitionKind::VisualWrap => {
                row_geometry.mark_current_row_flag_kind(
                    row_flags,
                    DisplayRowFlagKind::Continued,
                    row_limit,
                );
            }
        }
        let kind = self.kind;
        let max_rows = self.max_rows;
        let transition = DisplayRowBoundaryTransitionRequest::new(self.boundary_target(), max_rows)
            .emit(row_geometry, hit_rows, builder, output_emitter, evaluator);
        if kind == DisplayRowOverflowTransitionKind::VisualWrap && !transition.is_exhausted() {
            row_geometry.mark_current_row_flag_kind(
                row_flags,
                DisplayRowFlagKind::Continuation,
                row_limit,
            );
        }
        transition
    }
}

#[derive(Clone, Copy)]
pub(crate) struct LispStringRowAppendContext<'row> {
    active_face_context: DisplayRowActiveFaceAppendContext<'row, 'row>,
}

impl<'row> LispStringRowAppendContext<'row> {
    pub(crate) fn new(
        append_surface: &'row DisplayRowAppendSurface,
        geometry: &'row DisplayRowGeometryState,
        active_face: &'row DisplayRowActiveFaceState,
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

    fn render_active_face_source_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        base_face_id: u32,
        base_face: &'row ResolvedFace,
        request: LispStringSourceAppendRequest,
    ) -> DisplayRowPosition {
        let position = request.position();
        let Some(mut source_session) =
            LispStringSourceAppendSession::new(request, base_face_id, base_face)
        else {
            return position;
        };
        let frame = self.active_face_context.active_face_frame();
        source_session
            .render_to_text_row_and_emit(state, face_ids, frame, position)
            .map(|outcome| outcome.end_position())
            .unwrap_or(position)
    }

    pub(crate) fn render_prefix_source_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        base_face: &DisplayStringBaseFace,
        prefix_source: DisplayRowPrefixSource,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        self.render_active_face_source_request_to_text_row_and_emit(
            state,
            face_ids,
            base_face.face_id(),
            base_face.face(),
            prefix_source.append_request(position),
        )
    }
}

fn render_lisp_string_source_append_to_text_row_and_emit(
    state: &mut TextRowSourceRenderState<'_>,
    source: &mut LispStringSourceCursor,
    source_state: &mut DisplayRowSourceState,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> Option<CurrentTextRowRenderOutcome> {
    DisplayRowSourceAppendOperation::new(
        base_face,
        base_face_id,
        frame,
        position,
        DisplayRowAppendKind::SourceText,
    )
    .render_source_cursor_to_text_row_and_emit(state, source, source_state, face_ids)
}

pub(crate) struct LispStringSourceAppendContext<'a> {
    source: &'a mut LispStringSourceCursor,
    source_state: &'a mut DisplayRowSourceState,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
}

impl<'a> LispStringSourceAppendContext<'a> {
    pub(crate) fn new(
        source: &'a mut LispStringSourceCursor,
        source_state: &'a mut DisplayRowSourceState,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> Self {
        Self {
            source,
            source_state,
            base_face_id,
            base_face,
        }
    }

    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        render_lisp_string_source_append_to_text_row_and_emit(
            state,
            self.source,
            self.source_state,
            self.base_face,
            self.base_face_id,
            face_ids,
            frame,
            position,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LispStringSourceAppendRequest {
    position: DisplayRowPosition,
    source_id: LispStringSourceId,
    value: Value,
}

impl LispStringSourceAppendRequest {
    fn new(position: DisplayRowPosition, source_id: LispStringSourceId, value: Value) -> Self {
        Self {
            position,
            source_id,
            value,
        }
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn into_source(self, base_face_id: u32) -> Option<LispStringSourceCursor> {
        LispStringSourceCursor::new(
            self.source_id.raw(),
            self.value,
            RenderFaceRef::FaceId(base_face_id),
        )
    }
}

pub(crate) struct LispStringSourceAppendSession<'a> {
    source: LispStringSourceCursor,
    source_state: DisplayRowSourceState,
    base_face_id: u32,
    base_face: &'a ResolvedFace,
}

impl<'a> LispStringSourceAppendSession<'a> {
    fn new(
        request: LispStringSourceAppendRequest,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> Option<Self> {
        let source = request.into_source(base_face_id)?;
        Some(Self {
            source,
            source_state: DisplayRowSourceState::default(),
            base_face_id,
            base_face,
        })
    }

    fn append_context(&mut self) -> LispStringSourceAppendContext<'_> {
        LispStringSourceAppendContext::new(
            &mut self.source,
            &mut self.source_state,
            self.base_face_id,
            self.base_face,
        )
    }

    fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        frame: DisplayRowAppendFrame,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.append_context()
            .render_to_text_row_and_emit(state, face_ids, frame, position)
    }

    fn discard_pending_until_row_break(&mut self) -> bool {
        self.source_state.discard_pending_item();
        self.source.discard_until_row_break()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayRowPrefixRequest {
    None,
    Line,
    Wrap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowPrefixValues {
    line_property: Option<Value>,
    wrap_property: Option<Value>,
    line_default: Option<Value>,
    wrap_default: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayRowPrefixKind {
    Line,
    Wrap,
}

#[derive(Clone, Copy)]
enum BufferAnchoredLispStringSourceKind {
    OverlayString {
        overlay_id: Value,
        kind: OverlayStringKind,
    },
    Prefix(DisplayRowPrefixKind),
}

#[derive(Clone, Copy)]
struct BufferAnchoredLispStringSource {
    value: Value,
    anchor_charpos: CharPos0,
    kind: BufferAnchoredLispStringSourceKind,
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayRowPrefixSource {
    source: BufferAnchoredLispStringSource,
}

pub(crate) struct BufferLinePrefixRenderContext<'a> {
    values: DisplayRowPrefixValues,
    append_surface: &'a DisplayRowAppendSurface,
    row_geometry: &'a DisplayRowGeometryState,
    active_face_state: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
}

pub(crate) struct BufferLinePrefixRenderRequest<'a> {
    context: BufferLinePrefixRenderContext<'a>,
    position: DisplayRowPosition,
}

impl DisplayRowPrefixRequest {
    pub(crate) fn initial(has_prefix: bool, has_line_prefix: bool) -> Self {
        if has_prefix && has_line_prefix {
            Self::Line
        } else {
            Self::None
        }
    }

    pub(crate) fn request_line(&mut self) {
        *self = Self::Line;
    }

    pub(crate) fn request_wrap(&mut self) {
        *self = Self::Wrap;
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::None;
    }

    pub(crate) fn is_requested(self) -> bool {
        !matches!(self, Self::None)
    }

    pub(crate) fn apply_transition_prefix_action(
        &mut self,
        has_prefix: bool,
        action: TextRowTransitionPrefixAction,
    ) {
        if !has_prefix {
            return;
        }
        match action {
            TextRowTransitionPrefixAction::Line => self.request_line(),
            TextRowTransitionPrefixAction::Wrap => self.request_wrap(),
        }
    }

    pub(crate) fn source_for_value(
        self,
        value: Value,
        anchor_charpos: CharPos0,
    ) -> Option<DisplayRowPrefixSource> {
        let kind = match self {
            Self::Line => DisplayRowPrefixKind::Line,
            Self::Wrap => DisplayRowPrefixKind::Wrap,
            Self::None => return None,
        };
        Some(DisplayRowPrefixSource {
            source: BufferAnchoredLispStringSource::prefix(value, anchor_charpos, kind),
        })
    }

    pub(crate) fn source_from_values(
        self,
        values: DisplayRowPrefixValues,
        anchor_charpos: CharPos0,
    ) -> Option<DisplayRowPrefixSource> {
        let value = match self {
            Self::Line => values.line_property.or(values.line_default),
            Self::Wrap => values.wrap_property.or(values.wrap_default),
            Self::None => None,
        }?;
        self.source_for_value(value, anchor_charpos)
    }
}

impl DisplayRowPrefixValues {
    pub(crate) fn new(
        line_property: Option<Value>,
        wrap_property: Option<Value>,
        line_default: Option<Value>,
        wrap_default: Option<Value>,
    ) -> Self {
        Self {
            line_property: Self::lisp_string_value(line_property),
            wrap_property: Self::lisp_string_value(wrap_property),
            line_default: Self::lisp_string_value(line_default),
            wrap_default: Self::lisp_string_value(wrap_default),
        }
    }

    fn lisp_string_value(value: Option<Value>) -> Option<Value> {
        value.filter(|value| value.as_lisp_string().is_some())
    }

    pub(crate) fn default_values(line_default: Option<Value>, wrap_default: Option<Value>) -> Self {
        Self::new(None, None, line_default, wrap_default)
    }

    pub(crate) fn with_properties(
        self,
        line_property: Option<Value>,
        wrap_property: Option<Value>,
    ) -> Self {
        Self::new(
            line_property,
            wrap_property,
            self.line_default,
            self.wrap_default,
        )
    }

    pub(crate) fn has_default_prefix(self) -> bool {
        self.line_default.is_some() || self.wrap_default.is_some()
    }

    pub(crate) fn has_line_default_prefix(self) -> bool {
        self.line_default.is_some()
    }
}

impl BufferAnchoredLispStringSource {
    fn prefix(value: Value, anchor_charpos: CharPos0, kind: DisplayRowPrefixKind) -> Self {
        Self {
            value,
            anchor_charpos,
            kind: BufferAnchoredLispStringSourceKind::Prefix(kind),
        }
    }

    fn overlay_string(
        value: Value,
        overlay_id: Value,
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    ) -> Self {
        Self {
            value,
            anchor_charpos,
            kind: BufferAnchoredLispStringSourceKind::OverlayString { overlay_id, kind },
        }
    }

    fn anchor_i64(self) -> i64 {
        self.anchor_charpos.get() as i64
    }

    fn value(self) -> Value {
        self.value
    }

    fn origin(self) -> DisplayOrigin {
        match self.kind {
            BufferAnchoredLispStringSourceKind::OverlayString { overlay_id, kind } => {
                DisplayOrigin::OverlayString {
                    overlay_id,
                    anchor_charpos: self.anchor_charpos,
                    kind,
                }
            }
            BufferAnchoredLispStringSourceKind::Prefix(DisplayRowPrefixKind::Line) => {
                DisplayOrigin::LinePrefix {
                    anchor_charpos: self.anchor_charpos,
                }
            }
            BufferAnchoredLispStringSourceKind::Prefix(DisplayRowPrefixKind::Wrap) => {
                DisplayOrigin::WrapPrefix {
                    anchor_charpos: self.anchor_charpos,
                }
            }
        }
    }

    #[cfg(test)]
    fn base_face_policy(self) -> BaseFacePolicy {
        self.origin().default_base_face_policy()
    }

    fn source_id(self) -> LispStringSourceId {
        match self.kind {
            BufferAnchoredLispStringSourceKind::OverlayString { .. } => {
                LispStringSourceId::OVERLAY_STRING
            }
            BufferAnchoredLispStringSourceKind::Prefix(_) => LispStringSourceId::PREFIX,
        }
    }

    fn append_request(self, position: DisplayRowPosition) -> LispStringSourceAppendRequest {
        LispStringSourceAppendRequest::new(position, self.source_id(), self.value)
    }
}

impl DisplayRowPrefixSource {
    #[cfg(test)]
    pub(crate) fn value(self) -> Value {
        self.source.value()
    }

    pub(crate) fn origin(self) -> DisplayOrigin {
        self.source.origin()
    }

    #[cfg(test)]
    pub(crate) fn base_face_policy(self) -> BaseFacePolicy {
        self.source.base_face_policy()
    }

    fn append_request(self, position: DisplayRowPosition) -> LispStringSourceAppendRequest {
        self.source.append_request(position)
    }
}

impl<'a> BufferLinePrefixRenderContext<'a> {
    pub(crate) fn new(
        values: DisplayRowPrefixValues,
        append_surface: &'a DisplayRowAppendSurface,
        row_geometry: &'a DisplayRowGeometryState,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            values,
            append_surface,
            row_geometry,
            active_face_state,
            glyph_y_offset,
            default_row_height,
        }
    }

    pub(crate) fn render_requested_to_text_row_and_emit<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        state: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        if !request.is_requested() {
            return position;
        }

        let text_props = RustTextPropAccess::new(buffer);
        let line_property = text_props.get_property(anchor_charpos, Value::symbol("line-prefix"));
        let wrap_property = text_props.get_property(anchor_charpos, Value::symbol("wrap-prefix"));
        let source = request.source_from_values(
            self.values.with_properties(line_property, wrap_property),
            CharPos0::new(anchor_charpos as usize),
        );
        request.clear();

        let Some(prefix_source) = source else {
            return position;
        };

        let prefix_base_face =
            state.default_display_string_base_face(buffer, prefix_source.origin(), face_ids);
        LispStringRowAppendContext::new(
            self.append_surface,
            self.row_geometry,
            self.active_face_state,
            self.glyph_y_offset,
            self.default_row_height,
        )
        .render_prefix_source_to_text_row_and_emit(
            state,
            face_ids,
            &prefix_base_face,
            prefix_source,
            position,
        )
    }
}

impl<'a> BufferLinePrefixRenderRequest<'a> {
    pub(crate) fn new(
        context: BufferLinePrefixRenderContext<'a>,
        position: DisplayRowPosition,
    ) -> Self {
        Self { context, position }
    }

    pub(crate) fn render_requested_with_source_state_and_apply<B: LayoutBufferView>(
        self,
        request: &mut DisplayRowPrefixRequest,
        source_render: &mut TextRowSourceRenderState<'_>,
        buffer: &B,
        anchor_charpos: i64,
        face_ids: &mut FrameFaceIdAllocator,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.context.render_requested_to_text_row_and_emit(
            request,
            source_render,
            buffer,
            anchor_charpos,
            face_ids,
            self.position,
        );
        *x = position.x_px;
        *col = position.col;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRenderSource {
    source: BufferAnchoredLispStringSource,
}

impl OverlayStringRenderSource {
    pub(crate) fn new(
        overlay_string: OverlayDisplayString,
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    ) -> Self {
        Self {
            source: BufferAnchoredLispStringSource::overlay_string(
                overlay_string.string,
                overlay_string.overlay_id,
                anchor_charpos,
                kind,
            ),
        }
    }

    pub(crate) fn anchor_i64(self) -> i64 {
        self.source.anchor_i64()
    }

    pub(crate) fn value(self) -> Value {
        self.source.value()
    }

    pub(crate) fn origin(self) -> DisplayOrigin {
        self.source.origin()
    }

    #[cfg(test)]
    pub(crate) fn base_face_policy(self) -> BaseFacePolicy {
        self.source.base_face_policy()
    }

    pub(crate) fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> LispStringSourceAppendRequest {
        self.source.append_request(position)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OverlayStringRenderRowContext<'a> {
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) face_char_w: f32,
    pub(crate) char_h: f32,
    pub(crate) default_row_ascent: f32,
    text_y: f32,
    pub(crate) row_base: usize,
    pub(crate) max_rows: usize,
}

impl<'a> OverlayStringRenderRowContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &DisplayRowActiveFaceState,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self {
            append_surface,
            face_char_w: active_face_state.metrics().char_width,
            char_h,
            default_row_ascent,
            text_y,
            row_base,
            max_rows,
        }
    }

    pub(crate) fn content_x(self) -> f32 {
        self.append_surface.content_x()
    }

    pub(crate) fn right_edge(self) -> f32 {
        self.append_surface.right_edge()
    }

    pub(crate) fn geometry_defaults(self) -> DisplayRowGeometryDefaults {
        DisplayRowGeometryDefaults::new(self.text_y, self.char_h, self.default_row_ascent)
    }

    pub(crate) fn row_limit(self) -> DisplayRowLimit {
        DisplayRowLimit {
            max_rows: self.max_rows,
        }
    }

    pub(crate) fn cursor_visual_state(self, base_face: &ResolvedFace) -> CapturedCursorVisualState {
        CapturedCursorVisualState {
            face_width: self.face_char_w,
            face_height: self.char_h,
            face_ascent: self.default_row_ascent,
            background: neomacs_display_protocol::types::Color::from_pixel(base_face.bg),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferOverlayStringRenderContext<'a> {
    enabled: bool,
    window_id: u64,
    row_context: OverlayStringRenderRowContext<'a>,
}

pub(crate) struct OverlayStringRenderState<'a> {
    pub(crate) source_render: TextRowSourceRenderState<'a>,
    pub(crate) x: &'a mut f32,
    pub(crate) col: &'a mut usize,
    pub(crate) geometry: &'a mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'a mut CursorCaptureState,
    pub(crate) hit_rows: &'a mut Vec<HitRow>,
    pub(crate) hit_row_range: &'a mut HitRowRangeTracker,
    pub(crate) row_y_positions: &'a mut DisplayRowYPositions,
    pub(crate) face_ids: &'a mut FrameFaceIdAllocator,
}

impl<'a> OverlayStringRenderState<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_source_render(
        source_render: TextRowSourceRenderState<'a>,
        x: &'a mut f32,
        col: &'a mut usize,
        geometry: &'a mut DisplayRowGeometryState,
        cursor_info: &'a mut CursorCaptureState,
        hit_rows: &'a mut Vec<HitRow>,
        hit_row_range: &'a mut HitRowRangeTracker,
        row_y_positions: &'a mut DisplayRowYPositions,
        face_ids: &'a mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            source_render,
            x,
            col,
            geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BufferOverlayStringTextRowRenderContext<'a> {
    enabled: bool,
    window_id: u64,
    append_surface: &'a DisplayRowAppendSurface,
    char_h: f32,
    default_row_ascent: f32,
    text_y: f32,
    row_base: usize,
    max_rows: usize,
}

impl<'a> BufferOverlayStringTextRowRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        enabled: bool,
        window_id: u64,
        append_surface: &'a DisplayRowAppendSurface,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self {
            enabled,
            window_id,
            append_surface,
            char_h,
            default_row_ascent,
            text_y,
            row_base,
            max_rows,
        }
    }

    fn overlay_context(
        self,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> BufferOverlayStringRenderContext<'a> {
        BufferOverlayStringRenderContext::for_text_row(
            self.enabled,
            self.window_id,
            self.append_surface,
            active_face_state,
            self.char_h,
            self.default_row_ascent,
            self.text_y,
            self.row_base,
            self.max_rows,
        )
    }

    pub(crate) fn render_before_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_before_at(buffer, anchor_charpos, state);
    }

    pub(crate) fn render_after_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_after_at(buffer, anchor_charpos, state);
    }

    pub(crate) fn render_both_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        active_face_state: &DisplayRowActiveFaceState,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.overlay_context(active_face_state)
            .render_both_at(buffer, anchor_charpos, state);
    }
}

impl<'a> BufferOverlayStringRenderContext<'a> {
    pub(crate) fn new(
        enabled: bool,
        window_id: u64,
        row_context: OverlayStringRenderRowContext<'a>,
    ) -> Self {
        Self {
            enabled,
            window_id,
            row_context,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_text_row(
        enabled: bool,
        window_id: u64,
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &DisplayRowActiveFaceState,
        char_h: f32,
        default_row_ascent: f32,
        text_y: f32,
        row_base: usize,
        max_rows: usize,
    ) -> Self {
        Self::new(
            enabled,
            window_id,
            OverlayStringRenderRowContext::new(
                append_surface,
                active_face_state,
                char_h,
                default_row_ascent,
                text_y,
                row_base,
                max_rows,
            ),
        )
    }

    pub(crate) fn render_before_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_at_kind(buffer, anchor_charpos, OverlayStringKind::Before, state);
    }

    pub(crate) fn render_after_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_at_kind(buffer, anchor_charpos, OverlayStringKind::After, state);
    }

    pub(crate) fn render_both_at<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        self.render_before_at(buffer, anchor_charpos, state);
        self.render_after_at(buffer, anchor_charpos, state);
    }

    fn render_at_kind<B: LayoutBufferView>(
        self,
        buffer: &B,
        anchor_charpos: i64,
        kind: OverlayStringKind,
        state: &mut OverlayStringRenderState<'_>,
    ) {
        if !self.enabled {
            return;
        }
        let text_props = RustTextPropAccess::new_for_window(buffer, self.window_id);
        // overlay_strings_at now returns one GNU-ordered interleaved list; pick
        // the entries of this kind (within-kind order is preserved, so this is
        // behavior-neutral vs the old per-kind sort).
        let want_after = matches!(kind, OverlayStringKind::After);
        let overlay_strings: Vec<_> = text_props
            .overlay_strings_at(anchor_charpos)
            .into_iter()
            .filter(|entry| entry.after_string_p == want_after)
            .collect();
        for overlay_string in overlay_strings {
            render_overlay_string(
                buffer,
                OverlayStringRenderSource::new(
                    overlay_string,
                    CharPos0::new(anchor_charpos as usize),
                    kind,
                ),
                self.row_context,
                state,
            );
        }
    }
}

pub(crate) struct LispStringSourceRowAppendContext<'a> {
    source_context: LispStringSourceAppendContext<'a>,
    append_surface: &'a DisplayRowAppendSurface,
    glyph_y_offset: f32,
    metrics: DisplayRowAppendMetrics,
}

impl<'a> LispStringSourceRowAppendContext<'a> {
    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        geometry: &DisplayRowGeometryState,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let frame = DisplayRowTextAppendContext::new(
            self.append_surface,
            geometry,
            self.glyph_y_offset,
            self.metrics.default_row_height,
        )
        .text_row_frame(
            self.metrics.height,
            self.metrics.ascent,
            self.metrics.char_width,
        );
        self.source_context
            .render_to_text_row_and_emit(state, face_ids, frame, position)
    }
}

pub(crate) struct LispStringSourceRowAppendSession<'a> {
    source_session: LispStringSourceAppendSession<'a>,
    append_surface: &'a DisplayRowAppendSurface,
    glyph_y_offset: f32,
    metrics: DisplayRowAppendMetrics,
}

impl<'a> LispStringSourceRowAppendSession<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        request: LispStringSourceAppendRequest,
        base_face_id: u32,
        base_face: &'a ResolvedFace,
        append_surface: &'a DisplayRowAppendSurface,
        glyph_y_offset: f32,
        height: f32,
        ascent: f32,
        char_width: f32,
        default_row_height: f32,
    ) -> Option<Self> {
        let source_session = LispStringSourceAppendSession::new(request, base_face_id, base_face)?;
        Some(Self {
            source_session,
            append_surface,
            glyph_y_offset,
            metrics: DisplayRowAppendMetrics::text_row(
                height,
                ascent,
                char_width,
                default_row_height,
            ),
        })
    }

    fn append_context(&mut self) -> LispStringSourceRowAppendContext<'_> {
        LispStringSourceRowAppendContext {
            source_context: self.source_session.append_context(),
            append_surface: self.append_surface,
            glyph_y_offset: self.glyph_y_offset,
            metrics: self.metrics,
        }
    }

    pub(crate) fn render_to_text_row_and_emit(
        &mut self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        geometry: &DisplayRowGeometryState,
        position: DisplayRowPosition,
    ) -> Option<CurrentTextRowRenderOutcome> {
        self.append_context()
            .render_to_text_row_and_emit(state, face_ids, geometry, position)
    }

    pub(crate) fn discard_pending_until_row_break(&mut self) -> bool {
        self.source_session.discard_pending_until_row_break()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyntheticTextSource {
    source_id: u64,
    text: Box<str>,
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

#[derive(Clone)]
pub(crate) struct SyntheticTextAppendContext<'a> {
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> SyntheticTextAppendContext<'a> {
    pub(crate) fn new(
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            face_id,
            base_face,
            frame,
        }
    }

    pub(crate) fn append_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
        source: SyntheticTextSource,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyntheticTextMarker {
    InvisibleEllipsis,
    HscrollTruncation,
    SelectiveEllipsis,
}

impl SyntheticTextMarker {
    fn source_id(self) -> u64 {
        match self {
            Self::InvisibleEllipsis => SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS,
            Self::HscrollTruncation => SYNTHETIC_SOURCE_HSCROLL_TRUNCATION,
            Self::SelectiveEllipsis => SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS,
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::InvisibleEllipsis | Self::SelectiveEllipsis => "...",
            Self::HscrollTruncation => "$",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SyntheticTextRowAppendContext<'a> {
    active_face_context: DisplayRowActiveFaceAppendContext<'a, 'a>,
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
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let (position, source, face) = request.into_parts();
        match face {
            SyntheticTextAppendFace::ActiveFace => {
                let active_face = self.active_face_context.active_face;
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

#[derive(Clone, Copy)]
pub(crate) struct BufferSyntheticTextRenderContext<'a> {
    append_surface: &'a DisplayRowAppendSurface,
    active_face: &'a DisplayRowActiveFaceState,
    glyph_y_offset: f32,
    default_row_height: f32,
    default_row_ascent: f32,
    default_char_width: f32,
}

impl<'a> BufferSyntheticTextRenderContext<'a> {
    pub(crate) fn new(
        append_surface: &'a DisplayRowAppendSurface,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
        default_row_ascent: f32,
        default_char_width: f32,
    ) -> Self {
        Self {
            append_surface,
            active_face,
            glyph_y_offset,
            default_row_height,
            default_row_ascent,
            default_char_width,
        }
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
            self.default_row_height,
        )
    }

    pub(crate) fn render_request_to_text_row<'face>(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        geometry: &'a DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
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
        .map(|(_progress, position)| position)
    }

    pub(crate) fn hscroll_truncation_request(
        self,
        base_face: ResolvedFace,
        content_x: f32,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::text_row_metrics_marker(
            DisplayRowPosition {
                x_px: content_x,
                col: 0,
            },
            SyntheticTextMarker::HscrollTruncation,
            BasicFaceId::Default.into(),
            &base_face,
            self.default_row_height,
            self.default_row_ascent,
            self.default_char_width,
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
            .map(|(_progress, position)| position)
    }
}

pub(crate) fn render_face_ref_id(face: RenderFaceRef, fallback: u32) -> u32 {
    match face {
        RenderFaceRef::FaceId(face_id) => face_id,
        RenderFaceRef::Inherit => fallback,
    }
}

#[cfg(test)]
pub(crate) fn apply_pending_display_source_faces(
    builder: &mut GlyphMatrixBuilder,
    pending_faces: &mut Vec<PendingDisplaySourceFace>,
) {
    for pending in pending_faces.drain(..) {
        crate::display_row::insert_resolved_display_row_face(
            builder,
            pending.face_id,
            &pending.resolved,
            None,
        );
    }
}

#[cfg(test)]
pub(crate) fn append_lisp_string_to_text_row(
    state: &mut TextRowSourceRenderState<'_>,
    text_value: Value,
    source_id: u64,
    base_face: &ResolvedFace,
    base_face_id: u32,
    face_ids: &mut FrameFaceIdAllocator,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
) -> DisplayRowPosition {
    let request =
        LispStringSourceAppendRequest::new(position, LispStringSourceId(source_id), text_value);
    let Some(mut source_session) =
        LispStringSourceAppendSession::new(request, base_face_id, base_face)
    else {
        return position;
    };
    source_session
        .render_to_text_row_and_emit(state, face_ids, frame, position)
        .map(|outcome| outcome.end_position())
        .unwrap_or(position)
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

struct DisplayRowSourceAppendOperation<'face> {
    base_face: &'face ResolvedFace,
    base_face_id: u32,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    kind: DisplayRowAppendKind,
}

impl<'face> DisplayRowSourceAppendOperation<'face> {
    fn new(
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

    fn for_single_item(
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

    fn render_single_item_to_text_row_and_emit<P: DisplayRowRenderPolicy>(
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

    fn render_source_to_text_row_and_emit<S: DisplayItemSource, P: DisplayRowRenderPolicy>(
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

    fn render_source_cursor_to_text_row_and_emit<S: DisplayItemSource>(
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
pub(crate) enum BufferTextSourceCharOverflowAction {
    Fits,
    Truncate {
        transition: DisplayRowOverflowTransitionPlan,
    },
    WordWrap {
        break_candidate: WordWrapBreakCandidate,
        transition: DisplayRowOverflowTransitionPlan,
    },
    CharacterWrap {
        transition: DisplayRowOverflowTransitionPlan,
    },
}

impl BufferTextSourceCharOverflowAction {
    fn for_decision(decision: BufferTextRowOverflowDecision) -> Self {
        match decision {
            BufferTextRowOverflowDecision::Fits => Self::Fits,
            BufferTextRowOverflowDecision::Truncate => Self::Truncate {
                transition: DisplayRowOverflowTransitionPlan::truncation(
                    TextRowTransitionStatePolicy::truncation(),
                ),
            },
            BufferTextRowOverflowDecision::WordWrap { break_candidate } => Self::WordWrap {
                break_candidate,
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::visual_wrap(),
                ),
            },
            BufferTextRowOverflowDecision::CharacterWrap => Self::CharacterWrap {
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::character_wrap(),
                ),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextSourceCharPreparedAppend {
    plan: BufferTextSourceCharAppendPlan,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextOverflowRenderRequest {
    prepared_append: BufferTextSourceCharPreparedAppend,
    decoded_source_char: BufferTextDecodedSourceChar,
    context: BufferTextOverflowRenderContext,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextOverflowRenderContext {
    pub(crate) ch: char,
    pub(crate) right_edge_px: f32,
    pub(crate) wrap_mode: LineWrapMode,
    pub(crate) word_wrap: WordWrapRenderState,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextOverflowRenderOutcome {
    Fits,
    Transition(DisplayRowTransitionContinuation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceAppendContinuation {
    Rendered,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSourceCharRenderOutcome {
    Rendered,
    ContinueBufferWalk,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSelectiveDisplayTailRenderOutcome {
    NotHidden,
    ContinueBufferWalk,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferInvisibleTextRenderOutcome {
    Visible,
    ContinueBufferWalk,
}

pub(crate) struct BufferTextSourceCharRenderState<'a> {
    source_render: TextRowSourceRenderState<'a>,
    trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
    word_wrap: &'a mut WordWrapRenderState,
    x: &'a mut f32,
    col: &'a mut usize,
    charpos: &'a mut i64,
}

impl<'a> BufferTextSourceCharRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        trailing_whitespace: &'a mut TrailingWhitespaceRenderState,
        word_wrap: &'a mut WordWrapRenderState,
        x: &'a mut f32,
        col: &'a mut usize,
        charpos: &'a mut i64,
    ) -> Self {
        Self {
            source_render,
            trailing_whitespace,
            word_wrap,
            x,
            col,
            charpos,
        }
    }
}

pub(crate) struct BufferTextSpecialSourceCharRenderState<'a> {
    face_ids: &'a mut FrameFaceIdAllocator,
    source_render: TextRowSourceRenderState<'a>,
    face_scan: &'a mut FaceScanCheckpoint,
    word_wrap: &'a mut WordWrapRenderState,
    x: &'a mut f32,
    col: &'a mut usize,
    charpos: &'a mut i64,
}

impl<'a> BufferTextSpecialSourceCharRenderState<'a> {
    pub(crate) fn new(
        face_ids: &'a mut FrameFaceIdAllocator,
        source_render: TextRowSourceRenderState<'a>,
        face_scan: &'a mut FaceScanCheckpoint,
        word_wrap: &'a mut WordWrapRenderState,
        x: &'a mut f32,
        col: &'a mut usize,
        charpos: &'a mut i64,
    ) -> Self {
        Self {
            face_ids,
            source_render,
            face_scan,
            word_wrap,
            x,
            col,
            charpos,
        }
    }
}

impl BufferTextSourceAppendContinuation {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stopped)
    }
}

impl BufferTextSourceCharRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferSelectiveDisplayTailRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(self, Self::Stop)
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferInvisibleTextRenderOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }
}

impl BufferTextOverflowRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(
            self,
            Self::Transition(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            )
        )
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(
            self,
            Self::Transition(DisplayRowTransitionContinuation::Continue)
        )
    }
}

impl<'a> BufferSelectiveDisplayTailRenderRequest<'a> {
    pub(crate) fn new(
        source_char: BufferTextDecodedSourceChar,
        context: BufferSelectiveDisplayTailRenderContext<'a>,
    ) -> Self {
        Self {
            source_char,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferSelectiveDisplayTailRenderState<'_, '_>,
    ) -> BufferSelectiveDisplayTailRenderOutcome {
        let context = self.context;
        let selective_display = BufferSelectiveDisplayContext::new(
            context.text,
            context.selective_display,
            context.tab_width,
        );
        let Some(marker) = selective_display.carriage_return_tail_marker(self.source_char.ch())
        else {
            return BufferSelectiveDisplayTailRenderOutcome::NotHidden;
        };

        let BufferSelectiveDisplayTailRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            box_face,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            row_y_positions,
        } = state;
        let mut source_render = source_render;

        let mut synthetic_text_state =
            BufferSyntheticTextRenderState::new(source_render.reborrow(), x, col);
        marker.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                context.glyph_y_offset,
                context.char_h,
                context.default_face_ascent,
                context.char_w,
            ),
            row_geometry,
            &mut synthetic_text_state,
        );

        let tail_action =
            selective_display.skip_rest_of_line_after_carriage_return(byte_idx, charpos);
        if !tail_action.is_line_break() {
            return BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk;
        }

        tail_action.apply_hidden_line_break_row_state(
            row_geometry,
            row_extend,
            box_face,
            context.content_x,
            x,
        );
        let line_break_transition = DisplayRowLineBreakTransitionPlan::hidden_line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            context.row_geometry_defaults,
            context.text_matrix_row_base,
            row_y_positions,
            context.max_rows,
            row_geometry,
            row_flags,
            context.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(*charpos),
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
            0.0,
            DisplayRowTransitionRenderState::new(
                prefix_request,
                context.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            col,
        );
        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                context.text_start_byte + *byte_idx,
            ))
            .get() as i64;
        if tail_action
            .apply_after_hidden_line_break_transition(
                row_transition,
                synced_charpos,
                charpos,
                hit_row_range,
            )
            .should_break()
        {
            return BufferSelectiveDisplayTailRenderOutcome::Stop;
        }

        BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk
    }
}

impl<'a> BufferTextSourceCharRenderRequest<'a> {
    #[cfg(test)]
    pub(crate) fn new(
        decoded_source_char: BufferTextDecodedSourceChar,
        context: BufferTextSourceCharRenderContext<'a>,
    ) -> Self {
        Self::from_source_event(BufferTextSourceTextEvent::new(decoded_source_char), context)
    }

    pub(crate) fn from_source_event(
        source_event: BufferTextSourceTextEvent,
        context: BufferTextSourceCharRenderContext<'a>,
    ) -> Self {
        Self {
            source_event,
            context,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextSourceCharRenderRequestState<'_, '_>,
    ) -> BufferTextSourceCharRenderOutcome {
        let BufferTextSourceCharRenderRequestState {
            append_state,
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            face_scan,
            row_y_positions,
            cursor_info,
            face_ids,
            raise_span,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let decoded_source_char = self.source_event.decoded_char();
        let ch = decoded_source_char.ch();
        decoded_source_char.record_word_wrap_candidate(word_wrap, source_render.output_emitter());

        let buffer_source_char = self
            .source_event
            .source_char(context.params.nobreak_char_display);
        let buffer_row_append_context = BufferTextRowAppendContext::new(
            buffer,
            context.buffer_id,
            context.append_surface,
            context.active_face_state,
            context.glyph_y_offset,
            context.char_h,
        );
        let append_position = DisplayRowPosition {
            x_px: *x,
            col: *col,
        };
        let append_geometry = *row_geometry;

        let prepared_append = {
            let mut preparation_state = BufferTextSourceCharPreparationState::from_source_render(
                append_state,
                &mut source_render,
            );
            buffer_row_append_context.prepare_source_char_for_current_text_row(
                BufferTextSourceCharPreparationRequest::new(
                    append_geometry,
                    &buffer_source_char,
                    context.text,
                    decoded_source_char.start_byte_idx(),
                    append_position,
                ),
                &mut preparation_state,
            )
        };

        let prepared_append = match prepared_append {
            BufferTextPreparedSourceCharAppend::Special(special_prepared_append) => {
                let special_overflow_outcome = BufferTextSpecialOverflowRenderRequest::new(
                    &special_prepared_append,
                    BufferTextSpecialOverflowRenderContext {
                        text: context.text,
                        text_start_byte: context.text_start_byte,
                        x_px: *x,
                        right_edge_px: context.append_surface.full_text_right_edge(),
                        wrap_mode: context.params.wrap_mode,
                        row_visibility_limit: context.row_visibility_limit,
                        content_x: context.content_x,
                        has_prefix: context.has_prefix,
                        row_geometry_defaults: context.row_geometry_defaults,
                        text_matrix_row_base: context.text_matrix_row_base,
                        max_rows: context.max_rows,
                        row_limit: context.row_limit,
                    },
                )
                .render_if_needed_and_apply(
                    buffer,
                    BufferTextSpecialOverflowRenderState {
                        byte_idx,
                        charpos,
                        col,
                        source_render: source_render.reborrow(),
                        row_extend,
                        x,
                        line_numbers,
                        row_geometry,
                        row_flags,
                        hit_rows,
                        hit_row_range,
                        prefix_request,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                        row_y_positions,
                    },
                );
                if special_overflow_outcome.should_break() {
                    return BufferTextSourceCharRenderOutcome::Stop;
                }
                if special_overflow_outcome.should_continue_buffer_walk() {
                    return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
                }

                if special_prepared_append
                    .append_to_text_row_and_apply(
                        &buffer_row_append_context,
                        row_geometry,
                        context.params,
                        &mut BufferTextSpecialSourceCharRenderState::new(
                            face_ids,
                            source_render.reborrow(),
                            face_scan,
                            word_wrap,
                            x,
                            col,
                            charpos,
                        ),
                    )
                    .should_break()
                {
                    return BufferTextSourceCharRenderOutcome::Stop;
                }
                return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
            }
            BufferTextPreparedSourceCharAppend::Text(prepared_append) => prepared_append,
        };

        prepared_append
            .update_cursor_info_for_main_char(cursor_info, decoded_source_char.start_byte_idx());
        let overflow_outcome = BufferTextOverflowRenderRequest::new(
            prepared_append,
            decoded_source_char,
            BufferTextOverflowRenderContext {
                ch,
                right_edge_px: context.append_surface.right_edge(),
                wrap_mode: context.params.wrap_mode,
                word_wrap: *word_wrap,
                row_visibility_limit: context.row_visibility_limit,
                content_x: context.content_x,
                has_prefix: context.has_prefix,
                row_geometry_defaults: context.row_geometry_defaults,
                text_matrix_row_base: context.text_matrix_row_base,
                max_rows: context.max_rows,
                row_limit: context.row_limit,
            },
        )
        .render_if_needed_and_apply(
            context.text,
            BufferTextOverflowRenderState {
                byte_idx,
                charpos,
                col,
                source_render: source_render.reborrow(),
                row_extend,
                x,
                line_numbers,
                row_geometry,
                row_flags,
                hit_rows,
                hit_row_range,
                prefix_request,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
                face_scan,
                row_y_positions,
            },
        );
        if overflow_outcome.should_break() {
            return BufferTextSourceCharRenderOutcome::Stop;
        }
        if overflow_outcome.should_continue_buffer_walk() {
            return BufferTextSourceCharRenderOutcome::ContinueBufferWalk;
        }

        BufferDisplayPropertyTextModifierAction::clear_expired_raise_span(
            raise_span,
            *charpos,
            context.params.window_start,
        );

        {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
            context.overlay_context.render_before_at(
                buffer,
                *charpos,
                context.active_face_state,
                &mut overlay_state,
            );
        }

        prepared_append.capture_cursor_info_for_main_char_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            *x,
            decoded_source_char.start_byte_idx(),
            *col,
            ch == '\t',
            *charpos,
            context.point_charpos,
        );

        if prepared_append
            .append_to_text_row_and_apply(
                &buffer_row_append_context,
                &append_geometry,
                ch,
                &mut BufferTextSourceCharRenderState::new(
                    source_render.reborrow(),
                    trailing_whitespace,
                    word_wrap,
                    x,
                    col,
                    charpos,
                ),
            )
            .should_break()
        {
            return BufferTextSourceCharRenderOutcome::Stop;
        }

        {
            let mut overlay_state = OverlayStringRenderState::from_source_render(
                source_render.reborrow(),
                x,
                col,
                row_geometry,
                cursor_info,
                hit_rows,
                hit_row_range,
                row_y_positions,
                face_ids,
            );
            context.overlay_context.render_after_at(
                buffer,
                *charpos,
                context.active_face_state,
                &mut overlay_state,
            );
        }

        BufferTextSourceCharRenderOutcome::Rendered
    }
}

impl BufferTextOverflowRenderRequest {
    pub(crate) fn new(
        prepared_append: BufferTextSourceCharPreparedAppend,
        decoded_source_char: BufferTextDecodedSourceChar,
        context: BufferTextOverflowRenderContext,
    ) -> Self {
        Self {
            prepared_append,
            decoded_source_char,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply(
        self,
        text: &[u8],
        state: BufferTextOverflowRenderState<'_, '_>,
    ) -> BufferTextOverflowRenderOutcome {
        let BufferTextOverflowRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            face_scan,
            row_y_positions,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        match self.prepared_append.overflow_action(
            context.ch,
            context.right_edge_px,
            context.wrap_mode,
            context.word_wrap,
        ) {
            BufferTextSourceCharOverflowAction::Fits => BufferTextOverflowRenderOutcome::Fits,
            BufferTextSourceCharOverflowAction::Truncate { transition } => {
                let truncation_skip =
                    BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
                        text, byte_idx, charpos,
                    );
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    x,
                    context.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextOverflowRenderOutcome::Transition(
                    truncation_skip.transition_continuation(row_transition),
                )
            }
            BufferTextSourceCharOverflowAction::WordWrap {
                break_candidate: wrap_break,
                transition,
            } => {
                let word_wrap_action = BufferTextWordWrapSourceAction::new(wrap_break);
                word_wrap_action.apply_before_row_transition(
                    source_render.output_emitter(),
                    byte_idx,
                    charpos,
                    col,
                    row_extend,
                    x,
                    context.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                );
                BufferTextOverflowRenderOutcome::Transition(
                    word_wrap_action.apply_after_row_transition_and_prefix(
                        row_transition,
                        transition,
                        charpos,
                        hit_row_range,
                        face_scan,
                        row_geometry,
                        context.row_visibility_limit,
                        DisplayRowTransitionRenderState::new(
                            prefix_request,
                            context.has_prefix,
                            line_numbers,
                            hscroll_skip,
                            word_wrap,
                            trailing_whitespace,
                        ),
                    ),
                )
            }
            BufferTextSourceCharOverflowAction::CharacterWrap { transition } => {
                let character_wrap_action = BufferTextCharacterWrapSourceAction::from_decoded_char(
                    self.decoded_source_char,
                );
                character_wrap_action.apply_before_row_transition(row_extend, x, context.content_x);
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextOverflowRenderOutcome::Transition(
                    character_wrap_action.apply_after_visible_row_transition(
                        row_transition,
                        byte_idx,
                        charpos,
                        hit_row_range,
                        face_scan,
                        row_geometry,
                        context.row_visibility_limit,
                    ),
                )
            }
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferInvisibleTextScanAction {
    Unchecked,
    Visible { next_visible: i64 },
    Hidden(BufferInvisibleTextSkip),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferInvisibleTextSkip {
    start_byte_idx: usize,
    start_charpos: i64,
    skip_to: i64,
    next_visible: i64,
    point_in_hidden_region: bool,
    ellipsis: bool,
}

impl BufferInvisibleTextSkip {
    fn new(
        start_byte_idx: usize,
        start_charpos: i64,
        skip_to: i64,
        next_visible: i64,
        point_in_hidden_region: bool,
        ellipsis: bool,
    ) -> Self {
        Self {
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            ellipsis,
        }
    }

    #[cfg(test)]
    pub(crate) fn start_byte_idx(self) -> usize {
        self.start_byte_idx
    }

    #[cfg(test)]
    pub(crate) fn start_charpos(self) -> i64 {
        self.start_charpos
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    #[cfg(test)]
    pub(crate) fn next_visible(self) -> i64 {
        self.next_visible
    }

    #[cfg(test)]
    pub(crate) fn point_in_hidden_region(self) -> bool {
        self.point_in_hidden_region
    }

    #[cfg(test)]
    pub(crate) fn ellipsis(self) -> bool {
        self.ellipsis
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) {
        if !self.point_in_hidden_region {
            return;
        }
        capture_cursor_info(
            target,
            CapturedCursorInfo::from_active_face_state(
                active_face_state,
                CapturedCursorPlacement::from_row_text_position(
                    row_geometry.text_position(x, self.start_byte_idx, col),
                    CapturedCursorSlotWidth::FaceChar,
                    false,
                ),
            ),
        );
    }

    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<SyntheticTextAppendRequest> {
        self.ellipsis.then(|| {
            SyntheticTextAppendRequest::active_marker(
                position,
                SyntheticTextMarker::InvisibleEllipsis,
            )
        })
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferInvisibleTextRenderState<'_>,
    ) {
        let position = state.synthetic_text.position();
        self.capture_cursor_if_point(
            state.cursor_info,
            render_context.active_face,
            row_geometry,
            position.x_px,
            position.col,
        );

        let Some(request) = self.ellipsis_append_request(position) else {
            return;
        };
        state
            .synthetic_text
            .append_request_to_text_row(render_context, row_geometry, request);
    }
}

pub(crate) struct BufferInvisibleTextRenderState<'a> {
    synthetic_text: BufferSyntheticTextRenderState<'a>,
    cursor_info: &'a mut CursorCaptureState,
}

impl<'a> BufferInvisibleTextRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        cursor_info: &'a mut CursorCaptureState,
        x: &'a mut f32,
        col: &'a mut usize,
    ) -> Self {
        Self {
            synthetic_text: BufferSyntheticTextRenderState::new(source_render, x, col),
            cursor_info,
        }
    }
}

impl<'a> BufferInvisibleTextRenderRequest<'a> {
    pub(crate) fn new(context: BufferInvisibleTextRenderContext<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn render_at_checkpoint_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferInvisibleTextRenderRequestState<'_, '_>,
    ) -> BufferInvisibleTextRenderOutcome {
        let BufferInvisibleTextRenderRequestState {
            checkpoints,
            byte_idx,
            charpos,
            source_render,
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let action = BufferInvisibleTextScanContext::new(
            context.text,
            context.accessible_end,
            context.point_charpos,
            cursor_info.is_missing(),
        )
        .consume_at_checkpoint(buffer, checkpoints, byte_idx, charpos);
        let BufferInvisibleTextScanAction::Hidden(hidden_text) = action else {
            return BufferInvisibleTextRenderOutcome::Visible;
        };

        let mut hidden_text_state =
            BufferInvisibleTextRenderState::new(source_render.reborrow(), cursor_info, x, col);
        hidden_text.append_to_text_row_and_apply(
            BufferSyntheticTextRenderContext::new(
                context.append_surface,
                context.active_face_state,
                context.glyph_y_offset,
                context.char_h,
                context.default_face_ascent,
                context.char_w,
            ),
            row_geometry,
            &mut hidden_text_state,
        );

        let mut overlay_state = OverlayStringRenderState::from_source_render(
            source_render.reborrow(),
            x,
            col,
            row_geometry,
            cursor_info,
            hit_rows,
            hit_row_range,
            row_y_positions,
            face_ids,
        );
        context.overlay_context.render_after_at(
            buffer,
            *charpos,
            context.active_face_state,
            &mut overlay_state,
        );
        BufferInvisibleTextRenderOutcome::ContinueBufferWalk
    }
}

pub(crate) struct BufferSyntheticTextRenderState<'a> {
    source_render: TextRowSourceRenderState<'a>,
    x: &'a mut f32,
    col: &'a mut usize,
}

impl<'a> BufferSyntheticTextRenderState<'a> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'a>,
        x: &'a mut f32,
        col: &'a mut usize,
    ) -> Self {
        Self {
            source_render,
            x,
            col,
        }
    }

    fn position(&self) -> DisplayRowPosition {
        DisplayRowPosition {
            x_px: *self.x,
            col: *self.col,
        }
    }

    pub(crate) fn append_request_to_text_row<'ctx>(
        &mut self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        request: SyntheticTextAppendRequest,
    ) {
        let Some((_progress, position)) = render_context.render_request_to_text_row(
            &mut self.source_render,
            row_geometry,
            request,
        ) else {
            return;
        };
        *self.x = position.x_px;
        *self.col = position.col;
    }

    pub(crate) fn append_hscroll_truncation_marker_to_text_row<'ctx>(
        &mut self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        content_x: f32,
    ) {
        let request =
            render_context.hscroll_truncation_request(self.source_render.default_face(), content_x);
        self.append_request_to_text_row(render_context, row_geometry, request);
        self.source_render.mark_current_text_row_truncated_left();
    }
}

pub(crate) struct BufferCurrentFaceResolutionContext<'a, B: LayoutBufferView> {
    buffer: &'a B,
    face_resolver: &'a FaceResolver,
    measurement_policy: DisplayRowMeasurementPolicy,
    default_resolved: &'a ResolvedFace,
    default_face_char_w: f32,
    default_face_ascent: f32,
    default_face_h: f32,
    char_w: f32,
    char_h: f32,
    font_ascent: f32,
    window_system: bool,
}

impl<'a, B: LayoutBufferView> Clone for BufferCurrentFaceResolutionContext<'a, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, B: LayoutBufferView> Copy for BufferCurrentFaceResolutionContext<'a, B> {}

impl<'a, B: LayoutBufferView> BufferCurrentFaceResolutionContext<'a, B> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer: &'a B,
        face_resolver: &'a FaceResolver,
        measurement_policy: DisplayRowMeasurementPolicy,
        default_resolved: &'a ResolvedFace,
        default_face_char_w: f32,
        default_face_ascent: f32,
        default_face_h: f32,
        char_w: f32,
        char_h: f32,
        font_ascent: f32,
        window_system: bool,
    ) -> Self {
        Self {
            buffer,
            face_resolver,
            measurement_policy,
            default_resolved,
            default_face_char_w,
            default_face_ascent,
            default_face_h,
            char_w,
            char_h,
            font_ascent,
            window_system,
        }
    }

    pub(crate) fn resolve_at_checkpoint(
        &self,
        state: &mut BufferCurrentFaceResolutionState<'_, '_>,
        charpos: i64,
    ) -> bool {
        if !state.face_scan.should_resolve_at(charpos as usize) {
            return false;
        }

        let origin = DisplayOrigin::BufferText {
            charpos: neovm_core::buffer::CharPos0::new(charpos as usize),
        };
        let mut resolved = self.face_resolver.default_base_face_for_origin(
            Some(self.buffer),
            &origin,
            state.face_scan.next_check_mut(),
        );
        if let Some(factor) = state.height_span.value()
            && let Some(adjusted) = height_adjusted_face(
                &resolved,
                DisplayHeightFaceBasis {
                    canonical_face: self.default_resolved,
                    base_face: self.default_resolved,
                    fallback_char_width: self.default_face_char_w,
                    fallback_ascent: self.default_face_ascent,
                    fallback_row_height: self.default_face_h,
                },
                factor,
            )
        {
            resolved = adjusted;
        }

        let face_id = state.face_ids.allocate();
        let resolved_extend = resolved.extend;
        let resolved_bg = resolved.bg;
        let resolved_box_type = resolved.box_type;
        *state.active_face_state = state.source_render.resolve_and_install_measured_face(
            self.measurement_policy,
            face_id,
            resolved,
            self.window_system,
            self.char_w,
            DisplayRowFallbackMetrics::from_default_face_extents(
                self.char_w,
                self.char_h,
                self.font_ascent,
            ),
        );
        let face_metrics = state.active_face_state.metrics();
        state
            .row_geometry
            .include_row_extents(face_metrics.row_height, face_metrics.ascent);

        if resolved_extend {
            let ext_bg = Color::from_pixel(resolved_bg);
            state
                .row_extend
                .activate(state.row_geometry.current_row_marker(), (ext_bg, face_id));
        }

        if state.box_face.is_active() && resolved_box_type == 0 {
            state.box_face.clear();
        }
        if resolved_box_type > 0 {
            state
                .box_face
                .activate(state.row_geometry.current_row_marker(), state.x);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_at_checkpoint_with_source_state(
        &self,
        source_render: &mut TextRowSourceRenderState<'_>,
        face_scan: &mut FaceScanCheckpoint,
        height_span: &mut ActiveDisplayPropertySpan<f32>,
        face_ids: &mut FrameFaceIdAllocator,
        active_face_state: &mut DisplayRowActiveFaceState,
        row_geometry: &mut DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        x: f32,
        charpos: i64,
    ) -> bool {
        self.resolve_at_checkpoint(
            &mut BufferCurrentFaceResolutionState::new(
                source_render,
                face_scan,
                height_span,
                face_ids,
                active_face_state,
                row_geometry,
                row_extend,
                box_face,
                x,
            ),
            charpos,
        )
    }
}

pub(crate) struct BufferCurrentFaceResolutionState<'a, 'source> {
    source_render: &'a mut TextRowSourceRenderState<'source>,
    face_scan: &'a mut FaceScanCheckpoint,
    height_span: &'a ActiveDisplayPropertySpan<f32>,
    face_ids: &'a mut FrameFaceIdAllocator,
    active_face_state: &'a mut DisplayRowActiveFaceState,
    row_geometry: &'a mut DisplayRowGeometryState,
    row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
    box_face: &'a mut BoxFaceRowState,
    x: f32,
}

impl<'a, 'source> BufferCurrentFaceResolutionState<'a, 'source> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_render: &'a mut TextRowSourceRenderState<'source>,
        face_scan: &'a mut FaceScanCheckpoint,
        height_span: &'a ActiveDisplayPropertySpan<f32>,
        face_ids: &'a mut FrameFaceIdAllocator,
        active_face_state: &'a mut DisplayRowActiveFaceState,
        row_geometry: &'a mut DisplayRowGeometryState,
        row_extend: &'a mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &'a mut BoxFaceRowState,
        x: f32,
    ) -> Self {
        Self {
            source_render,
            face_scan,
            height_span,
            face_ids,
            active_face_state,
            row_geometry,
            row_extend,
            box_face,
            x,
        }
    }
}

pub(crate) struct BufferInvisibleTextScanContext<'a> {
    text: &'a [u8],
    accessible_end: i64,
    point_charpos: i64,
    cursor_missing: bool,
}

impl<'a> BufferInvisibleTextScanContext<'a> {
    pub(crate) fn new(
        text: &'a [u8],
        accessible_end: i64,
        point_charpos: i64,
        cursor_missing: bool,
    ) -> Self {
        Self {
            text,
            accessible_end,
            point_charpos,
            cursor_missing,
        }
    }

    pub(crate) fn consume_at_checkpoint<B: LayoutBufferView>(
        &self,
        buffer: &B,
        checkpoints: &mut TextPropertyScanCheckpoints,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferInvisibleTextScanAction {
        if !checkpoints.should_check_invisible(*charpos) {
            return BufferInvisibleTextScanAction::Unchecked;
        }

        let start_byte_idx = *byte_idx;
        let start_charpos = *charpos;
        let text_props = RustTextPropAccess::new(buffer);
        let (invisible, next_visible) = text_props.check_invisible(start_charpos);
        checkpoints.record_invisible_next(next_visible);

        if !invisible.hidden {
            return BufferInvisibleTextScanAction::Visible { next_visible };
        }

        let skip_to = next_visible.min(self.accessible_end);
        let point_in_hidden_region = self.cursor_missing
            && self.point_charpos >= start_charpos
            && self.point_charpos < skip_to;
        skip_text_to_charpos(self.text, byte_idx, charpos, skip_to);

        BufferInvisibleTextScanAction::Hidden(BufferInvisibleTextSkip::new(
            start_byte_idx,
            start_charpos,
            skip_to,
            next_visible,
            point_in_hidden_region,
            invisible.ellipsis,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferSelectiveDisplayLineTailAction {
    Exhausted,
    LineBreak { charpos: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayLineTailMarker;

impl BufferSelectiveDisplayLineTailMarker {
    pub(crate) fn ellipsis_append_request(
        self,
        position: DisplayRowPosition,
    ) -> SyntheticTextAppendRequest {
        SyntheticTextAppendRequest::active_marker(position, SyntheticTextMarker::SelectiveEllipsis)
    }

    pub(crate) fn append_to_text_row_and_apply<'ctx>(
        self,
        render_context: BufferSyntheticTextRenderContext<'ctx>,
        row_geometry: &'ctx DisplayRowGeometryState,
        state: &mut BufferSyntheticTextRenderState<'_>,
    ) {
        let request = self.ellipsis_append_request(state.position());
        state.append_request_to_text_row(render_context, row_geometry, request);
    }
}

impl BufferSelectiveDisplayLineTailAction {
    pub(crate) fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak { .. })
    }

    pub(crate) fn apply_hidden_line_break_row_state(
        self,
        row_geometry: &DisplayRowGeometryState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
        x: &mut f32,
    ) {
        if self.is_line_break() {
            *x = content_x;
            row_extend.clear();
            box_face.continue_on_row(row_geometry.next_row_marker(), content_x);
        }
    }

    pub(crate) fn sync_after_hidden_line_break_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn apply_after_hidden_line_break_transition(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_hidden_line_break_transition(synced_charpos, charpos, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> Option<i64> {
        match self {
            Self::LineBreak { charpos } => Some(charpos),
            Self::Exhausted => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayHiddenLines {
    hidden_line_count: usize,
}

impl BufferSelectiveDisplayHiddenLines {
    fn new(hidden_line_count: usize) -> Self {
        Self { hidden_line_count }
    }

    #[cfg(test)]
    pub(crate) fn hidden_line_count(self) -> usize {
        self.hidden_line_count
    }

    pub(crate) fn apply_to_line_numbers(self, line_numbers: &mut LineNumberRenderState) {
        for _ in 0..self.hidden_line_count {
            line_numbers.advance_hidden_line();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferSelectiveDisplayContext<'a> {
    text: &'a [u8],
    selective_display: i32,
    tab_width: i32,
}

impl<'a> BufferSelectiveDisplayContext<'a> {
    pub(crate) fn new(text: &'a [u8], selective_display: i32, tab_width: i32) -> Self {
        Self {
            text,
            selective_display,
            tab_width: tab_width.max(1),
        }
    }

    pub(crate) fn hides_carriage_return_tail(self, ch: char) -> bool {
        self.selective_display > 0 && ch == '\r'
    }

    pub(crate) fn carriage_return_tail_marker(
        self,
        ch: char,
    ) -> Option<BufferSelectiveDisplayLineTailMarker> {
        self.hides_carriage_return_tail(ch)
            .then_some(BufferSelectiveDisplayLineTailMarker)
    }

    pub(crate) fn hides_indented_lines_after_line_break(self, byte_idx: usize) -> bool {
        self.selective_display > 0
            && self.selective_display < i32::MAX
            && byte_idx < self.text.len()
    }

    pub(crate) fn skip_rest_of_line_after_carriage_return(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferSelectiveDisplayLineTailAction {
        *charpos += 1;
        while *byte_idx < self.text.len() {
            let (skip_ch, skip_len) = decode_utf8(&self.text[*byte_idx..]);
            if skip_len == 0 {
                break;
            }
            *byte_idx += skip_len;
            *charpos += 1;
            if skip_ch == '\n' {
                return BufferSelectiveDisplayLineTailAction::LineBreak { charpos: *charpos };
            }
        }

        BufferSelectiveDisplayLineTailAction::Exhausted
    }

    pub(crate) fn skip_hidden_indented_lines_after_line_break(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> BufferSelectiveDisplayHiddenLines {
        let mut hidden_line_count = 0;
        while *byte_idx < self.text.len() {
            let Some(indent) = self.indentation_columns_at(*byte_idx) else {
                break;
            };
            if indent <= self.selective_display {
                break;
            }

            if self.skip_line(byte_idx, charpos) {
                hidden_line_count += 1;
            }
        }

        BufferSelectiveDisplayHiddenLines::new(hidden_line_count)
    }

    pub(crate) fn apply_hidden_indented_lines_after_line_break(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        line_numbers: &mut LineNumberRenderState,
    ) -> BufferSelectiveDisplayHiddenLines {
        if !self.hides_indented_lines_after_line_break(*byte_idx) {
            return BufferSelectiveDisplayHiddenLines::new(0);
        }
        let hidden_lines = self.skip_hidden_indented_lines_after_line_break(byte_idx, charpos);
        hidden_lines.apply_to_line_numbers(line_numbers);
        hidden_lines
    }

    fn indentation_columns_at(self, mut byte_idx: usize) -> Option<i32> {
        if byte_idx >= self.text.len() {
            return None;
        }

        let mut indent = 0i32;
        while byte_idx < self.text.len() {
            match self.text[byte_idx] {
                b' ' => {
                    indent += 1;
                    byte_idx += 1;
                }
                b'\t' => {
                    indent = ((indent / self.tab_width) + 1) * self.tab_width;
                    byte_idx += 1;
                }
                _ => break,
            }
        }
        Some(indent)
    }

    fn skip_line(self, byte_idx: &mut usize, charpos: &mut i64) -> bool {
        while *byte_idx < self.text.len() {
            let (skip_ch, skip_len) = decode_utf8(&self.text[*byte_idx..]);
            if skip_len == 0 {
                break;
            }
            *byte_idx += skip_len;
            *charpos += 1;
            if skip_ch == '\n' {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextLineBreakSourceAction {
    ch_start_byte_idx: usize,
    charpos: i64,
    next_charpos: i64,
    line_spacing: f32,
}

pub(crate) struct BufferTextLineBreakRenderRequest<'a> {
    source_event: BufferTextLineBreakSourceEvent,
    context: BufferTextLineBreakRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextLineBreakRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) selective_display: i32,
    pub(crate) tab_width: i32,
    pub(crate) active_face_state: &'a DisplayRowActiveFaceState,
    pub(crate) point_charpos: i64,
    pub(crate) char_h: f32,
    pub(crate) extra_line_spacing: f32,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

impl<'a> BufferTextLineBreakRenderRequest<'a> {
    #[cfg(test)]
    pub(crate) fn new(
        source_char: BufferTextDecodedSourceChar,
        context: BufferTextLineBreakRenderContext<'a>,
    ) -> Self {
        Self::from_source_event(BufferTextLineBreakSourceEvent::new(source_char), context)
    }

    pub(crate) fn from_source_event(
        source_event: BufferTextLineBreakSourceEvent,
        context: BufferTextLineBreakRenderContext<'a>,
    ) -> Self {
        Self {
            source_event,
            context,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextLineBreakRenderState<'_, '_>,
    ) -> DisplayRowTransitionContinuation {
        let BufferTextLineBreakRenderState {
            byte_idx,
            charpos,
            cursor_info,
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render,
            x,
            col,
            prefix_request,
            line_numbers,
            hscroll_skip,
            word_wrap,
            row_flags,
            hit_rows,
            hit_row_range,
            row_y_positions,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        let line_break_action = BufferTextLineBreakSourceAction::for_decoded_newline(
            buffer,
            self.source_event.decoded_char(),
            context.char_h,
            context.extra_line_spacing,
        );
        line_break_action.capture_cursor_if_point(
            cursor_info,
            context.active_face_state,
            row_geometry,
            context.point_charpos,
            *x,
            *col,
        );
        line_break_action.apply_before_row_transition(
            row_geometry,
            trailing_whitespace,
            row_extend,
            box_face,
            source_render.output_emitter(),
            context.content_x,
            x,
            charpos,
        );

        let line_break_transition = DisplayRowLineBreakTransitionPlan::line_break();
        let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
            context.row_geometry_defaults,
            context.text_matrix_row_base,
            row_y_positions,
            context.max_rows,
            row_geometry,
            row_flags,
            context.row_limit,
            hit_rows,
            &mut source_render,
        )
        .emit_line_break_then_row_start(
            line_break_transition,
            hit_row_range.range_to(*charpos),
            DisplayRowPosition {
                x_px: *x,
                col: *col,
            },
            line_break_action.line_spacing(),
            DisplayRowTransitionRenderState::new(
                prefix_request,
                context.has_prefix,
                line_numbers,
                hscroll_skip,
                word_wrap,
                trailing_whitespace,
            ),
            col,
        );

        let synced_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                context.text_start_byte + *byte_idx,
            ))
            .get() as i64;
        let continuation = line_break_action.apply_after_line_break_row_transition(
            row_transition,
            synced_charpos,
            charpos,
            hit_row_range,
            row_geometry,
            box_face,
            context.content_x,
        );
        if continuation.should_break() {
            return continuation;
        }

        BufferSelectiveDisplayContext::new(
            context.text,
            context.selective_display,
            context.tab_width,
        )
        .apply_hidden_indented_lines_after_line_break(byte_idx, charpos, line_numbers);
        DisplayRowTransitionContinuation::Continue
    }
}

impl BufferTextLineBreakSourceAction {
    pub(crate) fn for_newline<B: LayoutBufferView>(
        buffer: &B,
        charpos: i64,
        ch_start_byte_idx: usize,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        let text_prop_spacing = RustTextPropAccess::new(buffer).check_line_spacing(charpos, char_h);
        let line_spacing = if text_prop_spacing > 0.0 {
            text_prop_spacing
        } else if extra_line_spacing > 0.0 {
            extra_line_spacing
        } else {
            0.0
        };
        Self {
            ch_start_byte_idx,
            charpos,
            next_charpos: charpos + 1,
            line_spacing,
        }
    }

    pub(crate) fn for_decoded_newline<B: LayoutBufferView>(
        buffer: &B,
        source_char: BufferTextDecodedSourceChar,
        char_h: f32,
        extra_line_spacing: f32,
    ) -> Self {
        Self::for_newline(
            buffer,
            source_char.start_charpos(),
            source_char.start_byte_idx(),
            char_h,
            extra_line_spacing,
        )
    }

    pub(crate) fn point_matches(self, point_charpos: i64) -> bool {
        point_charpos == self.charpos
    }

    pub(crate) fn next_charpos(self) -> i64 {
        self.next_charpos
    }

    pub(crate) fn line_spacing(self) -> f32 {
        self.line_spacing
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        trailing_whitespace: &mut TrailingWhitespaceRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        box_face: &mut BoxFaceRowState,
        output_emitter: &mut WindowOutputEmitter,
        content_x: f32,
        x: &mut f32,
        charpos: &mut i64,
    ) {
        trailing_whitespace.reset_after_row_transition();
        row_extend.clear();
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
        *charpos = self.next_charpos();
        *x = content_x;
        output_emitter.note_display_buffer_pos(LispCharPos1::new(*charpos));
    }

    pub(crate) fn apply_after_row_transition(
        self,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) {
        box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
    }

    pub(crate) fn apply_after_line_break_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        row_geometry: &DisplayRowGeometryState,
        box_face: &mut BoxFaceRowState,
        content_x: f32,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, charpos, hit_row_range);
        self.apply_after_row_transition(row_geometry, box_face, content_x);
        DisplayRowTransitionContinuation::Continue
    }

    pub(crate) fn sync_after_row_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        x: f32,
        col: usize,
    ) -> CapturedCursorInfo {
        CapturedCursorInfo::from_active_face_state(
            active_face_state,
            CapturedCursorPlacement::from_row_text_position(
                row_geometry.text_position(x, self.ch_start_byte_idx, col),
                CapturedCursorSlotWidth::FaceChar,
                false,
            ),
        )
    }

    pub(crate) fn capture_cursor_if_point(
        self,
        target: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        x: f32,
        col: usize,
    ) {
        if !target.is_missing() || !self.point_matches(point_charpos) {
            return;
        }
        capture_cursor_info(
            target,
            self.cursor_info(active_face_state, row_geometry, x, col),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextTruncationSkipAction {
    charpos: i64,
    reached_line_break: bool,
}

impl BufferTextTruncationSkipAction {
    pub(crate) fn consume_decoded_char_and_rest_of_line(
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) -> Self {
        *charpos += 1;
        let reached_line_break = skip_to_newline(text, byte_idx, charpos);
        Self {
            charpos: *charpos,
            reached_line_break,
        }
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
    }

    pub(crate) fn reached_line_break(self) -> bool {
        self.reached_line_break
    }

    pub(crate) fn apply_before_row_transition(
        self,
        line_numbers: &mut LineNumberRenderState,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        if self.reached_line_break() {
            line_numbers.advance_line();
        }
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn sync_after_row_transition(
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) {
        *charpos = synced_charpos;
        hit_row_range.advance_to(*charpos);
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: TextMatrixRowTransition,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            DisplayRowTransitionContinuation::Exhausted
        } else {
            DisplayRowTransitionContinuation::Continue
        }
    }

    pub(crate) fn sync_after_row_transition_if_visible(
        self,
        row_transition: TextMatrixRowTransition,
        synced_charpos: i64,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        Self::sync_after_row_transition(synced_charpos, charpos, hit_row_range);
        DisplayRowTransitionContinuation::Continue
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWordWrapSourceAction {
    break_candidate: WordWrapBreakCandidate,
}

impl BufferTextWordWrapSourceAction {
    pub(crate) fn new(break_candidate: WordWrapBreakCandidate) -> Self {
        Self { break_candidate }
    }

    pub(crate) fn restore_row_output_progress(self, output_emitter: &mut WindowOutputEmitter) {
        output_emitter.truncate_display_points(self.break_candidate.display_point_count());
        let (row_first_display_pos, row_last_display_pos) =
            self.break_candidate.row_display_positions();
        output_emitter
            .restore_current_row_display_positions(row_first_display_pos, row_last_display_pos);
    }

    pub(crate) fn rewind_source_state(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        col: &mut usize,
    ) {
        *byte_idx = self.break_candidate.byte_idx();
        *charpos = self.break_candidate.charpos();
        *col = 0;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        output_emitter: &mut WindowOutputEmitter,
        byte_idx: &mut usize,
        charpos: &mut i64,
        col: &mut usize,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        self.restore_row_output_progress(output_emitter);
        self.rewind_source_state(byte_idx, charpos, col);
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        *charpos = self.charpos();
        hit_row_range.advance_to(*charpos);
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_row_transition_and_prefix(
        self,
        row_transition: TextMatrixRowTransition,
        transition: DisplayRowOverflowTransitionPlan,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
        render_state: DisplayRowTransitionRenderState<'_>,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(charpos, hit_row_range, face_scan);
        render_state.apply_overflow_prefix(transition);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    pub(crate) fn charpos(self) -> i64 {
        self.break_candidate.charpos()
    }

    #[cfg(test)]
    pub(crate) fn byte_idx(self) -> usize {
        self.break_candidate.byte_idx()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSpecialWrapSourceAction {
    charpos: i64,
}

impl BufferTextSpecialWrapSourceAction {
    pub(crate) fn new(charpos: i64) -> Self {
        Self { charpos }
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn hit_range_and_advance(
        self,
        hit_row_range: &mut HitRowRangeTracker,
    ) -> DisplayRowHitRange {
        let hit_range = hit_row_range.range_to(self.charpos);
        hit_row_range.advance_to(self.charpos);
        hit_range
    }

    pub(crate) fn transition_continuation(
        self,
        row_transition: TextMatrixRowTransition,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
        )
    }

    #[cfg(test)]
    pub(crate) fn charpos(self) -> i64 {
        self.charpos
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextCharacterWrapSourceAction {
    ch_start_byte_idx: usize,
    ch_start_charpos: i64,
}

impl BufferTextCharacterWrapSourceAction {
    pub(crate) fn new(ch_start_byte_idx: usize, ch_start_charpos: i64) -> Self {
        Self {
            ch_start_byte_idx,
            ch_start_charpos,
        }
    }

    pub(crate) fn from_decoded_char(source_char: BufferTextDecodedSourceChar) -> Self {
        Self::new(source_char.start_byte_idx(), source_char.start_charpos())
    }

    pub(crate) fn rewind_source_state(self, byte_idx: &mut usize, charpos: &mut i64) {
        *byte_idx = self.ch_start_byte_idx;
        *charpos = self.ch_start_charpos;
    }

    pub(crate) fn apply_before_row_transition(
        self,
        row_extend: &mut DisplayRowScopedValue<(Color, u32)>,
        x: &mut f32,
        content_x: f32,
    ) {
        *x = content_x;
        row_extend.clear();
    }

    pub(crate) fn apply_after_row_transition(
        self,
        byte_idx: &mut usize,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
    ) {
        self.rewind_source_state(byte_idx, charpos);
        hit_row_range.advance_to(*charpos);
        face_scan.invalidate();
    }

    pub(crate) fn apply_after_visible_row_transition(
        self,
        row_transition: TextMatrixRowTransition,
        byte_idx: &mut usize,
        charpos: &mut i64,
        hit_row_range: &mut HitRowRangeTracker,
        face_scan: &mut FaceScanCheckpoint,
        row_geometry: &DisplayRowGeometryState,
        row_visibility_limit: DisplayRowVisibilityLimit,
    ) -> DisplayRowTransitionContinuation {
        if row_transition.is_exhausted() {
            return DisplayRowTransitionContinuation::Exhausted;
        }
        self.apply_after_row_transition(byte_idx, charpos, hit_row_range, face_scan);
        DisplayRowTransitionContinuation::after_visible_row_transition(
            row_transition,
            row_geometry,
            row_visibility_limit,
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

pub(crate) struct BufferTextSpecialOverflowRenderRequest<'a> {
    prepared_append: &'a BufferTextSpecialSourceCharPreparedAppend,
    context: BufferTextSpecialOverflowRenderContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferTextSpecialOverflowRenderContext<'a> {
    pub(crate) text: &'a [u8],
    pub(crate) text_start_byte: usize,
    pub(crate) x_px: f32,
    pub(crate) right_edge_px: f32,
    pub(crate) wrap_mode: LineWrapMode,
    pub(crate) row_visibility_limit: DisplayRowVisibilityLimit,
    pub(crate) content_x: f32,
    pub(crate) has_prefix: bool,
    pub(crate) row_geometry_defaults: DisplayRowGeometryDefaults,
    pub(crate) text_matrix_row_base: usize,
    pub(crate) max_rows: usize,
    pub(crate) row_limit: DisplayRowLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferTextSpecialOverflowRenderOutcome {
    Fits,
    AppendPrepared(DisplayRowTransitionContinuation),
    ContinueBufferWalk(DisplayRowTransitionContinuation),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BufferTextSpecialSourceCharOverflowAction {
    Fits,
    Truncate {
        transition: DisplayRowOverflowTransitionPlan,
    },
    Wrap {
        transition: DisplayRowOverflowTransitionPlan,
    },
}

impl BufferTextSpecialSourceCharOverflowAction {
    fn for_decision(decision: SpecialTextRowOverflowDecision) -> Self {
        match decision {
            SpecialTextRowOverflowDecision::Fits => Self::Fits,
            SpecialTextRowOverflowDecision::Truncate => Self::Truncate {
                transition: DisplayRowOverflowTransitionPlan::truncation(
                    TextRowTransitionStatePolicy::special_truncation(),
                ),
            },
            SpecialTextRowOverflowDecision::Wrap => Self::Wrap {
                transition: DisplayRowOverflowTransitionPlan::visual_wrap(
                    TextRowTransitionStatePolicy::special_visual_wrap(),
                ),
            },
        }
    }
}

impl BufferTextSpecialOverflowRenderOutcome {
    pub(crate) fn should_break(self) -> bool {
        matches!(
            self,
            Self::AppendPrepared(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            ) | Self::ContinueBufferWalk(
                DisplayRowTransitionContinuation::Exhausted
                    | DisplayRowTransitionContinuation::Hidden
            )
        )
    }

    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(
            self,
            Self::ContinueBufferWalk(DisplayRowTransitionContinuation::Continue)
        )
    }
}

impl<'a> BufferTextSpecialOverflowRenderRequest<'a> {
    pub(crate) fn new(
        prepared_append: &'a BufferTextSpecialSourceCharPreparedAppend,
        context: BufferTextSpecialOverflowRenderContext<'a>,
    ) -> Self {
        Self {
            prepared_append,
            context,
        }
    }

    pub(crate) fn render_if_needed_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferTextSpecialOverflowRenderState<'_, '_>,
    ) -> BufferTextSpecialOverflowRenderOutcome {
        let BufferTextSpecialOverflowRenderState {
            byte_idx,
            charpos,
            col,
            source_render,
            row_extend,
            x,
            line_numbers,
            row_geometry,
            row_flags,
            hit_rows,
            hit_row_range,
            prefix_request,
            hscroll_skip,
            word_wrap,
            trailing_whitespace,
            row_y_positions,
        } = state;
        let mut source_render = source_render;
        let context = self.context;

        match self.prepared_append.overflow_action(
            context.x_px,
            context.right_edge_px,
            context.wrap_mode,
        ) {
            None | Some(BufferTextSpecialSourceCharOverflowAction::Fits) => {
                BufferTextSpecialOverflowRenderOutcome::Fits
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Truncate { transition }) => {
                let truncation_skip =
                    BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
                        context.text,
                        byte_idx,
                        charpos,
                    );
                truncation_skip.apply_before_row_transition(
                    line_numbers,
                    row_extend,
                    x,
                    context.content_x,
                );
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_row_range.range_to(*charpos),
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                let synced_charpos = buffer
                    .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(
                        context.text_start_byte + *byte_idx,
                    ))
                    .get() as i64;
                BufferTextSpecialOverflowRenderOutcome::ContinueBufferWalk(
                    truncation_skip.sync_after_row_transition_if_visible(
                        row_transition,
                        synced_charpos,
                        charpos,
                        hit_row_range,
                    ),
                )
            }
            Some(BufferTextSpecialSourceCharOverflowAction::Wrap { transition }) => {
                let special_wrap_action = BufferTextSpecialWrapSourceAction::new(*charpos);
                special_wrap_action.apply_before_row_transition(row_extend, x, context.content_x);
                let hit_range = special_wrap_action.hit_range_and_advance(hit_row_range);
                let row_transition = DisplayRowTextWindowEmitContext::from_source_render(
                    context.row_geometry_defaults,
                    context.text_matrix_row_base,
                    row_y_positions,
                    context.max_rows,
                    row_geometry,
                    row_flags,
                    context.row_limit,
                    hit_rows,
                    &mut source_render,
                )
                .emit_overflow_then_row_start(
                    transition,
                    hit_range,
                    DisplayRowPosition {
                        x_px: *x,
                        col: *col,
                    },
                    DisplayRowTransitionRenderState::new(
                        prefix_request,
                        context.has_prefix,
                        line_numbers,
                        hscroll_skip,
                        word_wrap,
                        trailing_whitespace,
                    ),
                    col,
                );
                BufferTextSpecialOverflowRenderOutcome::AppendPrepared(
                    special_wrap_action.transition_continuation(
                        row_transition,
                        row_geometry,
                        context.row_visibility_limit,
                    ),
                )
            }
        }
    }
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

pub(crate) struct DisplayReplacementStringItemMeasurer {
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayRowRenderPolicy for DisplayReplacementStringItemMeasurer {
    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        _face_id: u32,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        let DisplayItemKind::SourceMappedText(text) = &item.kind else {
            return DisplayRowItemMeasurement::Default;
        };
        DisplayRowItemMeasurement::TextRun(
            self.active_face_state
                .text_run_measurement(font_metrics, text.text.as_ref()),
        )
    }
}

struct DisplayReplacementStringRenderPolicy<'a, M> {
    item_policy: &'a mut M,
}

impl<M: DisplayRowRenderPolicy> DisplayRowRenderPolicy
    for DisplayReplacementStringRenderPolicy<'_, M>
{
    fn stop_before_item(&mut self, item: &DisplayItem) -> bool {
        matches!(item.kind, DisplayItemKind::RowBreak(_))
    }

    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        face_id: u32,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        self.item_policy
            .measurement_for(item, face_id, font_metrics)
    }

    fn clipped_behavior(&mut self, item: &DisplayItem) -> DisplayRowRenderClipBehavior {
        if matches!(item.kind, DisplayItemKind::SourceMappedText(_)) {
            DisplayRowRenderClipBehavior::Stop
        } else {
            DisplayRowRenderClipBehavior::Continue
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DisplayReplacementStringSourceAppendRequest {
    position: DisplayRowPosition,
    source: BufferDisplayReplacementStringRequest,
}

impl DisplayReplacementStringSourceAppendRequest {
    fn new(position: DisplayRowPosition, source: BufferDisplayReplacementStringRequest) -> Self {
        Self { position, source }
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    #[cfg(test)]
    fn source_id(self) -> LispStringSourceId {
        LispStringSourceId(self.source.source_id())
    }

    #[cfg(test)]
    fn value(self) -> Value {
        self.source.value()
    }

    fn render_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_context: &DisplayReplacementAppendContext<'_>,
        item_policy: &mut impl DisplayRowRenderPolicy,
    ) -> DisplayRowPosition {
        let position = self.position();
        let Some(source) = self.source.into_source(append_context.face_id) else {
            return position;
        };
        let mut render_policy = DisplayReplacementStringRenderPolicy { item_policy };
        let Some(outcome) = DisplayRowSourceAppendOperation::new(
            append_context.base_face,
            append_context.face_id,
            append_context.frame.clone(),
            position,
            DisplayRowAppendKind::DisplayReplacementString,
        )
        .render_source_to_text_row_and_emit(state, source, face_ids, &mut render_policy) else {
            return position;
        };
        outcome.end_position()
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementStringAppendRequest {
    item: DisplayReplacementStringSourceItem,
    replacement_base_face: Option<DisplayStringBaseFace>,
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayReplacementStringAppendRequest {
    fn new(
        item: DisplayReplacementStringSourceItem,
        replacement_base_face: Option<DisplayStringBaseFace>,
        active_face_state: DisplayRowActiveFaceState,
    ) -> Self {
        Self {
            item,
            replacement_base_face,
            active_face_state,
        }
    }

    #[cfg(test)]
    fn origin(&self) -> DisplayOrigin {
        self.item.origin()
    }

    #[cfg(test)]
    fn base_face_policy(&self) -> BaseFacePolicy {
        self.item.base_face_policy()
    }

    fn string_item_measurer(&self) -> DisplayReplacementStringItemMeasurer {
        DisplayReplacementStringItemMeasurer {
            active_face_state: self.active_face_state.clone(),
        }
    }

    fn source_append_request(
        &self,
        replacement_source: BufferDisplayReplacementSource,
        position: DisplayRowPosition,
    ) -> DisplayReplacementStringSourceAppendRequest {
        DisplayReplacementStringSourceAppendRequest::new(
            position,
            BufferDisplayReplacementStringRequest::new(
                self.item.source_id(),
                self.item.value(),
                replacement_source,
            ),
        )
    }

    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        if self.item.is_empty() {
            return position;
        }
        let Some(ref replacement_base_face) = self.replacement_base_face else {
            debug_assert!(false, "display string replacement missing base face");
            return position;
        };
        let source_request =
            self.source_append_request(replacement_append_context.replacement_source, position);
        let mut item_policy = self.string_item_measurer();
        let append_context = replacement_append_context.full_text_width_active_face(
            replacement_base_face.face_id(),
            replacement_base_face.face(),
        );
        source_request.render_to_text_row_and_emit(
            state,
            face_ids,
            &append_context,
            &mut item_policy,
        )
    }
}

#[derive(Clone, Debug)]
struct DisplayReplacementItemAppendRequest {
    item: DisplayReplacementAppendItem,
    frame: DisplayReplacementItemAppendFrame,
    position: DisplayRowPosition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DisplayReplacementItemAppendFrame {
    ActiveFace,
    DisplayBox { height_px: f32, ascent_px: f32 },
}

impl DisplayReplacementItemAppendRequest {
    fn active_face(item: DisplayReplacementAppendItem, position: DisplayRowPosition) -> Self {
        Self {
            item,
            frame: DisplayReplacementItemAppendFrame::ActiveFace,
            position,
        }
    }

    fn display_box(
        item: DisplayReplacementAppendItem,
        height_px: f32,
        ascent_px: f32,
        position: DisplayRowPosition,
    ) -> Self {
        Self {
            item,
            frame: DisplayReplacementItemAppendFrame::DisplayBox {
                height_px,
                ascent_px,
            },
            position,
        }
    }
}

impl DisplayReplacementStretchSourceItem {
    fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<DisplayReplacementItemAppendRequest> {
        (self.width_px() > 0.0).then(|| {
            DisplayReplacementItemAppendRequest::active_face(
                DisplayReplacementAppendItem::stretch(self.geometry()),
                position,
            )
        })
    }
}

pub(crate) enum BufferDisplayPropertyTextAppendAction {
    Replacement(BufferDisplayPropertyTextReplacementOutcome),
    Modifiers(BufferDisplayPropertyTextModifierAction),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferDisplayPropertyTextWalkOutcome {
    Continue,
    ReplacementConsumed,
    FaceStateChanged,
}

pub(crate) struct BufferDisplayPropertyTextAppendRequest<'a> {
    source_event: BufferDisplayPropertyTextSourceEvent<'a>,
    context: BufferDisplayPropertyTextAppendContext<'a>,
}

#[derive(Clone, Copy)]
pub(crate) struct BufferDisplayPropertyTextAppendContext<'a> {
    buffer_id: BufferId,
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

pub(crate) struct DisplayPropertyReplacementSourceResolveRequest<'a, 'source> {
    display_property: &'a DisplayPropertyClassification,
    source_event: BufferDisplayPropertyTextSourceEvent<'source>,
    active_face_state: &'a DisplayRowActiveFaceState,
    font_metrics: &'a mut Option<FontMetricsService>,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    display_host: Option<&'a dyn DisplayHost>,
}

pub(crate) struct BufferDisplayPropertyTextRenderContext<'a> {
    buffer_id: BufferId,
    text_start_byte: usize,
    text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderRequest<'a, B: LayoutBufferView> {
    context: BufferDisplayPropertyCheckpointRenderContext<'a, B>,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderContext<'a, B: LayoutBufferView> {
    pub(crate) face_resolution_context: BufferCurrentFaceResolutionContext<'a, B>,
    pub(crate) buffer_id: BufferId,
    pub(crate) text_start_byte: usize,
    pub(crate) text: &'a [u8],
    pub(crate) current_x: f32,
    pub(crate) content_x: f32,
    pub(crate) params: &'a WindowParams,
    pub(crate) glyph_y_offset: f32,
    pub(crate) default_row_height: f32,
    pub(crate) start_position: DisplayRowPosition,
    pub(crate) charpos: i64,
    pub(crate) byte_idx: usize,
    pub(crate) accessible_end: i64,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderState<'a, 'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
    pub(crate) append_surface: &'a DisplayRowAppendSurface,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) checkpoints: &'emit mut TextPropertyScanCheckpoints,
    pub(crate) face_scan: &'emit mut FaceScanCheckpoint,
    pub(crate) active_face_state: &'emit mut DisplayRowActiveFaceState,
    pub(crate) row_extend: &'emit mut DisplayRowScopedValue<(Color, u32)>,
    pub(crate) box_face: &'emit mut BoxFaceRowState,
    pub(crate) byte_idx: &'emit mut usize,
    pub(crate) charpos: &'emit mut i64,
    pub(crate) x: &'emit mut f32,
    pub(crate) col: &'emit mut usize,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) raise_span: &'emit mut ActiveDisplayPropertySpan<f32>,
    pub(crate) height_span: &'emit mut ActiveDisplayPropertySpan<f32>,
    pub(crate) point_charpos: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementOutcome {
    replacement: DisplayPropertyReplacementAppendOutcome,
    skip_to: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferDisplayPropertyTextModifierStateOutcome {
    height_face_changed: bool,
}

impl BufferDisplayPropertyTextModifierStateOutcome {
    fn new(height_face_changed: bool) -> Self {
        Self {
            height_face_changed,
        }
    }

    pub(crate) fn height_face_changed(self) -> bool {
        self.height_face_changed
    }
}

impl BufferDisplayPropertyTextWalkOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        matches!(self, Self::ReplacementConsumed)
    }

    pub(crate) fn should_resolve_face(self) -> bool {
        matches!(self, Self::FaceStateChanged)
    }
}

impl BufferDisplayPropertyTextAppendAction {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_to_buffer_walk_state(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        x: &mut f32,
        col: &mut usize,
        cursor_info: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        raise_span: &mut ActiveDisplayPropertySpan<f32>,
        height_span: &mut ActiveDisplayPropertySpan<f32>,
        face_scan: &mut FaceScanCheckpoint,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        match self {
            Self::Replacement(replacement_outcome) => {
                replacement_outcome.capture_cursor_info_if_point(
                    cursor_info,
                    active_face_state,
                    row_geometry,
                    point_charpos,
                    *charpos,
                    *byte_idx,
                );
                replacement_outcome.apply_to_walk_state(text, byte_idx, charpos, x, col);
                BufferDisplayPropertyTextWalkOutcome::ReplacementConsumed
            }
            Self::Modifiers(modifiers) => {
                if modifiers
                    .apply_to_walk_state(raise_span, height_span, face_scan)
                    .height_face_changed()
                {
                    BufferDisplayPropertyTextWalkOutcome::FaceStateChanged
                } else {
                    BufferDisplayPropertyTextWalkOutcome::Continue
                }
            }
            Self::None => BufferDisplayPropertyTextWalkOutcome::Continue,
        }
    }
}

impl<'a, 'source> DisplayPropertyReplacementSourceResolveRequest<'a, 'source> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        display_property: &'a DisplayPropertyClassification,
        source_event: BufferDisplayPropertyTextSourceEvent<'source>,
        active_face_state: &'a DisplayRowActiveFaceState,
        font_metrics: &'a mut Option<FontMetricsService>,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        display_host: Option<&'a dyn DisplayHost>,
    ) -> Self {
        Self {
            display_property,
            source_event,
            active_face_state,
            font_metrics,
            current_x,
            content_x,
            params,
            display_host,
        }
    }

    fn face_metrics(&self) -> DisplayRowMeasuredFaceMetrics {
        self.active_face_state.metrics()
    }

    pub(crate) fn resolve(self) -> Option<DisplayPropertyReplacementSourceItem> {
        let display_property = self.display_property;
        let source_event = self.source_event;
        let face_metrics = self.face_metrics();
        let source_metrics = DisplayPropertyReplacementSourceMetrics::new(
            face_metrics.char_width,
            face_metrics.row_height,
            face_metrics.ascent,
        );
        let source_inputs = match display_property.replacement()? {
            DisplayReplacementProperty::String => {
                let replacement = self.source_event.value().as_utf8_str()?;
                let cursor_slot_width_px = replacement
                    .chars()
                    .next()
                    .map(|ch| {
                        self.active_face_state.advance_for_char(
                            self.font_metrics,
                            ch,
                            face_metrics.char_width,
                        )
                    })
                    .unwrap_or_else(|| face_metrics.char_width.max(1.0));
                DisplayPropertyReplacementSourceInputs::empty()
                    .with_string_cursor_slot_width_px(cursor_slot_width_px)
            }
            DisplayReplacementProperty::Stretch(_) => {
                let (display_ch, _) = decode_utf8(self.source_event.source_text());
                let display_char_width = self.active_face_state.advance_for_char(
                    self.font_metrics,
                    display_ch,
                    face_metrics.char_width,
                );
                DisplayPropertyReplacementSourceInputs::empty()
                    .with_stretch_display_char_width_px(display_char_width)
            }
            DisplayReplacementProperty::Media(media_replacement) => {
                let media = DisplayReplacementMediaSourceItem::resolve_display_property(
                    self.source_event.value(),
                    media_replacement,
                    self.display_host,
                    self.active_face_state,
                    face_metrics.char_width,
                    face_metrics.row_height,
                )?;
                DisplayPropertyReplacementSourceInputs::empty().with_media(media)
            }
        };
        DisplayPropertyReplacementSourceItem::from_display_property(
            display_property,
            source_event,
            self.current_x,
            self.content_x,
            self.params,
            source_metrics,
            source_inputs,
        )
    }
}

impl<'a> BufferDisplayPropertyTextRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer_id: BufferId,
        text_start_byte: usize,
        text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self {
            buffer_id,
            text_start_byte,
            text,
            active_face_state,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
        }
    }

    pub(crate) fn resolve_and_append_at_checkpoint<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        checkpoints: &mut TextPropertyScanCheckpoints,
        charpos: i64,
        byte_idx: usize,
        accessible_end: i64,
    ) -> BufferDisplayPropertyTextAppendAction {
        if !checkpoints.should_check_display(charpos) {
            return BufferDisplayPropertyTextAppendAction::None;
        }

        let text_props = RustTextPropAccess::new(buffer);
        let (display_property, next_change) = text_props.check_display_prop(charpos);
        checkpoints.record_display_next(next_change);
        let Some(value) = display_property else {
            return BufferDisplayPropertyTextAppendAction::None;
        };

        let source_event = BufferDisplayPropertyTextSourceEvent::new(
            value,
            self.text_start_byte,
            self.text,
            charpos,
            byte_idx,
            checkpoints.display_next(),
            checkpoints.display_skip_to(accessible_end),
        );
        BufferDisplayPropertyTextAppendRequest::for_source_event(
            source_event,
            BufferDisplayPropertyTextAppendContext {
                buffer_id: self.buffer_id,
                active_face_state: self.active_face_state,
                current_x: self.current_x,
                content_x: self.content_x,
                params: self.params,
                glyph_y_offset: self.glyph_y_offset,
                default_row_height: self.default_row_height,
                start_position: self.start_position,
            },
        )
        .resolve_and_append_to_text_row(
            buffer,
            state,
            face_ids,
            append_surface,
            row_geometry,
        )
    }
}

impl<'a, B: LayoutBufferView> BufferDisplayPropertyCheckpointRenderRequest<'a, B> {
    pub(crate) fn new(context: BufferDisplayPropertyCheckpointRenderContext<'a, B>) -> Self {
        Self { context }
    }

    pub(crate) fn render_and_apply(
        self,
        state: BufferDisplayPropertyCheckpointRenderState<'_, '_>,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        let BufferDisplayPropertyCheckpointRenderState {
            mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            checkpoints,
            face_scan,
            active_face_state,
            row_extend,
            box_face,
            byte_idx,
            charpos,
            x,
            col,
            cursor_info,
            raise_span,
            height_span,
            point_charpos,
        } = state;
        let context = self.context;

        BufferDisplayPropertyTextModifierAction::clear_expired_height_span(
            height_span,
            face_scan,
            context.charpos,
            context.params.window_start,
        );
        context
            .face_resolution_context
            .resolve_at_checkpoint_with_source_state(
                &mut source_render,
                face_scan,
                height_span,
                face_ids,
                active_face_state,
                row_geometry,
                row_extend,
                box_face,
                *x,
                context.charpos,
            );

        let action = BufferDisplayPropertyTextRenderContext::new(
            context.buffer_id,
            context.text_start_byte,
            context.text,
            active_face_state,
            context.current_x,
            context.content_x,
            context.params,
            context.glyph_y_offset,
            context.default_row_height,
            context.start_position,
        )
        .resolve_and_append_at_checkpoint(
            context.face_resolution_context.buffer,
            &mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            checkpoints,
            context.charpos,
            context.byte_idx,
            context.accessible_end,
        );
        let outcome = action.apply_to_buffer_walk_state(
            context.text,
            byte_idx,
            charpos,
            x,
            col,
            cursor_info,
            active_face_state,
            row_geometry,
            point_charpos,
            raise_span,
            height_span,
            face_scan,
        );
        if outcome.should_resolve_face() {
            context
                .face_resolution_context
                .resolve_at_checkpoint_with_source_state(
                    &mut source_render,
                    face_scan,
                    height_span,
                    face_ids,
                    active_face_state,
                    row_geometry,
                    row_extend,
                    box_face,
                    *x,
                    *charpos,
                );
        }
        outcome
    }
}

impl<'a> BufferDisplayPropertyTextAppendRequest<'a> {
    pub(crate) fn for_source_event(
        source_event: BufferDisplayPropertyTextSourceEvent<'a>,
        context: BufferDisplayPropertyTextAppendContext<'a>,
    ) -> Self {
        Self {
            source_event,
            context,
        }
    }

    pub(crate) fn resolve_and_append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> BufferDisplayPropertyTextAppendAction {
        let context = self.context;
        let display_property = classify_display_property(self.source_event.value());
        let replacement_item = state.with_font_metrics_and_display_host(|font_metrics, host| {
            DisplayPropertyReplacementSourceResolveRequest::new(
                &display_property,
                self.source_event,
                context.active_face_state,
                font_metrics,
                context.current_x,
                context.content_x,
                context.params,
                host,
            )
            .resolve()
        });
        if let Some(item) = replacement_item {
            let replacement = DisplayPropertyReplacementAppendRequest::new(
                BufferDisplayReplacementSource::new(
                    context.buffer_id,
                    self.source_event.anchor_charpos(),
                    self.source_event.anchor_bytepos(),
                ),
                item,
                context.glyph_y_offset,
                context.default_row_height,
                context.start_position,
            )
            .append_to_text_row(
                buffer,
                state,
                face_ids,
                append_surface,
                row_geometry,
                context.active_face_state,
            );
            return BufferDisplayPropertyTextAppendAction::Replacement(
                BufferDisplayPropertyTextReplacementOutcome {
                    replacement,
                    skip_to: self.source_event.skip_to(),
                },
            );
        }

        BufferDisplayPropertyTextModifierAction::for_display_property(
            &display_property,
            context.default_row_height,
            self.source_event.next_change(),
        )
        .map(BufferDisplayPropertyTextAppendAction::Modifiers)
        .unwrap_or(BufferDisplayPropertyTextAppendAction::None)
    }
}

impl BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) fn point_in_replacement(self, point_charpos: i64, start_charpos: i64) -> bool {
        point_charpos >= start_charpos && point_charpos < self.skip_to
    }

    pub(crate) fn start_position(self) -> DisplayRowPosition {
        self.replacement.start_position()
    }

    pub(crate) fn end_position(self) -> DisplayRowPosition {
        self.replacement.end_position()
    }

    pub(crate) fn skip_covered_buffer_text(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
    ) {
        skip_text_to_charpos(text, byte_idx, charpos, self.skip_to);
    }

    pub(crate) fn capture_cursor_info_if_point(
        self,
        cursor_info: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        start_charpos: i64,
        byte_idx: usize,
    ) {
        if cursor_info.is_missing() && self.point_in_replacement(point_charpos, start_charpos) {
            let start_position = self.start_position();
            capture_cursor_info(
                cursor_info,
                self.cursor_info(
                    active_face_state,
                    row_geometry.text_position(start_position.x_px, byte_idx, start_position.col),
                ),
            );
        }
    }

    pub(crate) fn apply_to_walk_state(
        self,
        text: &[u8],
        byte_idx: &mut usize,
        charpos: &mut i64,
        x: &mut f32,
        col: &mut usize,
    ) {
        let position = self.end_position();
        *x = position.x_px;
        *col = position.col;
        self.skip_covered_buffer_text(text, byte_idx, charpos);
    }

    #[cfg(test)]
    pub(crate) fn skip_to(self) -> i64 {
        self.skip_to
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
    ) -> CapturedCursorInfo {
        self.replacement.cursor_info(active_face_state, position)
    }
}

impl BufferDisplayPropertyTextModifierAction {
    pub(crate) fn clear_expired_raise_span(
        raise_span: &mut ActiveDisplayPropertySpan<f32>,
        charpos: i64,
        inactive_end_charpos: i64,
    ) {
        let _ = raise_span.clear_if_expired(charpos, inactive_end_charpos);
    }

    pub(crate) fn clear_expired_height_span(
        height_span: &mut ActiveDisplayPropertySpan<f32>,
        face_scan: &mut FaceScanCheckpoint,
        charpos: i64,
        inactive_end_charpos: i64,
    ) -> BufferDisplayPropertyTextModifierStateOutcome {
        let height_face_changed = height_span.clear_if_expired(charpos, inactive_end_charpos);
        if height_face_changed {
            face_scan.invalidate();
        }
        BufferDisplayPropertyTextModifierStateOutcome::new(height_face_changed)
    }

    pub(crate) fn apply_to_walk_state(
        self,
        raise_span: &mut ActiveDisplayPropertySpan<f32>,
        height_span: &mut ActiveDisplayPropertySpan<f32>,
        face_scan: &mut FaceScanCheckpoint,
    ) -> BufferDisplayPropertyTextModifierStateOutcome {
        if let Some(raise_offset_px) = self.raise_offset_px() {
            raise_span.set(raise_offset_px, self.next_change());
        }
        let height_face_changed = if let Some(factor) = self.height_factor() {
            height_span.set(factor, self.next_change());
            face_scan.invalidate();
            true
        } else {
            false
        };
        BufferDisplayPropertyTextModifierStateOutcome::new(height_face_changed)
    }
}

#[cfg(test)]
pub(crate) struct DisplayPropertyReplacementAppendResolveRequest<'a> {
    display_property: &'a DisplayPropertyClassification,
    value: Value,
    replacement_source: BufferDisplayReplacementSource,
    anchor_charpos: CharPos0,
    source_text: &'a [u8],
    active_face_state: &'a DisplayRowActiveFaceState,
    current_x: f32,
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

#[cfg(test)]
impl<'a> DisplayPropertyReplacementAppendResolveRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        display_property: &'a DisplayPropertyClassification,
        value: Value,
        replacement_source: BufferDisplayReplacementSource,
        anchor_charpos: CharPos0,
        source_text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self {
            display_property,
            value,
            replacement_source,
            anchor_charpos,
            source_text,
            active_face_state,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_text_property(
        display_property: &'a DisplayPropertyClassification,
        value: Value,
        buffer_id: BufferId,
        anchor_charpos: CharPos0,
        anchor_bytepos: EmacsBytePos,
        source_text: &'a [u8],
        active_face_state: &'a DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self::new(
            display_property,
            value,
            BufferDisplayReplacementSource::new(buffer_id, anchor_charpos, anchor_bytepos),
            anchor_charpos,
            source_text,
            active_face_state,
            current_x,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            start_position,
        )
    }

    pub(crate) fn resolve(
        self,
        font_metrics: &mut Option<FontMetricsService>,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<DisplayPropertyReplacementAppendRequest> {
        let source_event = BufferDisplayPropertyTextSourceEvent::with_anchor(
            self.value,
            self.anchor_charpos,
            self.replacement_source.byte_pos(),
            self.source_text,
            0,
            0,
        );
        let item = DisplayPropertyReplacementSourceResolveRequest::new(
            self.display_property,
            source_event,
            self.active_face_state,
            font_metrics,
            self.current_x,
            self.content_x,
            self.params,
            display_host,
        )
        .resolve()?;
        Some(DisplayPropertyReplacementAppendRequest::new(
            self.replacement_source,
            item,
            self.glyph_y_offset,
            self.default_row_height,
            self.start_position,
        ))
    }

    pub(crate) fn resolve_and_append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> Option<DisplayPropertyReplacementAppendOutcome> {
        let active_face_state = self.active_face_state;
        let request = state.with_font_metrics_and_display_host(|font_metrics, host| {
            self.resolve(font_metrics, host)
        })?;
        Some(request.append_to_text_row(
            buffer,
            state,
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        ))
    }
}

#[derive(Clone)]
pub(crate) struct DisplayPropertyReplacementAppendRequest {
    replacement_source: BufferDisplayReplacementSource,
    item: DisplayPropertyReplacementSourceItem,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

impl DisplayPropertyReplacementAppendRequest {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        item: DisplayPropertyReplacementSourceItem,
        glyph_y_offset: f32,
        default_row_height: f32,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self {
            replacement_source,
            item,
            glyph_y_offset,
            default_row_height,
            start_position,
        }
    }

    pub(crate) fn cursor_policy(&self) -> DisplayPropertyReplacementCursorPolicy {
        self.item.cursor_policy()
    }

    pub(crate) fn start_position(&self) -> DisplayRowPosition {
        self.start_position
    }

    pub(crate) fn into_plan<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayPropertyReplacementAppendPlan {
        let item = DisplayPropertyReplacementAppendPlanItemRequest::new(self.item).resolve(
            buffer,
            state,
            active_face_state,
            face_ids,
        );
        DisplayPropertyReplacementAppendPlan {
            replacement_source: self.replacement_source,
            item,
            glyph_y_offset: self.glyph_y_offset,
            default_row_height: self.default_row_height,
            start_position: self.start_position,
        }
    }

    #[cfg(test)]
    pub(crate) fn into_item(self) -> DisplayPropertyReplacementSourceItem {
        self.item
    }

    pub(crate) fn append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayPropertyReplacementAppendOutcome {
        let start_position = self.start_position();
        let cursor_policy = self.cursor_policy();
        let plan = state.display_property_replacement_append_plan(
            self,
            buffer,
            active_face_state,
            face_ids,
        );
        let end_position = plan.append_to_text_row(
            state,
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        );
        DisplayPropertyReplacementAppendOutcome {
            start_position,
            end_position,
            cursor_policy,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayPropertyReplacementAppendOutcome {
    start_position: DisplayRowPosition,
    end_position: DisplayRowPosition,
    cursor_policy: DisplayPropertyReplacementCursorPolicy,
}

impl DisplayPropertyReplacementAppendOutcome {
    pub(crate) fn start_position(self) -> DisplayRowPosition {
        self.start_position
    }

    pub(crate) fn end_position(self) -> DisplayRowPosition {
        self.end_position
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
    ) -> CapturedCursorInfo {
        display_property_replacement_cursor_info(self.cursor_policy, active_face_state, position)
    }
}

pub(crate) struct DisplayPropertyReplacementAppendPlan {
    replacement_source: BufferDisplayReplacementSource,
    item: DisplayPropertyReplacementAppendPlanItem,
    glyph_y_offset: f32,
    default_row_height: f32,
    start_position: DisplayRowPosition,
}

impl DisplayPropertyReplacementAppendPlan {
    #[cfg(test)]
    pub(crate) fn string_append_request(&self) -> Option<&DisplayReplacementStringAppendRequest> {
        match &self.item {
            DisplayPropertyReplacementAppendPlanItem::String(request) => Some(request),
            _ => None,
        }
    }

    pub(crate) fn append_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayRowPosition {
        let position = self.start_position;
        let replacement_append_context = DisplayReplacementRowAppendContext::new(
            self.replacement_source,
            append_surface,
            row_geometry,
            active_face_state,
            self.glyph_y_offset,
            self.default_row_height,
        );
        self.item.append_to_text_row(
            replacement_append_context,
            row_geometry,
            state,
            face_ids,
            position,
        )
    }
}

#[derive(Clone)]
enum DisplayPropertyReplacementAppendPlanItem {
    String(DisplayReplacementStringAppendRequest),
    Stretch(DisplayReplacementStretchSourceItem),
    Media(DisplayReplacementMediaSourceResolution),
}

struct DisplayPropertyReplacementAppendPlanItemRequest {
    item: DisplayPropertyReplacementSourceItem,
}

impl DisplayPropertyReplacementAppendPlanItemRequest {
    fn new(item: DisplayPropertyReplacementSourceItem) -> Self {
        Self { item }
    }

    fn resolve<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayPropertyReplacementAppendPlanItem {
        match self.item {
            DisplayPropertyReplacementSourceItem::String(item) => {
                let replacement_base_face = (!item.is_empty()).then(|| {
                    state.default_display_string_base_face_for_active_row(
                        buffer,
                        item.origin(),
                        active_face_state,
                        face_ids,
                    )
                });
                DisplayPropertyReplacementAppendPlanItem::String(
                    DisplayReplacementStringAppendRequest::new(
                        item,
                        replacement_base_face,
                        active_face_state.clone(),
                    ),
                )
            }
            DisplayPropertyReplacementSourceItem::Stretch(item) => {
                DisplayPropertyReplacementAppendPlanItem::Stretch(item)
            }
            DisplayPropertyReplacementSourceItem::Media(item) => {
                DisplayPropertyReplacementAppendPlanItem::Media(item)
            }
        }
    }
}

impl DisplayPropertyReplacementAppendPlanItem {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        match self {
            Self::String(request) => {
                request.append_to_text_row(replacement_append_context, state, face_ids, position)
            }
            Self::Stretch(stretch_item) => stretch_item.append_to_text_row(
                replacement_append_context,
                row_geometry,
                state,
                position,
            ),
            Self::Media(media_item) => media_item.append_to_text_row(
                replacement_append_context,
                row_geometry,
                state,
                position,
            ),
        }
    }
}

impl DisplayReplacementStretchSourceItem {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        let Some(request) = self.append_request(position) else {
            return position;
        };
        row_geometry.include_glyph_vertical_metrics(self.height_px(), self.ascent_px());
        replacement_append_context
            .append_item_request_to_text_row_and_emit(state, request)
            .map(|(_progress, position)| position)
            .unwrap_or(position)
    }
}

impl DisplayReplacementMediaSourceResolution {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        match self {
            Self::Media(media_item) => media_item.append_to_text_row(
                replacement_append_context,
                row_geometry,
                state,
                position,
            ),
            Self::Placeholder(placeholder_item) => {
                placeholder_item.append_to_text_row(replacement_append_context, state, position)
            }
        }
    }
}

impl DisplayReplacementMediaSourceItem {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        if let Some((progress, appended_position)) = replacement_append_context
            .append_item_request_to_text_row_and_emit(state, self.append_request(position))
            && let Some((height, ascent)) = self.row_extents_after_append(&progress)
        {
            row_geometry.include_row_extents(height, ascent);
            appended_position
        } else {
            position
        }
    }
}

impl DisplayReplacementSourceMappedTextItem {
    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        replacement_append_context
            .append_item_request_to_text_row_and_emit(state, self.append_request(position))
            .map(|(_progress, position)| position)
            .unwrap_or(position)
    }
}

impl DisplayReplacementMediaSourceItem {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_display_property(
        display_prop: Value,
        replacement: &DisplayMediaReplacementProperty,
        display_host: Option<&dyn DisplayHost>,
        active_face_state: &DisplayRowActiveFaceState,
        fallback_char_width: f32,
        fallback_row_height: f32,
    ) -> Option<DisplayReplacementMediaSourceResolution> {
        match resolve_display_replacement(
            display_prop,
            replacement,
            display_host,
            active_face_state.resolved_face(),
            fallback_char_width,
            fallback_row_height,
        )? {
            ResolvedDisplayReplacement::Media(media) => {
                Some(DisplayReplacementMediaSourceResolution::Media(Self::new(
                    media,
                    active_face_state.metrics().row_height,
                    active_face_state.metrics().ascent,
                    replacement.uses_xwidget_cursor_extents(),
                )))
            }
            ResolvedDisplayReplacement::Placeholder(placeholder) => {
                Some(DisplayReplacementMediaSourceResolution::Placeholder(
                    DisplayReplacementSourceMappedTextItem::new(placeholder),
                ))
            }
        }
    }

    pub(crate) fn row_extents_after_append(
        self,
        progress: &DisplayRowAppendProgress,
    ) -> Option<(f32, f32)> {
        if progress.status == DisplayRowAppendStatus::Complete && progress.metrics.width_px > 0.0 {
            Some((self.display_height_px(), self.display_ascent_px()))
        } else {
            None
        }
    }

    fn append_request(self, position: DisplayRowPosition) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest::display_box(
            DisplayReplacementAppendItem::media(self.media()),
            self.display_height_px(),
            self.display_ascent_px(),
            position,
        )
    }
}

impl DisplayReplacementSourceMappedTextItem {
    fn append_request(self, position: DisplayRowPosition) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest::active_face(
            DisplayReplacementAppendItem::source_mapped_text(self.into_text()),
            position,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayReplacementRowAppendContext<'a> {
    replacement_source: BufferDisplayReplacementSource,
    append_surface: &'a DisplayRowAppendSurface,
    placement: DisplayRowAppendPlacement,
    active_face: &'a DisplayRowActiveFaceState,
    default_row_height: f32,
}

impl<'a> DisplayReplacementRowAppendContext<'a> {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        default_row_height: f32,
    ) -> Self {
        Self {
            replacement_source,
            append_surface,
            placement: DisplayRowAppendPlacement::from_geometry_state(geometry, glyph_y_offset),
            active_face,
            default_row_height,
        }
    }

    fn active_face_frame(self) -> DisplayRowAppendFrame {
        self.append_surface.frame(
            self.placement,
            DisplayRowAppendMetrics::from_active_face_state(
                self.active_face,
                self.default_row_height,
            ),
        )
    }

    fn full_text_width_active_face_frame(self) -> DisplayRowAppendFrame {
        self.append_surface.full_text_width_surface().frame(
            self.placement,
            DisplayRowAppendMetrics::from_active_face_state(
                self.active_face,
                self.default_row_height,
            ),
        )
    }

    fn display_box_frame(self, height_px: f32, ascent_px: f32) -> DisplayRowAppendFrame {
        self.append_surface.frame(
            self.placement,
            DisplayRowAppendMetrics::display_box_from_active_face_state(
                self.active_face,
                height_px,
                ascent_px,
                self.default_row_height,
            ),
        )
    }

    fn active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.active_face_frame(),
        )
    }

    fn full_text_width_active_face(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.full_text_width_active_face_frame(),
        )
    }

    fn display_box(
        self,
        face_id: u32,
        base_face: &'a ResolvedFace,
        height_px: f32,
        ascent_px: f32,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            self.replacement_source,
            face_id,
            base_face,
            self.display_box_frame(height_px, ascent_px),
        )
    }

    fn append_item_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        request: DisplayReplacementItemAppendRequest,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let DisplayReplacementItemAppendRequest {
            item,
            frame,
            position,
        } = request;
        let append_context = match frame {
            DisplayReplacementItemAppendFrame::ActiveFace => {
                self.active_face(self.active_face.face_id(), self.active_face.resolved_face())
            }
            DisplayReplacementItemAppendFrame::DisplayBox {
                height_px,
                ascent_px,
            } => self.display_box(
                self.active_face.face_id(),
                self.active_face.resolved_face(),
                height_px,
                ascent_px,
            ),
        };
        append_context.append_replacement_item_to_text_row_and_emit(state, item, position)
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementAppendContext<'a> {
    replacement_source: BufferDisplayReplacementSource,
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> DisplayReplacementAppendContext<'a> {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        face_id: u32,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            replacement_source,
            face_id,
            base_face,
            frame,
        }
    }

    fn append_replacement_item_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        item: DisplayReplacementAppendItem,
        position: DisplayRowPosition,
    ) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
        let item = item.into_display_item(self.replacement_source, self.face_id);
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        DisplayRowSourceAppendOperation::for_single_item(
            &item,
            self.base_face,
            self.face_id,
            self.frame.clone(),
            position,
            DisplayRowAppendKind::DisplayReplacement,
        )
        .render_single_item_to_text_row_and_emit(state, item, &mut render_policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DisplayRowAppendPlacement {
    row: usize,
    y: f32,
    glyph_y: f32,
}

impl DisplayRowAppendPlacement {
    fn new(row: usize, y: f32, glyph_y: f32) -> Self {
        Self { row, y, glyph_y }
    }

    fn from_geometry_state(geometry: &DisplayRowGeometryState, glyph_y_offset: f32) -> Self {
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

    fn frame(
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
    height: f32,
    ascent: f32,
    char_width: f32,
    space_width: f32,
    default_row_height: f32,
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
enum DisplayRowAppendKind {
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

fn append_synthetic_text_to_display_row(
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
