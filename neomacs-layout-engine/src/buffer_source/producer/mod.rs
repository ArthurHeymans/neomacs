//! The buffer element production nucleus.
//!
//! [`BufferElementProducer`] is the sole owner of the buffer-text cursor, the
//! display-source resolve state, and the consumption state (including the
//! pending split-remainder queue). Everything outside it — row assembly, the
//! append surface, overflow decisions — is renderer state and lives on
//! [`BufferSourceWalk`](crate::buffer_source::walk::BufferSourceWalk).
//!
//! Producer position is saved and reinstated as an opaque [`ProducerSnapshot`],
//! mirroring GNU's `SAVE_IT` / `RESTORE_IT`: the wrap retry does not merely
//! rewind a published buffer position, it reseats the whole producer.

pub(crate) mod vocabulary;

use crate::buffer_source::consumption::{BufferSourceConsumedItem, BufferSourceConsumptionState};
use crate::buffer_source::face_resolution::BufferSourceFaceResolutionContext;
use crate::buffer_source::text_source::BufferTextSourceCursor;
use crate::display_item::RenderFaceRef;
use crate::display_source::DisplaySourceTextPosition;
use crate::display_source::{DisplaySourceContext, DisplaySourceStepItem};
use crate::display_source_resolver::{
    BufferDisplaySourcePropertyResolver, DisplaySourceResolveState, PendingDisplaySourceFace,
};
use crate::display_spec::DisplayFringeLayout;
use crate::frame_face_arena::FrameFaceAttempt;
use crate::neovm_bridge::{LayoutBufferView, ResolvedFace};
use neomacs_display_protocol::types::FaceId;
use neovm_core::buffer::{BufferId, CharPos0};

/// An opaque save of the producer's whole seating: cursor position plus the
/// consumption state (its pending queue included). Restoring one reinstates the
/// producer exactly, which is what the wrap retry needs and what a bare
/// position rewind cannot express.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProducerSnapshot {
    cursor_char_pos: CharPos0,
    consumption: BufferSourceConsumptionState,
}

/// What one production step yielded: the element (if any), the walk position it
/// left behind, and the side effects the resolver collected while producing it.
pub(crate) struct ProducedStep {
    pub(crate) source_item: Option<BufferSourceConsumedItem>,
    pub(crate) source_position: DisplaySourceTextPosition,
    pub(crate) pending_faces: Vec<PendingDisplaySourceFace>,
    pub(crate) pending_fringes: Vec<DisplayFringeLayout>,
}

pub(crate) struct BufferElementProducer<'request, B: LayoutBufferView> {
    source_cursor: BufferTextSourceCursor<'request, B>,
    source_resolve_state: DisplaySourceResolveState,
    source_consumption: BufferSourceConsumptionState,
}

impl<'request, B: LayoutBufferView> BufferElementProducer<'request, B> {
    /// Producer with no window context (tests / non-window callers). The
    /// redisplay path uses [`new_for_window`](Self::new_for_window) so overlay
    /// `window` properties are honored.
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
        }
    }

    /// Save the producer's seating for a later [`restore`](Self::restore).
    pub(crate) fn snapshot(&self) -> ProducerSnapshot {
        ProducerSnapshot {
            cursor_char_pos: self.source_cursor.current_char_pos(),
            consumption: self.source_consumption.clone(),
        }
    }

    /// Reinstate a saved seating (GNU `RESTORE_IT`).
    pub(crate) fn restore(&mut self, snapshot: ProducerSnapshot) {
        let ProducerSnapshot {
            cursor_char_pos,
            consumption,
        } = snapshot;
        self.source_consumption = consumption;
        self.source_cursor.reset_to(cursor_char_pos);
    }

    /// The seating a retry at `source_position` starts from: that position with
    /// no queued remainders. Split remainders queued during the attempt describe
    /// positions *past* the retry point, so they are dropped.
    fn seating_at(&self, source_position: DisplaySourceTextPosition) -> ProducerSnapshot {
        ProducerSnapshot {
            cursor_char_pos: CharPos0::new(source_position.charpos().max(0) as usize),
            consumption: self.source_consumption.without_pending_render_items(),
        }
    }

    /// Reseat the producer at a row-wrap retry position so the current character
    /// is re-produced on the continuation row.
    ///
    /// During the overflow attempt the candidate char was already consumed: when
    /// its text run was split per-character, the remainder of the run was queued
    /// in the pending queue at a position *after* the candidate, and the cursor
    /// advanced past it. The word-wrap break rewinds `progress`/`source_position`
    /// to the candidate, but without this the next element produced is the stale
    /// pending remainder (candidate + 1), skipping the candidate char entirely
    /// (it stays drawn once on the previous row and is never re-rendered on the
    /// continuation row).
    ///
    /// This applies to both a word-wrap break candidate and character-wrap at a
    /// full row. It mirrors GNU's iterator restore (`RESTORE_IT` in
    /// `display_line`/`move_it_in_display_line_to`), which reseats the whole
    /// iterator — not just its published buffer position — at the retry point.
    pub(crate) fn rewind_to(&mut self, source_position: DisplaySourceTextPosition) {
        let seating = self.seating_at(source_position);
        self.restore(seating);
    }

    /// Consume only a PREFIX of the element just produced: reseat the cursor at
    /// `resume_charpos` so the next element begins there.
    ///
    /// The producer-side of GNU's `set_iterator_to_next` — the cursor position
    /// IS the resume state, so nothing is queued. It replaces the fit split,
    /// which rendered a fitting prefix and pushed the unrendered tail back into
    /// `pending_render_items` for the next loop iteration to pop.
    pub(crate) fn consume_prefix_to(&mut self, resume_charpos: i64) {
        debug_assert!(
            !self.has_pending_render_items(),
            "a prefix consume must not race queued remainders"
        );
        self.source_cursor
            .reset_to(CharPos0::new(resume_charpos.max(0) as usize));
    }

    /// Drop queued remainders without moving the cursor (the truncation skip,
    /// which advances the position itself).
    pub(crate) fn clear_pending_render_items(&mut self) {
        self.source_consumption.clear_pending_render_items();
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

    /// Produce the next element at `source_position`, resolving faces and
    /// fringe specs into the returned step.
    pub(crate) fn produce_step(
        &mut self,
        mut source_position: DisplaySourceTextPosition,
        face_resolution_context: BufferSourceFaceResolutionContext<'_, B>,
        face_ids: &mut FrameFaceAttempt,
    ) -> ProducedStep {
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
        ProducedStep {
            source_item,
            source_position,
            pending_faces,
            pending_fringes,
        }
    }

    /// Produce the next element against a caller-supplied source context, with
    /// no face resolver attached. Focused tests use this to observe the raw
    /// element stream.
    #[cfg(test)]
    pub(crate) fn next_consumed_item(
        &mut self,
        context: &mut DisplaySourceContext<'_>,
        position: &mut DisplaySourceTextPosition,
    ) -> Option<BufferSourceConsumedItem> {
        self.source_consumption.next_source_consumption_item(
            &mut self.source_cursor,
            context,
            position,
        )
    }

    /// Produce the next element with face resolution wired to `face_basis`,
    /// without the renderer-side context [`produce_step`](Self::produce_step)
    /// needs. The stream-equivalence harness drives the producer this way.
    #[cfg(test)]
    pub(crate) fn next_consumed_item_with_face_basis(
        &mut self,
        buffer: &B,
        face_basis: crate::display_source_resolver::DisplaySourceFaceBasis<'_>,
        face_ids: &mut FrameFaceAttempt,
        position: &mut DisplaySourceTextPosition,
    ) -> Option<BufferSourceConsumedItem> {
        let mut pending_faces = Vec::new();
        let mut pending_fringes = Vec::new();
        let params = crate::display_source_resolver::DisplaySourceResolveParams::new(
            face_basis,
            None,
            Default::default(),
        );
        let mut resolver = BufferDisplaySourcePropertyResolver::new(
            buffer,
            params,
            &mut self.source_resolve_state,
            face_ids,
            &mut pending_faces,
        );
        let mut context = DisplaySourceContext::with_face_resolver_and_fringe_sink(
            &mut resolver,
            &mut pending_fringes,
        );
        self.source_consumption.next_source_consumption_item(
            &mut self.source_cursor,
            &mut context,
            position,
        )
    }
}

#[cfg(test)]
#[path = "producer_test.rs"]
mod tests;

#[cfg(test)]
#[path = "stream_harness_test.rs"]
mod stream_harness_tests;
