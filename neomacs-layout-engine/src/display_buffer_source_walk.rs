//! Buffer text source walking and source-position updates.
//!
//! The main buffer renderer owns orchestration, while this module owns
//! source cursor driving, pending face installation, and source-position
//! updates used by row lifecycle renderers.

use crate::display_buffer_source_consumption::{
    BufferSourceConsumedItem, BufferSourceConsumptionState,
};
use crate::display_buffer_source_face_resolution::BufferSourceFaceResolutionContext;
use crate::display_buffer_source_overflow::BufferSourceTruncationSkipAction;
use crate::display_buffer_source_row_lifecycle::{
    BufferSourceHscrollSkipAction, BufferSourceInvisibleTextScanAction,
    BufferSourceInvisibleTextScanContext, BufferSourceSelectiveDisplayContext,
    BufferSourceSelectiveDisplayHiddenLines, BufferSourceSelectiveDisplayLineTailAction,
    consume_hscroll_skip_from_position,
};
use crate::display_buffer_text_source::BufferTextSourceCursor;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::RenderFaceRef;
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{
    HorizontalScrollSkipState, InvisibleTextScanCheckpoint, LineNumberRenderState,
};
use crate::display_source::DisplaySourceTextPosition;
use crate::display_source::{DisplaySourceContext, DisplaySourceStepItem};
use crate::display_source_item_append::DisplaySourceRowAppendState;
use crate::display_source_progress::DisplaySourceProgressState;
use crate::display_source_resolver::{DisplaySourcePropertyResolver, DisplaySourceResolveState};
use crate::display_source_walk::DisplaySourcePositionConsumption;
use crate::neovm_bridge::LayoutBufferView;
use neovm_core::buffer::{BufferId, CharPos0};

pub(crate) struct BufferSourceWalk<'request, B: LayoutBufferView> {
    source_cursor: BufferTextSourceCursor<'request, B>,
    source_resolve_state: DisplaySourceResolveState,
    source_consumption: BufferSourceConsumptionState,
    append_state: DisplaySourceRowAppendState,
}

struct BufferSourceWalkConsumption {
    source_item: Option<BufferSourceConsumedItem>,
    source_position: DisplaySourceTextPosition,
    pending_faces: Vec<crate::display_source_resolver::PendingDisplaySourceFace>,
}

impl BufferSourceWalkConsumption {
    fn new(
        source_item: Option<BufferSourceConsumedItem>,
        source_position: DisplaySourceTextPosition,
        pending_faces: Vec<crate::display_source_resolver::PendingDisplaySourceFace>,
    ) -> Self {
        Self {
            source_item,
            source_position,
            pending_faces,
        }
    }

    fn apply_to_progress(
        self,
        progress: &mut DisplaySourceProgressState<'_>,
    ) -> (
        Option<BufferSourceConsumedItem>,
        Vec<crate::display_source_resolver::PendingDisplaySourceFace>,
    ) {
        if self.source_item.is_none() {
            progress.apply_source_position(self.source_position);
        }
        (self.source_item, self.pending_faces)
    }

    fn apply_to_render_progress<B: LayoutBufferView>(
        self,
        progress: &mut DisplaySourceProgressState<'_>,
        face_resolution_context: BufferSourceFaceResolutionContext<'_, B>,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> Option<BufferSourceConsumedItem> {
        let (source_item, pending_faces) = self.apply_to_progress(progress);
        face_resolution_context.install_pending_source_faces(
            source_render,
            row_geometry,
            pending_faces,
        );
        source_item
    }
}

impl<'request, B: LayoutBufferView> BufferSourceWalk<'request, B> {
    pub(crate) fn new(
        buffer_id: BufferId,
        buffer: &'request B,
        start_charpos: i64,
        text_start_byte: usize,
    ) -> Self {
        Self {
            source_cursor: BufferTextSourceCursor::new(
                buffer_id,
                buffer,
                CharPos0::new(start_charpos.max(0) as usize),
                CharPos0::new(usize::MAX),
                RenderFaceRef::Inherit,
            ),
            source_resolve_state: DisplaySourceResolveState::default(),
            source_consumption: BufferSourceConsumptionState::new(text_start_byte),
            append_state: DisplaySourceRowAppendState::default(),
        }
    }

    pub(crate) fn append_state(&mut self) -> &mut DisplaySourceRowAppendState {
        &mut self.append_state
    }

    pub(crate) fn prepend_pending_render_items<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = DisplaySourceStepItem>,
    {
        self.source_consumption.prepend_pending_render_items(items);
    }

    /// Rewind the source consumption + cursor state to a word-wrap break
    /// candidate so the candidate char (the word boundary) is re-produced on the
    /// continuation row.
    ///
    /// During the overflow attempt the candidate char was already consumed: when
    /// its text run was split per-character, the remainder of the run was queued
    /// in `pending_render_items` at a position *after* the candidate, and the
    /// `BufferTextSourceCursor` advanced past it. The word-wrap break rewinds
    /// `progress`/`source_position` to the candidate, but without this the next
    /// source item produced is the stale pending remainder (candidate + 1),
    /// skipping the candidate char entirely (it stays drawn once on the previous
    /// row and is never re-rendered on the continuation row).
    ///
    /// This mirrors GNU's word-wrap rewind (`it = it_before_word`, RESTORE_IT in
    /// `display_line`/`move_it_in_display_line_to`), which reseats the whole
    /// iterator — not just its buffer position — back to the word boundary.
    /// Clearing the pending queue drops the stale remainder; reseating the cursor
    /// to the candidate's char position makes the next consumption re-read the
    /// candidate char from the buffer.
    pub(crate) fn rewind_source_consumption_to(
        &mut self,
        source_position: DisplaySourceTextPosition,
    ) {
        self.source_consumption.clear_pending_render_items();
        self.source_cursor
            .reset_to(CharPos0::new(source_position.charpos().max(0) as usize));
    }

    fn consume_source_item(
        &mut self,
        mut source_position: DisplaySourceTextPosition,
        face_resolution_context: BufferSourceFaceResolutionContext<'_, B>,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> BufferSourceWalkConsumption {
        let mut pending_faces = Vec::new();
        let source_item = {
            let params = face_resolution_context.source_resolve_params(None);
            let mut resolver = DisplaySourcePropertyResolver::new(
                params,
                &mut self.source_resolve_state,
                face_ids,
                &mut pending_faces,
            );
            let mut source_context = DisplaySourceContext::with_face_resolver(&mut resolver);
            self.source_consumption.next_source_consumption_item(
                &mut self.source_cursor,
                &mut source_context,
                &mut source_position,
            )
        };
        BufferSourceWalkConsumption::new(source_item, source_position, pending_faces)
    }

    pub(crate) fn consume_source_item_for_render(
        &mut self,
        progress: &mut DisplaySourceProgressState<'_>,
        face_resolution_context: BufferSourceFaceResolutionContext<'_, B>,
        face_ids: &mut FrameFaceIdAllocator,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> Option<BufferSourceConsumedItem> {
        self.consume_source_item(
            progress.source_position(),
            face_resolution_context.clone(),
            face_ids,
        )
        .apply_to_render_progress(
            progress,
            face_resolution_context,
            source_render,
            row_geometry,
        )
    }

    pub(crate) fn consume_hscroll_skip(
        &mut self,
        text: &[u8],
        source_position: DisplaySourceTextPosition,
        hscroll_skip: &mut HorizontalScrollSkipState,
        tab_width: i32,
    ) -> DisplaySourcePositionConsumption<Option<BufferSourceHscrollSkipAction>> {
        let mut source_position = source_position;
        let action =
            consume_hscroll_skip_from_position(text, &mut source_position, hscroll_skip, tab_width);
        DisplaySourcePositionConsumption::new(action, source_position)
    }

    pub(crate) fn consume_invisible_checkpoint(
        &mut self,
        buffer: &B,
        context: BufferSourceInvisibleTextScanContext<'_>,
        checkpoints: &mut InvisibleTextScanCheckpoint,
        source_position: DisplaySourceTextPosition,
    ) -> DisplaySourcePositionConsumption<BufferSourceInvisibleTextScanAction> {
        let mut source_position = source_position;
        let action = context.consume_at_checkpoint(buffer, checkpoints, &mut source_position);
        DisplaySourcePositionConsumption::new(action, source_position)
    }

    pub(crate) fn consume_selective_display_tail(
        &mut self,
        selective_display: BufferSourceSelectiveDisplayContext<'_>,
        source_position: DisplaySourceTextPosition,
    ) -> DisplaySourcePositionConsumption<BufferSourceSelectiveDisplayLineTailAction> {
        let mut source_position = source_position;
        let action =
            selective_display.skip_rest_of_line_after_carriage_return(&mut source_position);
        DisplaySourcePositionConsumption::new(action, source_position)
    }

    pub(crate) fn consume_hidden_indented_lines_after_line_break(
        &mut self,
        selective_display: BufferSourceSelectiveDisplayContext<'_>,
        source_position: DisplaySourceTextPosition,
        line_numbers: &mut LineNumberRenderState,
    ) -> DisplaySourcePositionConsumption<BufferSourceSelectiveDisplayHiddenLines> {
        let mut source_position = source_position;
        let hidden_lines = selective_display
            .apply_hidden_indented_lines_after_line_break(&mut source_position, line_numbers);
        DisplaySourcePositionConsumption::new(hidden_lines, source_position)
    }

    pub(crate) fn consume_truncation_skip(
        &mut self,
        text: &[u8],
        source_position: DisplaySourceTextPosition,
    ) -> DisplaySourcePositionConsumption<BufferSourceTruncationSkipAction> {
        self.source_consumption.clear_pending_render_items();
        let mut source_position = source_position;
        let action = BufferSourceTruncationSkipAction::consume_source_step_char_and_rest_of_line(
            text,
            &mut source_position,
        );
        DisplaySourcePositionConsumption::new(action, source_position)
    }

    pub(crate) fn source_position_update(
        &mut self,
        source_position: DisplaySourceTextPosition,
    ) -> DisplaySourcePositionConsumption<()> {
        DisplaySourcePositionConsumption::new((), source_position)
    }
}
