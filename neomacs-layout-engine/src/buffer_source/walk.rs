//! Buffer text source walking and source-position updates.
//!
//! The main buffer renderer owns orchestration, while this module owns
//! source cursor driving, pending face installation, and source-position
//! updates used by row lifecycle renderers.

use crate::buffer_source::consumption::{BufferSourceConsumedItem, BufferSourceConsumptionState};
use crate::buffer_source::face_resolution::BufferSourceFaceResolutionContext;
use crate::buffer_source::overflow::BufferSourceTruncationSkipAction;
use crate::buffer_source::row_lifecycle::{
    BufferSourceHscrollSkipAction, BufferSourceInvisibleTextScanAction,
    BufferSourceInvisibleTextScanContext, BufferSourceSelectiveDisplayContext,
    BufferSourceSelectiveDisplayHiddenLines, BufferSourceSelectiveDisplayLineTailAction,
    consume_hscroll_skip_from_position,
};
use crate::buffer_source::text_source::BufferTextSourceCursor;
use crate::display_item::RenderFaceRef;
use crate::display_row::geometry::DisplayRowGeometryState;
use crate::display_row::source_render::TextRowSourceRenderState;
use crate::display_row::walk_state::{
    HorizontalScrollSkipState, InvisibleTextScanCheckpoint, LineNumberRenderState,
};
use crate::display_source::DisplaySourceTextPosition;
use crate::display_source::{DisplaySourceContext, DisplaySourceStepItem};
use crate::display_source_item_append::DisplaySourceRowAppendState;
use crate::display_source_progress::DisplaySourceProgressState;
use crate::display_source_resolver::{
    BufferDisplaySourcePropertyResolver, DisplaySourceResolveState,
};
use crate::display_source_walk::DisplaySourcePositionConsumption;
use crate::frame_face_arena::FrameFaceAttempt;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace};
use neomacs_display_protocol::types::FaceId;
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
    pending_fringes: Vec<crate::display_spec::DisplayFringeLayout>,
}

impl BufferSourceWalkConsumption {
    fn new(
        source_item: Option<BufferSourceConsumedItem>,
        source_position: DisplaySourceTextPosition,
        pending_faces: Vec<crate::display_source_resolver::PendingDisplaySourceFace>,
        pending_fringes: Vec<crate::display_spec::DisplayFringeLayout>,
    ) -> Self {
        Self {
            source_item,
            source_position,
            pending_faces,
            pending_fringes,
        }
    }

    fn apply_to_progress(
        self,
        progress: &mut DisplaySourceProgressState<'_>,
    ) -> (
        Option<BufferSourceConsumedItem>,
        Vec<crate::display_source_resolver::PendingDisplaySourceFace>,
        Vec<crate::display_spec::DisplayFringeLayout>,
    ) {
        if self.source_item.is_none() {
            progress.apply_source_position(self.source_position);
        }
        (self.source_item, self.pending_faces, self.pending_fringes)
    }

    fn apply_to_render_progress<B: LayoutBufferView>(
        self,
        progress: &mut DisplaySourceProgressState<'_>,
        face_resolution_context: BufferSourceFaceResolutionContext<'_, B>,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        face_ids: &mut FrameFaceAttempt,
    ) -> Option<BufferSourceConsumedItem> {
        let (source_item, pending_faces, pending_fringes) = self.apply_to_progress(progress);
        face_resolution_context.install_pending_source_faces(
            source_render,
            row_geometry,
            pending_faces,
        );
        // Record any `(left-fringe …)` / `(right-fringe …)` specs the buffer-text
        // walk collected on the current row (zero inline width was already
        // applied; the bitmap draws in the fringe column). The fallback face is
        // only used when neither a `set-fringe-bitmap-face` override nor the
        // spec's FACE resolves (magit always supplies a FACE).
        let fallback_face_id = FaceId::from(neomacs_display_protocol::face::BasicFaceId::Default);
        for layout in &pending_fringes {
            source_render.record_fringe_bitmap_layout(layout, face_ids, fallback_face_id);
        }
        source_item
    }
}

impl<'request, B: LayoutBufferView> BufferSourceWalk<'request, B> {
    /// Walk with no window context (tests / non-window callers). The redisplay
    /// path uses [`new_for_window`](Self::new_for_window) so overlay `window`
    /// properties are honored.
    #[allow(dead_code)] // retained for non-window callers and focused tests
    pub(crate) fn new(
        buffer_id: BufferId,
        buffer: &'request B,
        start_charpos: i64,
        text_start_byte: usize,
    ) -> Self {
        Self::new_for_window(buffer_id, buffer, None, start_charpos, text_start_byte)
    }

    pub(crate) fn new_for_window(
        buffer_id: BufferId,
        buffer: &'request B,
        window_id: Option<u64>,
        start_charpos: i64,
        text_start_byte: usize,
    ) -> Self {
        Self {
            source_cursor: BufferTextSourceCursor::new_for_window(
                buffer_id,
                buffer,
                window_id,
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

    pub(crate) fn resolved_source_face(&self, face_id: FaceId) -> Option<&ResolvedFace> {
        self.source_resolve_state.resolved_face(face_id)
    }

    pub(crate) fn remember_resolved_source_face_if_absent(
        &mut self,
        face_id: FaceId,
        face: &ResolvedFace,
    ) {
        if self.source_resolve_state.resolved_face(face_id).is_none() {
            self.source_resolve_state.remember_face(face_id, face);
        }
    }

    /// Whether split-run remainders are queued for re-consumption. The routed
    /// ascii-row acquisition path refuses to bypass the walk while any are
    /// pending (they always describe an in-progress, non-plain row anyway).
    pub(crate) fn has_pending_render_items(&self) -> bool {
        self.source_consumption.has_pending_render_items()
    }

    /// Telemetry-only view: how many split-run remainders are queued.
    pub(crate) fn pending_render_items_len(&self) -> usize {
        self.source_consumption.pending_render_items_len()
    }

    pub(crate) fn prepend_pending_render_items<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = DisplaySourceStepItem>,
    {
        self.source_consumption.prepend_pending_render_items(items);
    }

    /// Rewind source consumption and its cursor to a row-wrap retry position so
    /// the current character is re-produced on the continuation row.
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
    /// This applies to both a word-wrap break candidate and character-wrap at a
    /// full row. It mirrors GNU's iterator restore (`RESTORE_IT` in
    /// `display_line`/`move_it_in_display_line_to`), which reseats the whole
    /// iterator — not just its published buffer position — at the retry point.
    /// Clearing the pending queue drops the stale run remainder; reseating the
    /// cursor makes the next consumption re-read the rejected character.
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
        face_ids: &mut FrameFaceAttempt,
    ) -> BufferSourceWalkConsumption {
        let mut pending_faces = Vec::new();
        let mut pending_fringes = Vec::new();
        let source_item = {
            let params = face_resolution_context.source_resolve_params(None);
            let mut resolver = BufferDisplaySourcePropertyResolver::new(
                face_resolution_context.buffer(),
                params,
                &mut self.source_resolve_state,
                face_ids,
                &mut pending_faces,
            );
            let mut source_context = DisplaySourceContext::with_face_resolver_and_fringe_sink(
                &mut resolver,
                &mut pending_fringes,
            );
            self.source_consumption.next_source_consumption_item(
                &mut self.source_cursor,
                &mut source_context,
                &mut source_position,
            )
        };
        BufferSourceWalkConsumption::new(
            source_item,
            source_position,
            pending_faces,
            pending_fringes,
        )
    }

    pub(crate) fn consume_source_item_for_render(
        &mut self,
        progress: &mut DisplaySourceProgressState<'_>,
        face_resolution_context: BufferSourceFaceResolutionContext<'_, B>,
        face_ids: &mut FrameFaceAttempt,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> Option<BufferSourceConsumedItem> {
        let consumption = self.consume_source_item(
            progress.source_position(),
            face_resolution_context,
            face_ids,
        );
        consumption.apply_to_render_progress(
            progress,
            face_resolution_context,
            source_render,
            row_geometry,
            face_ids,
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
