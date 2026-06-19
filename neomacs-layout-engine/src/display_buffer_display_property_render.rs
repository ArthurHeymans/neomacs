use crate::display_buffer_text_render::BufferTextWindowProgressState;
use crate::display_buffer_text_source::{BufferTextReplacementItem, BufferTextSourceItem};
use crate::display_cursor::{CapturedCursorInfo, CursorCaptureState, capture_cursor_info};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::RenderFaceRef;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_builder::DisplayRowPosition;
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowTextPosition};
use crate::display_row_replacement::DisplayPropertyReplacementAppendOutcome;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{TextPropertyScanCheckpoints, skip_text_to_charpos};
use crate::display_source_resolver::DisplayPropertyReplacementAppendRequestResolver;
use crate::neovm_bridge::{LayoutBufferView, RustTextPropAccess};
use crate::types::WindowParams;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferDisplayPropertyTextWalkOutcome {
    Continue,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderRequest<'a, B: LayoutBufferView> {
    context: BufferDisplayPropertyCheckpointRenderContext<'a, B>,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderContext<'a, B: LayoutBufferView> {
    pub(crate) buffer: &'a B,
    pub(crate) charpos: i64,
}

pub(crate) struct BufferDisplayPropertyCheckpointRenderState<'emit> {
    checkpoints: &'emit mut TextPropertyScanCheckpoints,
}

impl<'emit> BufferDisplayPropertyCheckpointRenderState<'emit> {
    pub(crate) fn new(checkpoints: &'emit mut TextPropertyScanCheckpoints) -> Self {
        Self { checkpoints }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferDisplayPropertyTextReplacementOutcome {
    pub(crate) replacement: DisplayPropertyReplacementAppendOutcome,
    pub(crate) skip_to: i64,
}

pub(crate) enum BufferDisplayPropertyTextReplacementRenderOutcome {
    Continue,
    Fallback(BufferTextSourceItem),
    Stop,
}

pub(crate) struct BufferDisplayPropertyTextReplacementRenderRequest<'a> {
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

pub(crate) struct BufferDisplayPropertyTextReplacementRenderState<'emit> {
    pub(crate) source_render: TextRowSourceRenderState<'emit>,
    pub(crate) face_ids: &'emit mut FrameFaceIdAllocator,
    pub(crate) append_surface: &'emit DisplayRowAppendSurface,
    pub(crate) row_geometry: &'emit mut DisplayRowGeometryState,
    pub(crate) cursor_info: &'emit mut CursorCaptureState,
    pub(crate) progress: BufferTextWindowProgressState<'emit>,
}

impl BufferDisplayPropertyTextWalkOutcome {
    pub(crate) fn should_continue_buffer_walk(self) -> bool {
        false
    }
}

impl<'a, B: LayoutBufferView> BufferDisplayPropertyCheckpointRenderRequest<'a, B> {
    pub(crate) fn new(context: BufferDisplayPropertyCheckpointRenderContext<'a, B>) -> Self {
        Self { context }
    }

    pub(crate) fn render_and_apply(
        self,
        state: BufferDisplayPropertyCheckpointRenderState<'_>,
    ) -> BufferDisplayPropertyTextWalkOutcome {
        let BufferDisplayPropertyCheckpointRenderState { checkpoints } = state;
        let context = self.context;
        if checkpoints.should_check_display(context.charpos) {
            let text_props = RustTextPropAccess::new(context.buffer);
            let (_, next_change) = text_props.check_display_prop(context.charpos);
            checkpoints.record_display_next(next_change);
        }
        BufferDisplayPropertyTextWalkOutcome::Continue
    }
}

impl<'a> BufferDisplayPropertyTextReplacementRenderRequest<'a> {
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

    pub(crate) fn render_and_apply<B: LayoutBufferView>(
        self,
        buffer: &B,
        state: BufferDisplayPropertyTextReplacementRenderState<'_>,
    ) -> BufferDisplayPropertyTextReplacementRenderOutcome {
        let BufferDisplayPropertyTextReplacementRenderState {
            mut source_render,
            face_ids,
            append_surface,
            row_geometry,
            cursor_info,
            progress,
        } = state;
        let Some(source_text) = self
            .replacement
            .source_text(self.text_start_byte, self.text)
        else {
            return BufferDisplayPropertyTextReplacementRenderOutcome::Stop;
        };
        let append_request =
            source_render.with_font_metrics_and_display_host(|font_metrics, host| {
                DisplayPropertyReplacementAppendRequestResolver::for_typed_replacement(
                    self.replacement.classification(),
                    self.replacement.replacement_source(),
                    self.replacement.value(),
                    self.replacement.start_charpos0(),
                    source_text,
                    self.active_face_state,
                    *progress.row.x,
                    self.content_x,
                    self.params,
                    self.glyph_y_offset,
                    self.default_row_height,
                    progress.row_position(),
                )
                .resolve(font_metrics, host)
            });
        let Some(request) = append_request else {
            let Some(source_item) = self.replacement.fallback_source_item(
                self.text_start_byte,
                self.text,
                RenderFaceRef::FaceId(self.active_face_state.face_id()),
            ) else {
                return BufferDisplayPropertyTextReplacementRenderOutcome::Stop;
            };
            return BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item);
        };
        let outcome = request.append_to_text_row(
            buffer,
            &mut source_render.reborrow(),
            face_ids,
            append_surface,
            row_geometry,
            self.active_face_state,
        );
        let replacement_outcome = BufferDisplayPropertyTextReplacementOutcome {
            replacement: outcome,
            skip_to: self.replacement.end_charpos(),
        };
        replacement_outcome.capture_cursor_info_if_point(
            cursor_info,
            self.active_face_state,
            row_geometry,
            self.point_charpos,
            self.replacement.start_charpos(),
            *progress.byte_idx,
        );
        replacement_outcome.apply_to_walk_state(
            self.text,
            progress.byte_idx,
            progress.charpos,
            progress.row.x,
            progress.row.col,
        );
        BufferDisplayPropertyTextReplacementRenderOutcome::Continue
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
