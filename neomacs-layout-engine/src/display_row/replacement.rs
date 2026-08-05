use crate::display_cursor::{CapturedCursorInfo, display_property_replacement_cursor_info};
#[cfg(test)]
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{
    BufferDisplayReplacementSource, DisplayItem, DisplayItemKind, DisplayPointerAppearance,
    DisplayPropertyReplacementDescriptor,
};
#[cfg(test)]
use crate::display_origin::DisplayOrigin;
use crate::display_row::append_context::{
    DisplayRowAppendFrame, DisplayRowAppendKind, DisplayRowAppendMetrics,
    DisplayRowAppendPlacement, DisplayRowAppendSurface,
};
use crate::display_row::builder::{
    DisplayRowAppendProgress, DisplayRowItemMeasurement, DisplayRowPosition,
};
use crate::display_row::face_state::DisplayRowActiveFaceState;
use crate::display_row::geometry::{DisplayRowGeometryState, DisplayRowTextPosition};
#[cfg(test)]
use crate::display_row::lisp_string::LispStringSourceId;
use crate::display_row::metrics::DisplayRowFallbackMetrics;
use crate::display_row::render_policy::{DisplayRowRenderClipBehavior, DisplayRowRenderPolicy};
use crate::display_row::source_append::SingleDisplayItemAppendContext;
use crate::display_row::source_render::TextRowSourceRenderState;
use crate::display_row::source_state::DisplayRowSourceState;
use crate::display_source::{
    BufferDisplayReplacementStringRequest, DisplayItemOnceSource,
    DisplayPropertyReplacementCursorPolicy, DisplayPropertyReplacementSourceItem,
    DisplayReplacementMediaSourceItem, DisplayReplacementMediaSourceResolution,
    DisplayReplacementSourceMappedTextItem, DisplayReplacementStretchSourceItem,
    DisplayReplacementStringSourceItem,
};
use crate::display_source_append_plan::NaturalDisplayRowAppendRenderPolicy;
use crate::display_source_resolver::{
    DisplayPropertyReplacementSourceResolveRequest, DisplayStringBaseFace,
};
use crate::font::metrics::FontMetricsService;
use crate::frame_face_arena::FrameFaceAttempt;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace};
use crate::types::WindowParams;
use neomacs_display_protocol::types::FaceId;
#[cfg(test)]
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;

pub(crate) struct DisplayReplacementStringItemMeasurer {
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayRowRenderPolicy for DisplayReplacementStringItemMeasurer {
    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        _face_id: FaceId,
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
    /// Set when the display string contains a newline (a `RowBreak` item). GNU
    /// (xdisp.c `display_line`) treats a '\n' inside a `display` string as a
    /// row terminator, exactly like a buffer newline; the caller must emit a
    /// row break so the following buffer text starts on a fresh row.
    produced_row_break: bool,
}

impl<M: DisplayRowRenderPolicy> DisplayRowRenderPolicy
    for DisplayReplacementStringRenderPolicy<'_, M>
{
    fn stop_before_item(&mut self, item: &DisplayItem) -> bool {
        if matches!(item.kind, DisplayItemKind::RowBreak(_)) {
            self.produced_row_break = true;
            true
        } else {
            false
        }
    }

    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        face_id: FaceId,
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

#[derive(Clone, Debug)]
struct DisplayReplacementStringSourceAppendRequest {
    position: DisplayRowPosition,
    source: BufferDisplayReplacementStringRequest,
}

impl DisplayReplacementStringSourceAppendRequest {
    fn new(position: DisplayRowPosition, source: BufferDisplayReplacementStringRequest) -> Self {
        Self { position, source }
    }

    fn position(&self) -> DisplayRowPosition {
        self.position
    }

    fn render_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        append_context: &DisplayReplacementAppendContext<'_>,
        item_policy: &mut impl DisplayRowRenderPolicy,
    ) -> DisplayReplacementAppendResult {
        let position = self.position();
        let Some(source) = self
            .source
            .into_source(append_context.single_item.face_id())
        else {
            return DisplayReplacementAppendResult::without_row_break(position);
        };
        let mut render_policy = DisplayReplacementStringRenderPolicy {
            item_policy,
            produced_row_break: false,
        };
        let mut source = source;
        let mut source_state = DisplayRowSourceState::default();
        let Some(outcome) = append_context.single_item.render_source_with_policy(
            state,
            face_ids,
            &mut source,
            &mut source_state,
            position,
            DisplayRowAppendKind::DisplayReplacementString,
            &mut render_policy,
            append_context.single_item.face_id(),
        ) else {
            return DisplayReplacementAppendResult::new(position, render_policy.produced_row_break);
        };
        DisplayReplacementAppendResult::new(
            outcome.end_position(),
            render_policy.produced_row_break,
        )
    }
}

/// Result of appending a display-property replacement onto a text row: the
/// position after the appended content plus whether the replacement's content
/// (a `display` string) contained a newline that must terminate the row.
#[derive(Clone, Copy)]
pub(crate) struct DisplayReplacementAppendResult {
    position: DisplayRowPosition,
    produced_row_break: bool,
}

impl DisplayReplacementAppendResult {
    fn new(position: DisplayRowPosition, produced_row_break: bool) -> Self {
        Self {
            position,
            produced_row_break,
        }
    }

    fn without_row_break(position: DisplayRowPosition) -> Self {
        Self::new(position, false)
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    fn produced_row_break(self) -> bool {
        self.produced_row_break
    }
}

#[derive(Clone)]
struct DisplayReplacementStringAppendRequest {
    item: DisplayReplacementStringSourceItem,
    replacement_base_face: Option<DisplayStringBaseFace>,
    active_face_state: DisplayRowActiveFaceState,
}

#[cfg(test)]
pub(crate) struct DisplayReplacementStringSourceSnapshot {
    value: Value,
    source_id: LispStringSourceId,
    position: DisplayRowPosition,
    origin: DisplayOrigin,
    base_face_policy: BaseFacePolicy,
    cursor_slot_width_px: f32,
    is_empty: bool,
}

#[cfg(test)]
pub(crate) struct DisplayPropertyReplacementStringPlanSnapshot {
    origin: DisplayOrigin,
    base_face_policy: BaseFacePolicy,
    has_replacement_base_face: bool,
}

#[cfg(test)]
impl DisplayReplacementStringSourceSnapshot {
    pub(crate) fn value(&self) -> Value {
        self.value
    }

    pub(crate) fn source_id(&self) -> LispStringSourceId {
        self.source_id
    }

    pub(crate) fn position(&self) -> DisplayRowPosition {
        self.position
    }

    pub(crate) fn origin(&self) -> DisplayOrigin {
        self.origin
    }

    pub(crate) fn base_face_policy(&self) -> BaseFacePolicy {
        self.base_face_policy
    }

    pub(crate) fn cursor_slot_width_px(&self) -> f32 {
        self.cursor_slot_width_px
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.is_empty
    }
}

#[cfg(test)]
impl DisplayPropertyReplacementStringPlanSnapshot {
    pub(crate) fn origin(&self) -> DisplayOrigin {
        self.origin
    }

    pub(crate) fn base_face_policy(&self) -> BaseFacePolicy {
        self.base_face_policy
    }

    pub(crate) fn has_replacement_base_face(&self) -> bool {
        self.has_replacement_base_face
    }
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

    fn string_item_measurer(&self) -> DisplayReplacementStringItemMeasurer {
        DisplayReplacementStringItemMeasurer {
            active_face_state: self.active_face_state.clone(),
        }
    }

    #[cfg(test)]
    fn plan_snapshot(&self) -> DisplayPropertyReplacementStringPlanSnapshot {
        DisplayPropertyReplacementStringPlanSnapshot {
            origin: self.item.origin(),
            base_face_policy: self.item.base_face_policy(),
            has_replacement_base_face: self.replacement_base_face.is_some(),
        }
    }

    fn source_append_request(
        &self,
        replacement_source: BufferDisplayReplacementSource,
        position: DisplayRowPosition,
        pointer_appearance: Option<DisplayPointerAppearance>,
    ) -> DisplayReplacementStringSourceAppendRequest {
        DisplayReplacementStringSourceAppendRequest::new(
            position,
            BufferDisplayReplacementStringRequest::new(
                self.item.source_id(),
                self.item.value(),
                replacement_source,
            )
            .with_pointer_appearance(pointer_appearance),
        )
    }

    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        position: DisplayRowPosition,
        pointer_appearance: Option<DisplayPointerAppearance>,
    ) -> DisplayReplacementAppendResult {
        if self.item.is_empty() {
            return DisplayReplacementAppendResult::without_row_break(position);
        }
        let Some(ref replacement_base_face) = self.replacement_base_face else {
            debug_assert!(false, "display string replacement missing base face");
            return DisplayReplacementAppendResult::without_row_break(position);
        };
        let source_request = self.source_append_request(
            replacement_append_context.replacement_source,
            position,
            pointer_appearance,
        );
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

#[cfg(test)]
impl DisplayReplacementStringSourceItem {
    pub(crate) fn append_source_snapshot(
        &self,
        position: DisplayRowPosition,
    ) -> DisplayReplacementStringSourceSnapshot {
        DisplayReplacementStringSourceSnapshot {
            value: self.value(),
            source_id: LispStringSourceId::display_replacement(self.source_id()),
            position,
            origin: self.origin(),
            base_face_policy: self.base_face_policy(),
            cursor_slot_width_px: self.cursor_slot_width_px(),
            is_empty: self.is_empty(),
        }
    }

    pub(crate) fn measurement_from_active_face(
        &self,
        active_face_state: &DisplayRowActiveFaceState,
        item: &DisplayItem,
        face_id: FaceId,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        let mut measurer = DisplayReplacementStringItemMeasurer {
            active_face_state: active_face_state.clone(),
        };
        DisplayRowRenderPolicy::measurement_for(&mut measurer, item, face_id, font_metrics)
    }
}

#[derive(Clone, Debug)]
struct DisplayReplacementItemAppendRequest {
    kind: DisplayItemKind,
    frame: DisplayReplacementItemAppendFrame,
    position: DisplayRowPosition,
    pointer_appearance: Option<DisplayPointerAppearance>,
}

#[derive(Clone, Debug)]
struct DisplayReplacementItemAppendPlan {
    item: DisplayItem,
    frame: DisplayReplacementItemAppendFrame,
    position: DisplayRowPosition,
}

#[derive(Clone, Debug)]
struct DisplayReplacementItemAppendTemplate {
    kind: DisplayItemKind,
    frame: DisplayReplacementItemAppendFrame,
    row_geometry_update: DisplayReplacementItemRowGeometryUpdate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DisplayReplacementItemAppendFrame {
    ActiveFace,
    DisplayBox { height_px: f32, ascent_px: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DisplayReplacementItemRowGeometryUpdate {
    None,
    BeforeAppendGlyphMetrics { height_px: f32, ascent_px: f32 },
    AfterCompleteRowExtents { height_px: f32, ascent_px: f32 },
}

impl DisplayReplacementItemAppendRequest {
    #[cfg(test)]
    fn active_face(kind: DisplayItemKind, position: DisplayRowPosition) -> Self {
        Self {
            kind,
            frame: DisplayReplacementItemAppendFrame::ActiveFace,
            position,
            pointer_appearance: None,
        }
    }

    #[cfg(test)]
    fn display_box(
        kind: DisplayItemKind,
        height_px: f32,
        ascent_px: f32,
        position: DisplayRowPosition,
    ) -> Self {
        Self {
            kind,
            frame: DisplayReplacementItemAppendFrame::DisplayBox {
                height_px,
                ascent_px,
            },
            position,
            pointer_appearance: None,
        }
    }

    fn into_plan(
        self,
        replacement_source: BufferDisplayReplacementSource,
        face_id: FaceId,
    ) -> DisplayReplacementItemAppendPlan {
        DisplayReplacementItemAppendPlan {
            item: replacement_source
                .display_item(face_id, self.kind)
                .with_pointer_appearance(self.pointer_appearance),
            frame: self.frame,
            position: self.position,
        }
    }
}

impl DisplayReplacementItemAppendPlan {
    fn frame(&self) -> DisplayReplacementItemAppendFrame {
        self.frame
    }

    fn into_parts(self) -> (DisplayItem, DisplayRowPosition) {
        (self.item, self.position)
    }
}

impl DisplayReplacementItemAppendTemplate {
    fn active_face(
        kind: DisplayItemKind,
        row_geometry_update: DisplayReplacementItemRowGeometryUpdate,
    ) -> Self {
        Self {
            kind,
            frame: DisplayReplacementItemAppendFrame::ActiveFace,
            row_geometry_update,
        }
    }

    fn display_box(
        kind: DisplayItemKind,
        height_px: f32,
        ascent_px: f32,
        row_geometry_update: DisplayReplacementItemRowGeometryUpdate,
    ) -> Self {
        Self {
            kind,
            frame: DisplayReplacementItemAppendFrame::DisplayBox {
                height_px,
                ascent_px,
            },
            row_geometry_update,
        }
    }

    fn from_stretch(item: DisplayReplacementStretchSourceItem) -> Option<Self> {
        (item.width_px() > 0.0).then(|| {
            Self::active_face(
                item.display_item_kind(),
                DisplayReplacementItemRowGeometryUpdate::BeforeAppendGlyphMetrics {
                    height_px: item.height_px(),
                    ascent_px: item.ascent_px(),
                },
            )
        })
    }

    fn from_media_resolution(item: DisplayReplacementMediaSourceResolution) -> Self {
        match item {
            DisplayReplacementMediaSourceResolution::Media(media_item) => Self::display_box(
                DisplayItemKind::MediaReplacement(media_item.media()),
                media_item.display_height_px(),
                media_item.display_ascent_px(),
                DisplayReplacementItemRowGeometryUpdate::AfterCompleteRowExtents {
                    height_px: media_item.display_height_px(),
                    ascent_px: media_item.display_ascent_px(),
                },
            ),
            DisplayReplacementMediaSourceResolution::Placeholder(placeholder_item) => {
                Self::active_face(
                    DisplayItemKind::SourceMappedText(
                        crate::display_item::DisplaySourceMappedText::new(
                            placeholder_item.into_text(),
                        ),
                    ),
                    DisplayReplacementItemRowGeometryUpdate::None,
                )
            }
        }
    }

    fn into_request(
        self,
        position: DisplayRowPosition,
        pointer_appearance: Option<DisplayPointerAppearance>,
    ) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest {
            kind: self.kind,
            frame: self.frame,
            position,
            pointer_appearance,
        }
    }

    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        position: DisplayRowPosition,
        pointer_appearance: Option<DisplayPointerAppearance>,
    ) -> DisplayRowPosition {
        let geometry_update = self.row_geometry_update;
        if let DisplayReplacementItemRowGeometryUpdate::BeforeAppendGlyphMetrics {
            height_px,
            ascent_px,
        } = geometry_update
        {
            row_geometry.include_glyph_vertical_metrics(height_px, ascent_px);
        }
        let Some(progress) = replacement_append_context.append_item_request_to_text_row_and_emit(
            state,
            face_ids,
            self.into_request(position, pointer_appearance),
        ) else {
            return position;
        };
        if let DisplayReplacementItemRowGeometryUpdate::AfterCompleteRowExtents {
            height_px,
            ascent_px,
        } = geometry_update
            && progress.is_complete_with_positive_width()
        {
            row_geometry.include_row_extents(height_px, ascent_px);
        }
        progress.end()
    }
}

#[derive(Clone)]
struct DisplayPropertyReplacementAppendRequest {
    replacement_source: BufferDisplayReplacementSource,
    item: DisplayPropertyReplacementSourceItem,
    glyph_y_offset: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
    start_position: DisplayRowPosition,
    pointer_appearance: Option<DisplayPointerAppearance>,
}

impl DisplayPropertyReplacementAppendRequest {
    fn new(
        replacement_source: BufferDisplayReplacementSource,
        item: DisplayPropertyReplacementSourceItem,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        start_position: DisplayRowPosition,
        pointer_appearance: Option<DisplayPointerAppearance>,
    ) -> Self {
        Self {
            replacement_source,
            item,
            glyph_y_offset,
            fallback_metrics,
            start_position,
            pointer_appearance,
        }
    }

    fn cursor_policy(&self) -> DisplayPropertyReplacementCursorPolicy {
        self.item.cursor_policy()
    }

    #[allow(clippy::too_many_arguments)]
    fn from_typed_replacement_descriptor(
        descriptor: &DisplayPropertyReplacementDescriptor,
        source_text: &[u8],
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        current_x: f32,
        content_x: f32,
        params: &WindowParams,
        display_host: Option<&dyn DisplayHost>,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        start_position: DisplayRowPosition,
    ) -> Option<Self> {
        let item = DisplayPropertyReplacementSourceResolveRequest::from_typed_replacement(
            descriptor.classification(),
            descriptor.anchor_charpos(),
            source_text,
            active_face_state,
            font_metrics,
            current_x,
            content_x,
            params,
            display_host,
        )
        .resolve()?;
        Some(Self::new(
            descriptor.replacement_source(),
            item,
            glyph_y_offset,
            fallback_metrics,
            start_position,
            descriptor.pointer_appearance().cloned(),
        ))
    }

    fn start_position(&self) -> DisplayRowPosition {
        self.start_position
    }

    fn into_plan<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceAttempt,
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
            fallback_metrics: self.fallback_metrics,
            start_position: self.start_position,
            pointer_appearance: self.pointer_appearance,
        }
    }

    fn append_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayPropertyReplacementAppendOutcome {
        let start_position = self.start_position();
        let cursor_policy = self.cursor_policy();
        let plan = self.into_plan(buffer, state, active_face_state, face_ids);
        let append_result = plan.append_to_text_row(
            state,
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        );
        DisplayPropertyReplacementAppendOutcome::new(
            start_position,
            append_result.position(),
            cursor_policy,
            append_result.produced_row_break(),
        )
    }
}

#[derive(Clone)]
pub(crate) struct DisplayPropertyReplacementRowRenderRequest {
    append_request: DisplayPropertyReplacementAppendRequest,
}

impl DisplayPropertyReplacementRowRenderRequest {
    #[cfg(test)]
    pub(crate) fn from_resolved_source_item(
        replacement_source: BufferDisplayReplacementSource,
        item: DisplayPropertyReplacementSourceItem,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        start_position: DisplayRowPosition,
    ) -> Self {
        Self {
            append_request: DisplayPropertyReplacementAppendRequest::new(
                replacement_source,
                item,
                glyph_y_offset,
                fallback_metrics,
                start_position,
                None,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_typed_replacement_descriptor(
        descriptor: &DisplayPropertyReplacementDescriptor,
        source_text: &[u8],
        active_face_state: &DisplayRowActiveFaceState,
        font_metrics: &mut Option<FontMetricsService>,
        current_x: f32,
        content_x: f32,
        params: &WindowParams,
        display_host: Option<&dyn DisplayHost>,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        start_position: DisplayRowPosition,
    ) -> Option<Self> {
        DisplayPropertyReplacementAppendRequest::from_typed_replacement_descriptor(
            descriptor,
            source_text,
            active_face_state,
            font_metrics,
            current_x,
            content_x,
            params,
            display_host,
            glyph_y_offset,
            fallback_metrics,
            start_position,
        )
        .map(|append_request| Self { append_request })
    }

    #[cfg(test)]
    pub(crate) fn cursor_policy(&self) -> DisplayPropertyReplacementCursorPolicy {
        self.append_request.cursor_policy()
    }

    #[cfg(test)]
    pub(crate) fn start_position(&self) -> DisplayRowPosition {
        self.append_request.start_position()
    }

    #[cfg(test)]
    pub(crate) fn into_item(self) -> DisplayPropertyReplacementSourceItem {
        self.append_request.item
    }

    #[cfg(test)]
    pub(crate) fn string_plan_snapshot<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceAttempt,
    ) -> Option<DisplayPropertyReplacementStringPlanSnapshot> {
        self.append_request
            .into_plan(buffer, state, active_face_state, face_ids)
            .string_plan_snapshot()
    }

    pub(crate) fn render_to_text_row<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayPropertyReplacementAppendOutcome {
        self.append_request.append_to_text_row(
            buffer,
            state,
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayPropertyReplacementAppendOutcome {
    start_position: DisplayRowPosition,
    end_position: DisplayRowPosition,
    cursor_policy: DisplayPropertyReplacementCursorPolicy,
    produced_row_break: bool,
}

impl DisplayPropertyReplacementAppendOutcome {
    pub(crate) fn new(
        start_position: DisplayRowPosition,
        end_position: DisplayRowPosition,
        cursor_policy: DisplayPropertyReplacementCursorPolicy,
        produced_row_break: bool,
    ) -> Self {
        Self {
            start_position,
            end_position,
            cursor_policy,
            produced_row_break,
        }
    }

    pub(crate) fn start_position(self) -> DisplayRowPosition {
        self.start_position
    }

    pub(crate) fn end_position(self) -> DisplayRowPosition {
        self.end_position
    }

    /// Whether the replacement's `display` string contained a newline that must
    /// terminate the current row (GNU treats display-string '\n' as a row
    /// break; see xdisp.c `display_line`).
    pub(crate) fn produced_row_break(self) -> bool {
        self.produced_row_break
    }

    pub(crate) fn cursor_info(
        self,
        active_face_state: &DisplayRowActiveFaceState,
        position: DisplayRowTextPosition,
        preceding_charpos: Option<i64>,
    ) -> CapturedCursorInfo {
        display_property_replacement_cursor_info(
            self.cursor_policy,
            active_face_state,
            position,
            preceding_charpos,
        )
    }
}

struct DisplayPropertyReplacementAppendPlan {
    replacement_source: BufferDisplayReplacementSource,
    item: DisplayPropertyReplacementAppendPlanItem,
    glyph_y_offset: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
    start_position: DisplayRowPosition,
    pointer_appearance: Option<DisplayPointerAppearance>,
}

impl DisplayPropertyReplacementAppendPlan {
    #[cfg(test)]
    fn string_plan_snapshot(&self) -> Option<DisplayPropertyReplacementStringPlanSnapshot> {
        match &self.item {
            DisplayPropertyReplacementAppendPlanItem::String(request) => {
                Some(request.plan_snapshot())
            }
            _ => None,
        }
    }

    pub(crate) fn append_to_text_row(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        append_surface: &DisplayRowAppendSurface,
        row_geometry: &mut DisplayRowGeometryState,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> DisplayReplacementAppendResult {
        let position = self.start_position;
        let replacement_append_context = DisplayReplacementRowAppendContext::new(
            self.replacement_source,
            append_surface,
            row_geometry,
            active_face_state,
            self.glyph_y_offset,
            self.fallback_metrics,
        );
        self.item.append_to_text_row(
            replacement_append_context,
            row_geometry,
            state,
            face_ids,
            position,
            self.pointer_appearance,
        )
    }
}

#[derive(Clone)]
// Large payload variant; boxing is a perf hint deferred out of the lint gate.
#[allow(clippy::large_enum_variant)]
enum DisplayPropertyReplacementAppendPlanItem {
    Empty,
    String(DisplayReplacementStringAppendRequest),
    Item(DisplayReplacementItemAppendTemplate),
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
        face_ids: &mut FrameFaceAttempt,
    ) -> DisplayPropertyReplacementAppendPlanItem {
        match self.item {
            DisplayPropertyReplacementSourceItem::Empty => {
                DisplayPropertyReplacementAppendPlanItem::Empty
            }
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
                DisplayReplacementItemAppendTemplate::from_stretch(item)
                    .map(DisplayPropertyReplacementAppendPlanItem::Item)
                    .unwrap_or(DisplayPropertyReplacementAppendPlanItem::Empty)
            }
            DisplayPropertyReplacementSourceItem::Media(item) => {
                DisplayPropertyReplacementAppendPlanItem::Item(
                    DisplayReplacementItemAppendTemplate::from_media_resolution(item),
                )
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
        face_ids: &mut FrameFaceAttempt,
        position: DisplayRowPosition,
        pointer_appearance: Option<DisplayPointerAppearance>,
    ) -> DisplayReplacementAppendResult {
        match self {
            Self::Empty => DisplayReplacementAppendResult::without_row_break(position),
            Self::String(request) => request.append_to_text_row(
                replacement_append_context,
                state,
                face_ids,
                position,
                pointer_appearance,
            ),
            Self::Item(item) => {
                DisplayReplacementAppendResult::without_row_break(item.append_to_text_row(
                    replacement_append_context,
                    row_geometry,
                    state,
                    face_ids,
                    position,
                    pointer_appearance,
                ))
            }
        }
    }
}

impl DisplayReplacementMediaSourceItem {
    #[cfg(test)]
    pub(crate) fn row_extents_after_append(
        self,
        progress: &DisplayRowAppendProgress,
    ) -> Option<(f32, f32)> {
        if progress.is_complete_with_positive_width() {
            Some((self.display_height_px(), self.display_ascent_px()))
        } else {
            None
        }
    }

    #[cfg(test)]
    fn append_request(self, position: DisplayRowPosition) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest::display_box(
            DisplayItemKind::MediaReplacement(self.media()),
            self.display_height_px(),
            self.display_ascent_px(),
            position,
        )
    }
}

impl DisplayReplacementSourceMappedTextItem {
    #[cfg(test)]
    fn append_request(self, position: DisplayRowPosition) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest::active_face(
            DisplayItemKind::SourceMappedText(crate::display_item::DisplaySourceMappedText::new(
                self.into_text(),
            )),
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
    fallback_metrics: DisplayRowFallbackMetrics,
}

impl<'a> DisplayReplacementRowAppendContext<'a> {
    pub(crate) fn new(
        replacement_source: BufferDisplayReplacementSource,
        append_surface: &'a DisplayRowAppendSurface,
        geometry: &DisplayRowGeometryState,
        active_face: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        Self {
            replacement_source,
            append_surface,
            placement: DisplayRowAppendPlacement::from_geometry_state(geometry, glyph_y_offset),
            active_face,
            fallback_metrics,
        }
    }

    fn active_face_frame(self) -> DisplayRowAppendFrame {
        self.append_surface.frame(
            self.placement,
            DisplayRowAppendMetrics::from_active_face_state(
                self.active_face,
                self.fallback_metrics,
            ),
        )
    }

    fn full_text_width_active_face_frame(self) -> DisplayRowAppendFrame {
        self.append_surface.full_text_width_surface().frame(
            self.placement,
            DisplayRowAppendMetrics::from_active_face_state(
                self.active_face,
                self.fallback_metrics,
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
                self.fallback_metrics,
            ),
        )
    }

    fn active_face(
        self,
        face_id: FaceId,
        base_face: &'a ResolvedFace,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(face_id, base_face, self.active_face_frame())
    }

    fn full_text_width_active_face(
        self,
        face_id: FaceId,
        base_face: &'a ResolvedFace,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            face_id,
            base_face,
            self.full_text_width_active_face_frame(),
        )
    }

    fn display_box(
        self,
        face_id: FaceId,
        base_face: &'a ResolvedFace,
        height_px: f32,
        ascent_px: f32,
    ) -> DisplayReplacementAppendContext<'a> {
        DisplayReplacementAppendContext::new(
            face_id,
            base_face,
            self.display_box_frame(height_px, ascent_px),
        )
    }

    fn append_item_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        request: DisplayReplacementItemAppendRequest,
    ) -> Option<DisplayRowAppendProgress> {
        let plan = request.into_plan(self.replacement_source, self.active_face.face_id());
        let append_context = match plan.frame() {
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
        append_context.append_replacement_item_plan_to_text_row_and_emit(state, face_ids, plan)
    }

    #[cfg(test)]
    pub(crate) fn append_stretch_source_item_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        item: DisplayReplacementStretchSourceItem,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        let request =
            DisplayReplacementItemAppendTemplate::from_stretch(item)?.into_request(position, None);
        self.append_item_request_to_text_row_and_emit(state, face_ids, request)
    }

    #[cfg(test)]
    pub(crate) fn append_media_source_item_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        item: DisplayReplacementMediaSourceItem,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        self.append_item_request_to_text_row_and_emit(
            state,
            face_ids,
            item.append_request(position),
        )
    }

    #[cfg(test)]
    pub(crate) fn append_source_mapped_text_item_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        item: DisplayReplacementSourceMappedTextItem,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        self.append_item_request_to_text_row_and_emit(
            state,
            face_ids,
            item.append_request(position),
        )
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementAppendContext<'a> {
    single_item: SingleDisplayItemAppendContext<'a>,
}

impl<'a> DisplayReplacementAppendContext<'a> {
    pub(crate) fn new(
        face_id: FaceId,
        base_face: &'a ResolvedFace,
        frame: DisplayRowAppendFrame,
    ) -> Self {
        Self {
            single_item: SingleDisplayItemAppendContext::new(base_face, face_id, frame),
        }
    }

    fn append_replacement_item_plan_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        plan: DisplayReplacementItemAppendPlan,
    ) -> Option<DisplayRowAppendProgress> {
        let (item, position) = plan.into_parts();
        let mut source = DisplayItemOnceSource::new(item);
        let mut source_state = DisplayRowSourceState::default();
        let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
        let outcome = self.single_item.render_source_with_policy(
            state,
            face_ids,
            &mut source,
            &mut source_state,
            position,
            DisplayRowAppendKind::DisplayReplacement,
            &mut render_policy,
            self.single_item.face_id(),
        )?;
        Some(outcome.into_append_progress(position))
    }

    #[cfg(test)]
    pub(crate) fn append_replacement_item_kind_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        replacement_source: BufferDisplayReplacementSource,
        kind: DisplayItemKind,
        position: DisplayRowPosition,
    ) -> Option<DisplayRowAppendProgress> {
        let plan = DisplayReplacementItemAppendRequest::active_face(kind, position)
            .into_plan(replacement_source, self.single_item.face_id());
        self.append_replacement_item_plan_to_text_row_and_emit(state, face_ids, plan)
    }

    #[cfg(test)]
    pub(crate) fn append_replacement_string_source_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        face_ids: &mut FrameFaceAttempt,
        replacement_source: BufferDisplayReplacementSource,
        source_id: LispStringSourceId,
        value: Value,
        position: DisplayRowPosition,
        item_policy: &mut impl DisplayRowRenderPolicy,
    ) -> DisplayRowPosition {
        DisplayReplacementStringSourceAppendRequest::new(
            position,
            BufferDisplayReplacementStringRequest::new(source_id.raw(), value, replacement_source),
        )
        .render_to_text_row_and_emit(state, face_ids, self, item_policy)
        .position()
    }
}
