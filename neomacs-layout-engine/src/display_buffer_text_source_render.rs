//! Buffer text source consumption with replacement application.

use crate::display_buffer_display_property_render::{
    BufferDisplayPropertyTextReplacementOutcome, BufferDisplayPropertyTextReplacementRenderOutcome,
    BufferDisplayPropertyTextReplacementRenderState,
    BufferDisplayPropertyTextReplacementResolveRequest,
};
use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_face_resolution::BufferCurrentFaceResolutionContext;
use crate::display_buffer_text_loop_context::BufferTextWindowLoopRequestContext;
use crate::display_buffer_text_progress::BufferTextWindowProgressState;
use crate::display_buffer_text_source_consumption::BufferTextConsumedDisplayItem;
use crate::display_buffer_text_source_walk::BufferTextWindowSourceWalk;
use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BufferTextWindowSourceRenderOutcome {
    DisplayItem(BufferTextConsumedDisplayItem),
    ContinueBufferWalk,
    StopBufferWalk,
}

#[derive(Clone, Debug, PartialEq)]
enum BufferTextWindowSourceRenderItem {
    DisplayItem(BufferTextConsumedDisplayItem),
    Replacement(BufferTextReplacementItem),
}

pub(crate) struct BufferTextWindowSourceRenderRequest<'request, 'emit, 'surface, 'face> {
    loop_context: BufferTextWindowLoopRequestContext,
    text: &'request [u8],
    params: &'request WindowParams,
    active_face_state: &'face DisplayRowActiveFaceState,
    source_render: TextRowSourceRenderState<'emit>,
    face_ids: &'emit mut FrameFaceIdAllocator,
    append_surface: &'surface DisplayRowAppendSurface,
    row_geometry: &'emit mut DisplayRowGeometryState,
    cursor_info: &'emit mut CursorCaptureState,
    progress: BufferTextWindowProgressState<'emit>,
}

impl BufferTextWindowSourceRenderOutcome {
    pub(crate) fn should_continue_buffer_walk(&self) -> bool {
        matches!(self, Self::ContinueBufferWalk)
    }

    pub(crate) fn should_stop_buffer_walk(&self) -> bool {
        matches!(self, Self::StopBufferWalk)
    }
}

impl<'request, 'emit, 'surface, 'face>
    BufferTextWindowSourceRenderRequest<'request, 'emit, 'surface, 'face>
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        loop_context: BufferTextWindowLoopRequestContext,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &'face DisplayRowActiveFaceState,
        source_render: TextRowSourceRenderState<'emit>,
        face_ids: &'emit mut FrameFaceIdAllocator,
        append_surface: &'surface DisplayRowAppendSurface,
        row_geometry: &'emit mut DisplayRowGeometryState,
        cursor_info: &'emit mut CursorCaptureState,
        progress: BufferTextWindowProgressState<'emit>,
    ) -> Self {
        Self {
            loop_context,
            text,
            params,
            active_face_state,
            source_render,
            face_ids,
            append_surface,
            row_geometry,
            cursor_info,
            progress,
        }
    }

    pub(crate) fn consume_next<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        face_resolution_context: BufferCurrentFaceResolutionContext<'request, B>,
        buffer: &B,
    ) -> BufferTextWindowSourceRenderOutcome
    where
        'surface: 'request,
    {
        let Some(source_item) = source_walk.consume_source_item_for_render(
            &mut self.progress,
            face_resolution_context,
            self.face_ids,
            &mut self.source_render.reborrow(),
            self.row_geometry,
            BufferTextWindowSourceRenderItem::DisplayItem,
            BufferTextWindowSourceRenderItem::Replacement,
        ) else {
            return BufferTextWindowSourceRenderOutcome::StopBufferWalk;
        };

        match source_item {
            BufferTextWindowSourceRenderItem::DisplayItem(source_item) => {
                BufferTextWindowSourceRenderOutcome::DisplayItem(source_item)
            }
            BufferTextWindowSourceRenderItem::Replacement(replacement) => {
                self.consume_replacement(source_walk, replacement, buffer)
            }
        }
    }

    fn consume_replacement<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferTextWindowSourceWalk<'request, B>,
        replacement: BufferTextReplacementItem,
        buffer: &B,
    ) -> BufferTextWindowSourceRenderOutcome {
        let start_charpos = replacement.start_charpos();
        let request = BufferDisplayPropertyTextReplacementResolveRequest::new(
            replacement,
            self.loop_context.text_start_byte(),
            self.text,
            self.loop_context.content_x(),
            self.params,
            0.0,
            self.loop_context.char_height(),
            self.active_face_state,
        );
        let current_x = *self.progress.row.x;
        let start_position = self.progress.row_position();
        match request.resolve_and_render(
            buffer,
            BufferDisplayPropertyTextReplacementRenderState::new(
                self.source_render.reborrow(),
                self.face_ids,
                self.append_surface,
                self.row_geometry,
                self.active_face_state,
            ),
            current_x,
            start_position,
        ) {
            BufferDisplayPropertyTextReplacementRenderOutcome::Rendered(outcome) => {
                self.apply_replacement_outcome(outcome, start_charpos);
                BufferTextWindowSourceRenderOutcome::ContinueBufferWalk
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Fallback(source_item) => {
                let Some(source_step) = source_walk
                    .consume_fallback_source_item_for_render(source_item, &mut self.progress)
                else {
                    return BufferTextWindowSourceRenderOutcome::StopBufferWalk;
                };
                BufferTextWindowSourceRenderOutcome::DisplayItem(source_step)
            }
            BufferDisplayPropertyTextReplacementRenderOutcome::Stop => {
                BufferTextWindowSourceRenderOutcome::StopBufferWalk
            }
        }
    }

    fn apply_replacement_outcome(
        &mut self,
        outcome: BufferDisplayPropertyTextReplacementOutcome,
        start_charpos: i64,
    ) {
        outcome.capture_cursor_info_if_point(
            self.cursor_info,
            self.active_face_state,
            self.row_geometry,
            self.loop_context.point_charpos(),
            start_charpos,
            *self.progress.byte_idx,
        );
        let walk_update = outcome.walk_update(self.text, self.progress.source_position());
        self.progress.row.apply_position(walk_update.row_position());
        self.progress
            .apply_source_position(walk_update.source_position());
    }
}
