use crate::display_cursor::{CapturedCursorInfo, CursorCaptureState, capture_cursor_info};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::{BufferDisplayPropertyReplacementItem, RenderFaceRef};
use crate::display_row::{DisplayRowActiveFaceState, DisplayRowFallbackMetrics};
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowTextPosition};
use crate::display_row_replacement::{
    DisplayPropertyReplacementAppendOutcome, DisplayPropertyReplacementRowRenderRequest,
};
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_source::DisplaySourceItem;
use crate::display_source::DisplaySourceTextPosition;
use crate::display_source_progress::DisplaySourceProgressState;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) replacement: DisplayPropertyReplacementAppendOutcome,
    pub(crate) skip_to: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementWalkUpdate {
    row_position: DisplayRowPosition,
    source_position: DisplaySourceTextPosition,
}

pub(crate) enum BufferDisplayPropertyTextReplacementRenderOutcome {
    Rendered(BufferDisplayPropertyTextReplacementOutcome),
    Fallback(DisplaySourceItem),
    Stop,
}

pub(crate) struct BufferDisplayPropertyTextReplacementRenderRequest<'a, 'face> {
    replacement: BufferDisplayPropertyReplacementItem,
    text_start_byte: usize,
    text: &'a [u8],
    content_x: f32,
    params: &'a WindowParams,
    glyph_y_offset: f32,
    fallback_metrics: DisplayRowFallbackMetrics,
    active_face_state: &'face DisplayRowActiveFaceState,
}

pub(crate) struct BufferDisplayPropertyTextReplacementRenderState<'emit> {
    source_render: TextRowSourceRenderState<'emit>,
    face_ids: &'emit mut FrameFaceIdAllocator,
    append_surface: &'emit DisplayRowAppendSurface,
    row_geometry: &'emit mut DisplayRowGeometryState,
    active_face_state: &'emit DisplayRowActiveFaceState,
}

impl<'emit> BufferDisplayPropertyTextReplacementRenderState<'emit> {
    pub(crate) fn new(
        source_render: TextRowSourceRenderState<'emit>,
        face_ids: &'emit mut FrameFaceIdAllocator,
        append_surface: &'emit DisplayRowAppendSurface,
        row_geometry: &'emit mut DisplayRowGeometryState,
        active_face_state: &'emit DisplayRowActiveFaceState,
    ) -> Self {
        Self {
            source_render,
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        }
    }
}

impl<'a, 'face> BufferDisplayPropertyTextReplacementRenderRequest<'a, 'face> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        replacement: BufferDisplayPropertyReplacementItem,
        text_start_byte: usize,
        text: &'a [u8],
        content_x: f32,
        params: &'a WindowParams,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        active_face_state: &'face DisplayRowActiveFaceState,
    ) -> Self {
        Self {
            replacement,
            text_start_byte,
            text,
            content_x,
            params,
            glyph_y_offset,
            fallback_metrics,
            active_face_state,
        }
    }

    fn fallback_render_item(&self) -> Option<DisplaySourceItem> {
        let fallback = self.replacement.fallback_display_item(
            self.text_start_byte,
            self.text,
            RenderFaceRef::FaceId(self.active_face_state.face_id()),
        )?;
        let (item, start_byte_idx, start_charpos, source_char) = fallback.into_parts();
        Some(DisplaySourceItem::new(
            item,
            start_byte_idx,
            start_charpos,
            source_char,
        ))
    }

    fn render_append_request<B: LayoutBufferView>(
        &self,
        append_request: DisplayPropertyReplacementRowRenderRequest,
        buffer: &B,
        state: BufferDisplayPropertyTextReplacementRenderState<'_>,
    ) -> BufferDisplayPropertyTextReplacementOutcome {
        let BufferDisplayPropertyTextReplacementRenderState {
            mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        } = state;
        let outcome = append_request.render_to_text_row(
            buffer,
            &mut source_render.reborrow(),
            face_ids,
            append_surface,
            row_geometry,
            active_face_state,
        );
        BufferDisplayPropertyTextReplacementOutcome {
            replacement: outcome,
            skip_to: self.replacement.descriptor().skip_to_charpos(),
        }
    }

    pub(crate) fn render<B: LayoutBufferView>(
        self,
        buffer: &B,
        mut state: BufferDisplayPropertyTextReplacementRenderState<'_>,
        current_x: f32,
        start_position: DisplayRowPosition,
    ) -> BufferDisplayPropertyTextReplacementRenderOutcome {
        let Some(source_text) = self
            .replacement
            .source_text(self.text_start_byte, self.text)
        else {
            return BufferDisplayPropertyTextReplacementRenderOutcome::Stop;
        };
        let descriptor = self.replacement.descriptor();
        let append_request =
            state
                .source_render
                .with_font_metrics_and_display_host(|font_metrics, host| {
                    DisplayPropertyReplacementRowRenderRequest::from_typed_replacement_descriptor(
                        descriptor,
                        source_text,
                        self.active_face_state,
                        font_metrics,
                        current_x,
                        self.content_x,
                        self.params,
                        host,
                        self.glyph_y_offset,
                        self.fallback_metrics,
                        start_position,
                    )
                });
        match append_request {
            Some(request) => BufferDisplayPropertyTextReplacementRenderOutcome::Rendered(
                self.render_append_request(request, buffer, state),
            ),
            None => {
                let Some(source_item) = self.fallback_render_item() else {
                    return BufferDisplayPropertyTextReplacementRenderOutcome::Stop;
                };
                BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item)
            }
        }
    }
}

impl BufferDisplayPropertyTextReplacementOutcome {
    fn point_in_replacement(self, point_charpos: i64, start_charpos: i64) -> bool {
        point_charpos >= start_charpos && point_charpos < self.skip_to
    }

    fn start_position(self) -> DisplayRowPosition {
        self.replacement.start_position()
    }

    fn end_position(self) -> DisplayRowPosition {
        self.replacement.end_position()
    }

    fn skip_covered_buffer_text(self, text: &[u8], position: &mut DisplaySourceTextPosition) {
        position.skip_chars_until(text, self.skip_to);
    }

    fn capture_cursor_info_if_point(
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

    fn walk_update(
        self,
        text: &[u8],
        mut source_position: DisplaySourceTextPosition,
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

    pub(crate) fn apply_to_progress_and_cursor(
        self,
        text: &[u8],
        progress: &mut DisplaySourceProgressState<'_>,
        cursor_info: &mut CursorCaptureState,
        active_face_state: &DisplayRowActiveFaceState,
        row_geometry: &DisplayRowGeometryState,
        point_charpos: i64,
        start_charpos: i64,
    ) {
        self.capture_cursor_info_if_point(
            cursor_info,
            active_face_state,
            row_geometry,
            point_charpos,
            start_charpos,
            *progress.byte_idx,
        );
        let walk_update = self.walk_update(text, progress.source_position());
        progress.row.apply_position(walk_update.row_position());
        progress.apply_source_position(walk_update.source_position());
    }
}

impl BufferDisplayPropertyTextReplacementWalkUpdate {
    pub(crate) fn new(
        row_position: DisplayRowPosition,
        source_position: DisplaySourceTextPosition,
    ) -> Self {
        Self {
            row_position,
            source_position,
        }
    }

    pub(crate) fn row_position(self) -> DisplayRowPosition {
        self.row_position
    }

    pub(crate) fn source_position(self) -> DisplaySourceTextPosition {
        self.source_position
    }
}
