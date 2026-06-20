use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_progress::BufferTextWindowProgressState;
use crate::display_buffer_text_source::{
    BufferTextSourceItem, BufferTextSourcePosition, BufferTextSourceStepChar,
};
use crate::display_cursor::{CapturedCursorInfo, CursorCaptureState, capture_cursor_info};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::RenderFaceRef;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowTextPosition};
use crate::display_row_replacement::{
    DisplayPropertyReplacementAppendOutcome, DisplayPropertyReplacementAppendRequest,
};
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_source_resolver::DisplayPropertyReplacementAppendRequestResolver;
use crate::font_metrics::FontMetricsService;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;
use neovm_core::emacs_core::eval::DisplayHost;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) replacement: DisplayPropertyReplacementAppendOutcome,
    pub(crate) skip_to: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementWalkUpdate {
    row_position: DisplayRowPosition,
    source_position: BufferTextSourcePosition,
}

pub(crate) enum BufferDisplayPropertyTextReplacementResolveOutcome {
    Resolved(BufferDisplayPropertyTextReplacementRenderRequest),
    Fallback(BufferTextSourceItem),
    Stop,
}

pub(crate) struct BufferDisplayPropertyTextReplacementResolveRequest<'a> {
    replacement: BufferTextReplacementItem,
    text_start_byte: usize,
    text: &'a [u8],
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    default_row_height: f32,
    active_face_state: &'a DisplayRowActiveFaceState,
    point_charpos: i64,
}

pub(crate) struct BufferDisplayPropertyTextReplacementRenderRequest {
    append_request: DisplayPropertyReplacementAppendRequest,
    skip_to: i64,
    start_charpos: i64,
    point_charpos: i64,
}

pub(crate) struct BufferDisplayPropertyTextReplacementRenderState<'emit> {
    text: &'emit [u8],
    source_render: TextRowSourceRenderState<'emit>,
    face_ids: &'emit mut FrameFaceIdAllocator,
    append_surface: &'emit DisplayRowAppendSurface,
    row_geometry: &'emit mut DisplayRowGeometryState,
    cursor_info: &'emit mut CursorCaptureState,
    active_face_state: &'emit DisplayRowActiveFaceState,
    progress: BufferTextWindowProgressState<'emit>,
}

impl<'emit> BufferDisplayPropertyTextReplacementRenderState<'emit> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        text: &'emit [u8],
        source_render: TextRowSourceRenderState<'emit>,
        face_ids: &'emit mut FrameFaceIdAllocator,
        append_surface: &'emit DisplayRowAppendSurface,
        row_geometry: &'emit mut DisplayRowGeometryState,
        cursor_info: &'emit mut CursorCaptureState,
        active_face_state: &'emit DisplayRowActiveFaceState,
        progress: BufferTextWindowProgressState<'emit>,
    ) -> Self {
        Self {
            text,
            source_render,
            face_ids,
            append_surface,
            row_geometry,
            cursor_info,
            active_face_state,
            progress,
        }
    }
}

impl<'a> BufferDisplayPropertyTextReplacementResolveRequest<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        replacement: BufferTextReplacementItem,
        text_start_byte: usize,
        text: &'a [u8],
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        default_row_height: f32,
        active_face_state: &'a DisplayRowActiveFaceState,
        point_charpos: i64,
    ) -> Self {
        Self {
            replacement,
            text_start_byte,
            text,
            content_x,
            params,
            glyph_y_offset,
            default_row_height,
            active_face_state,
            point_charpos,
        }
    }

    pub(crate) fn resolve(
        self,
        font_metrics: &mut Option<FontMetricsService>,
        display_host: Option<&dyn DisplayHost>,
        current_x: f32,
        start_position: DisplayRowPosition,
    ) -> BufferDisplayPropertyTextReplacementResolveOutcome {
        let Some(source_text) = self
            .replacement
            .source_text(self.text_start_byte, self.text)
        else {
            return BufferDisplayPropertyTextReplacementResolveOutcome::Stop;
        };
        let append_request =
            DisplayPropertyReplacementAppendRequestResolver::for_typed_replacement(
                self.replacement.classification(),
                self.replacement.replacement_source(),
                self.replacement.value(),
                self.replacement.start_charpos0(),
                source_text,
                self.active_face_state,
                current_x,
                self.content_x,
                self.params,
                self.glyph_y_offset,
                self.default_row_height,
                start_position,
            )
            .resolve(font_metrics, display_host);
        let Some(request) = append_request else {
            let Some(source_item) = self.replacement.fallback_source_item(
                self.text_start_byte,
                self.text,
                RenderFaceRef::FaceId(self.active_face_state.face_id()),
            ) else {
                return BufferDisplayPropertyTextReplacementResolveOutcome::Stop;
            };
            return BufferDisplayPropertyTextReplacementResolveOutcome::Fallback(source_item);
        };
        BufferDisplayPropertyTextReplacementResolveOutcome::Resolved(
            BufferDisplayPropertyTextReplacementRenderRequest::new(
                request,
                self.replacement.end_charpos(),
                self.replacement.start_charpos(),
                self.point_charpos,
            ),
        )
    }
}

impl BufferDisplayPropertyTextReplacementRenderRequest {
    pub(crate) fn new(
        append_request: DisplayPropertyReplacementAppendRequest,
        skip_to: i64,
        start_charpos: i64,
        point_charpos: i64,
    ) -> Self {
        Self {
            append_request,
            skip_to,
            start_charpos,
            point_charpos,
        }
    }

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferDisplayPropertyTextReplacementRenderState<'_>,
    ) -> BufferDisplayPropertyTextReplacementWalkUpdate {
        let BufferDisplayPropertyTextReplacementRenderState {
            text,
            mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            cursor_info,
            active_face_state,
            mut progress,
        } = state;
        let outcome = self.append_request.append_to_text_row(
            buffer,
            &mut source_render.reborrow(),
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        );
        let replacement_outcome = BufferDisplayPropertyTextReplacementOutcome {
            replacement: outcome,
            skip_to: self.skip_to,
        };
        replacement_outcome.capture_cursor_info_if_point(
            cursor_info,
            active_face_state,
            row_geometry,
            self.point_charpos,
            self.start_charpos,
            *progress.byte_idx,
        );
        let walk_update = replacement_outcome.walk_update(text, progress.source_position());
        progress.row.apply_position(walk_update.row_position());
        walk_update
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
        position: &mut BufferTextSourcePosition,
    ) {
        while position.charpos() < self.skip_to && position.byte_idx() < text.len() {
            if BufferTextSourceStepChar::consume_from_position(text, position).is_none() {
                break;
            }
        }
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

    pub(crate) fn walk_update(
        self,
        text: &[u8],
        mut source_position: BufferTextSourcePosition,
    ) -> BufferDisplayPropertyTextReplacementWalkUpdate {
        self.skip_covered_buffer_text(text, &mut source_position);
        BufferDisplayPropertyTextReplacementWalkUpdate::new(self.end_position(), source_position)
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

impl BufferDisplayPropertyTextReplacementWalkUpdate {
    pub(crate) fn new(
        row_position: DisplayRowPosition,
        source_position: BufferTextSourcePosition,
    ) -> Self {
        Self {
            row_position,
            source_position,
        }
    }

    pub(crate) fn row_position(self) -> DisplayRowPosition {
        self.row_position
    }

    pub(crate) fn source_position(self) -> BufferTextSourcePosition {
        self.source_position
    }
}
