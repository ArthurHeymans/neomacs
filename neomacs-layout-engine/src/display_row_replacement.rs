use crate::display_cursor::{CapturedCursorInfo, display_property_replacement_cursor_info};
use crate::display_face_id::FrameFaceIdAllocator;
#[cfg(test)]
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{DisplayItem, DisplayItemKind};
#[cfg(test)]
use crate::display_origin::DisplayOrigin;
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowRenderClipBehavior, DisplayRowRenderPolicy,
    DisplayRowSourceState,
};
use crate::display_row_append_context::{
    DisplayRowAppendFrame, DisplayRowAppendKind, DisplayRowAppendMetrics,
    DisplayRowAppendPlacement, DisplayRowAppendSurface,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowItemMeasurement, DisplayRowPosition,
};
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowTextPosition};
#[cfg(test)]
use crate::display_row_lisp_string::LispStringSourceId;
use crate::display_row_source_append::{
    DisplayItemSourceAppendRequest, SingleDisplayItemSourceRequest,
    render_display_item_source_with_policy, render_single_display_item_naturally,
};
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_source::{
    BufferDisplayReplacementSource, BufferDisplayReplacementStringRequest,
    DisplayPropertyReplacementCursorPolicy, DisplayPropertyReplacementSourceItem,
    DisplayReplacementMediaSourceItem, DisplayReplacementMediaSourceResolution,
    DisplayReplacementSourceMappedTextItem, DisplayReplacementStretchSourceItem,
    DisplayReplacementStringSourceItem,
};
use crate::display_source_resolver::DisplayStringBaseFace;
use crate::font_metrics::FontMetricsService;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace};
#[cfg(test)]
use neovm_core::emacs_core::Value;

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
pub(crate) struct DisplayReplacementStringSourceAppendRequest {
    pub(crate) position: DisplayRowPosition,
    source: BufferDisplayReplacementStringRequest,
}

impl DisplayReplacementStringSourceAppendRequest {
    pub(crate) fn new(
        position: DisplayRowPosition,
        source: BufferDisplayReplacementStringRequest,
    ) -> Self {
        Self { position, source }
    }

    fn position(self) -> DisplayRowPosition {
        self.position
    }

    #[cfg(test)]
    pub(crate) fn source_id(self) -> LispStringSourceId {
        LispStringSourceId(self.source.source_id())
    }

    #[cfg(test)]
    pub(crate) fn value(self) -> Value {
        self.source.value()
    }

    pub(crate) fn render_to_text_row_and_emit(
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
        let request = DisplayItemSourceAppendRequest::new(
            append_context.base_face,
            append_context.face_id,
            &append_context.frame,
            position,
            DisplayRowAppendKind::DisplayReplacementString,
        );
        let mut source = source;
        let mut source_state = DisplayRowSourceState::default();
        let Some(outcome) = render_display_item_source_with_policy(
            state,
            face_ids,
            &mut source,
            &mut source_state,
            request,
            &mut render_policy,
        ) else {
            return position;
        };
        outcome.end_position()
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementStringAppendRequest {
    item: DisplayReplacementStringSourceItem,
    pub(crate) replacement_base_face: Option<DisplayStringBaseFace>,
    active_face_state: DisplayRowActiveFaceState,
}

impl DisplayReplacementStringAppendRequest {
    pub(crate) fn new(
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
    pub(crate) fn origin(&self) -> DisplayOrigin {
        self.item.origin()
    }

    #[cfg(test)]
    pub(crate) fn base_face_policy(&self) -> BaseFacePolicy {
        self.item.base_face_policy()
    }

    pub(crate) fn string_item_measurer(&self) -> DisplayReplacementStringItemMeasurer {
        DisplayReplacementStringItemMeasurer {
            active_face_state: self.active_face_state.clone(),
        }
    }

    pub(crate) fn source_append_request(
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
pub(crate) struct DisplayReplacementItemAppendRequest {
    kind: DisplayItemKind,
    frame: DisplayReplacementItemAppendFrame,
    position: DisplayRowPosition,
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayReplacementItemAppendPlan {
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
    pub(crate) fn active_face(kind: DisplayItemKind, position: DisplayRowPosition) -> Self {
        Self {
            kind,
            frame: DisplayReplacementItemAppendFrame::ActiveFace,
            position,
        }
    }

    #[cfg(test)]
    pub(crate) fn display_box(
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
        }
    }

    pub(crate) fn into_plan(
        self,
        replacement_source: BufferDisplayReplacementSource,
        face_id: u32,
    ) -> DisplayReplacementItemAppendPlan {
        DisplayReplacementItemAppendPlan {
            item: replacement_source.display_item(face_id, self.kind),
            frame: self.frame,
            position: self.position,
        }
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

    fn into_request(self, position: DisplayRowPosition) -> DisplayReplacementItemAppendRequest {
        DisplayReplacementItemAppendRequest {
            kind: self.kind,
            frame: self.frame,
            position,
        }
    }

    fn append_to_text_row(
        self,
        replacement_append_context: DisplayReplacementRowAppendContext<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        state: &mut TextRowSourceRenderState<'_>,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        let geometry_update = self.row_geometry_update;
        if let DisplayReplacementItemRowGeometryUpdate::BeforeAppendGlyphMetrics {
            height_px,
            ascent_px,
        } = geometry_update
        {
            row_geometry.include_glyph_vertical_metrics(height_px, ascent_px);
        }
        let Some(progress) = replacement_append_context
            .append_item_request_to_text_row_and_emit(state, self.into_request(position))
        else {
            return position;
        };
        if let DisplayReplacementItemRowGeometryUpdate::AfterCompleteRowExtents {
            height_px,
            ascent_px,
        } = geometry_update
            && progress.status == DisplayRowAppendStatus::Complete
            && progress.metrics.width_px > 0.0
        {
            row_geometry.include_row_extents(height_px, ascent_px);
        }
        progress.end
    }
}

impl DisplayReplacementStretchSourceItem {
    #[cfg(test)]
    pub(crate) fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> Option<DisplayReplacementItemAppendRequest> {
        (self.width_px() > 0.0).then(|| {
            DisplayReplacementItemAppendRequest::active_face(self.display_item_kind(), position)
        })
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
        let plan = self.into_plan(buffer, state, active_face_state, face_ids);
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
    pub(crate) start_position: DisplayRowPosition,
    pub(crate) end_position: DisplayRowPosition,
    pub(crate) cursor_policy: DisplayPropertyReplacementCursorPolicy,
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
        face_ids: &mut FrameFaceIdAllocator,
        position: DisplayRowPosition,
    ) -> DisplayRowPosition {
        match self {
            Self::Empty => position,
            Self::String(request) => {
                request.append_to_text_row(replacement_append_context, state, face_ids, position)
            }
            Self::Item(item) => {
                item.append_to_text_row(replacement_append_context, row_geometry, state, position)
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
        if progress.status == DisplayRowAppendStatus::Complete && progress.metrics.width_px > 0.0 {
            Some((self.display_height_px(), self.display_ascent_px()))
        } else {
            None
        }
    }

    #[cfg(test)]
    pub(crate) fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> DisplayReplacementItemAppendRequest {
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
    pub(crate) fn append_request(
        self,
        position: DisplayRowPosition,
    ) -> DisplayReplacementItemAppendRequest {
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
        DisplayReplacementAppendContext::new(face_id, base_face, self.active_face_frame())
    }

    fn full_text_width_active_face(
        self,
        face_id: u32,
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
        face_id: u32,
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

    pub(crate) fn append_item_request_to_text_row_and_emit(
        self,
        state: &mut TextRowSourceRenderState<'_>,
        request: DisplayReplacementItemAppendRequest,
    ) -> Option<DisplayRowAppendProgress> {
        let plan = request.into_plan(self.replacement_source, self.active_face.face_id());
        let append_context = match plan.frame {
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
        append_context.append_replacement_item_plan_to_text_row_and_emit(state, plan)
    }
}

#[derive(Clone)]
pub(crate) struct DisplayReplacementAppendContext<'a> {
    face_id: u32,
    base_face: &'a ResolvedFace,
    frame: DisplayRowAppendFrame,
}

impl<'a> DisplayReplacementAppendContext<'a> {
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

    pub(crate) fn append_replacement_item_plan_to_text_row_and_emit(
        &self,
        state: &mut TextRowSourceRenderState<'_>,
        plan: DisplayReplacementItemAppendPlan,
    ) -> Option<DisplayRowAppendProgress> {
        let request = SingleDisplayItemSourceRequest::new(
            self.base_face,
            self.face_id,
            &self.frame,
            plan.item,
            plan.position,
            DisplayRowAppendKind::DisplayReplacement,
        );
        render_single_display_item_naturally(state, request)
    }
}
