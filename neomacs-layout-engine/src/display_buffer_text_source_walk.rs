//! Buffer text source walking and source-position updates.
//!
//! The main buffer renderer owns orchestration, while this module owns
//! source cursor driving, pending face installation, and source-position
//! updates used by row lifecycle renderers.

use crate::display_buffer_display_property_source::BufferTextReplacementItem;
use crate::display_buffer_text_face_resolution::BufferCurrentFaceResolutionContext;
use crate::display_buffer_text_overflow::BufferTextTruncationSkipAction;
use crate::display_buffer_text_progress::BufferTextWindowProgressState;
use crate::display_buffer_text_row_lifecycle::{
    BufferHscrollSkipAction, BufferHscrollSkipSourceStep, BufferInvisibleTextScanAction,
    BufferInvisibleTextScanContext, BufferSelectiveDisplayContext,
    BufferSelectiveDisplayHiddenLines, BufferSelectiveDisplayLineTailAction,
};
use crate::display_buffer_text_source::{BufferTextSourceCursor, BufferTextSourcePosition};
use crate::display_buffer_text_source_consumption::{
    BufferTextConsumedDisplayItem, BufferTextSourceConsumptionState, BufferTextSourceItem,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_item::RenderFaceRef;
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::display_row_walk_state::{
    HorizontalScrollSkipState, InvisibleTextScanCheckpoint, LineNumberRenderState,
};
use crate::display_source::DisplaySourceContext;
use crate::display_source_resolver::{
    DisplaySourcePropertyResolver, DisplaySourceResolveState, PendingDisplaySourceFace,
};
use crate::neovm_bridge::LayoutBufferView;
use neovm_core::buffer::{BufferId, CharPos0};

pub(crate) struct BufferTextWindowSourceWalk<'request, B: LayoutBufferView> {
    source_cursor: BufferTextSourceCursor<'request, B>,
    source_resolve_state: DisplaySourceResolveState,
    source_consumption: BufferTextSourceConsumptionState,
}

struct BufferTextWindowSourceConsumption<R> {
    source_item: Option<R>,
    source_position: BufferTextSourcePosition,
    pending_faces: Vec<PendingDisplaySourceFace>,
}

struct BufferTextWindowFallbackSourceConsumption {
    source_item: Option<BufferTextConsumedDisplayItem>,
    source_position: BufferTextSourcePosition,
}

impl<R> BufferTextWindowSourceConsumption<R> {
    fn apply_to_progress(
        self,
        progress: &mut BufferTextWindowProgressState<'_>,
    ) -> (Option<R>, Vec<PendingDisplaySourceFace>) {
        progress.apply_source_position(self.source_position);
        (self.source_item, self.pending_faces)
    }

    fn apply_to_render_progress<B: LayoutBufferView>(
        self,
        progress: &mut BufferTextWindowProgressState<'_>,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
    ) -> Option<R> {
        let (source_item, pending_faces) = self.apply_to_progress(progress);
        face_resolution_context.install_pending_source_faces(
            source_render,
            row_geometry,
            pending_faces,
        );
        source_item
    }
}

impl BufferTextWindowFallbackSourceConsumption {
    fn apply_to_progress(
        self,
        progress: &mut BufferTextWindowProgressState<'_>,
    ) -> Option<BufferTextConsumedDisplayItem> {
        progress.apply_source_position(self.source_position);
        self.source_item
    }
}

impl<'request, B: LayoutBufferView> BufferTextWindowSourceWalk<'request, B> {
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
            source_consumption: BufferTextSourceConsumptionState::new(text_start_byte),
        }
    }

    fn consume_source_item<R>(
        &mut self,
        mut source_position: BufferTextSourcePosition,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
        face_ids: &mut FrameFaceIdAllocator,
        display_item: impl FnOnce(BufferTextConsumedDisplayItem) -> R,
        replacement: impl FnOnce(BufferTextReplacementItem) -> R,
    ) -> BufferTextWindowSourceConsumption<R> {
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
            self.source_consumption.next_source_consumption_result(
                &mut self.source_cursor,
                &mut source_context,
                &mut source_position,
                display_item,
                replacement,
            )
        };
        BufferTextWindowSourceConsumption {
            source_item,
            source_position,
            pending_faces,
        }
    }

    pub(crate) fn consume_source_item_for_render<R>(
        &mut self,
        progress: &mut BufferTextWindowProgressState<'_>,
        face_resolution_context: BufferCurrentFaceResolutionContext<'_, B>,
        face_ids: &mut FrameFaceIdAllocator,
        source_render: &mut TextRowSourceRenderState<'_>,
        row_geometry: &mut DisplayRowGeometryState,
        display_item: impl FnOnce(BufferTextConsumedDisplayItem) -> R,
        replacement: impl FnOnce(BufferTextReplacementItem) -> R,
    ) -> Option<R> {
        self.consume_source_item(
            progress.source_position(),
            face_resolution_context.clone(),
            face_ids,
            display_item,
            replacement,
        )
        .apply_to_render_progress(
            progress,
            face_resolution_context,
            source_render,
            row_geometry,
        )
    }

    fn consume_fallback_source_item(
        &mut self,
        source_item: BufferTextSourceItem,
        mut source_position: BufferTextSourcePosition,
    ) -> BufferTextWindowFallbackSourceConsumption {
        let source_item = self
            .source_consumption
            .consume_fallback_source_item(source_item, &mut source_position);
        BufferTextWindowFallbackSourceConsumption {
            source_item,
            source_position,
        }
    }

    pub(crate) fn consume_fallback_source_item_for_render(
        &mut self,
        source_item: BufferTextSourceItem,
        progress: &mut BufferTextWindowProgressState<'_>,
    ) -> Option<BufferTextConsumedDisplayItem> {
        self.consume_fallback_source_item(source_item, progress.source_position())
            .apply_to_progress(progress)
    }

    pub(crate) fn consume_hscroll_skip(
        &mut self,
        text: &[u8],
        source_position: BufferTextSourcePosition,
        hscroll_skip: &mut HorizontalScrollSkipState,
        tab_width: i32,
    ) -> BufferTextWindowSourcePositionConsumption<Option<BufferHscrollSkipAction>> {
        let mut source_position = source_position;
        let action = BufferHscrollSkipSourceStep::consume_from_position(
            text,
            &mut source_position,
            hscroll_skip,
            tab_width,
        );
        BufferTextWindowSourcePositionConsumption::new(action, source_position)
    }

    pub(crate) fn consume_invisible_checkpoint(
        &mut self,
        buffer: &B,
        context: BufferInvisibleTextScanContext<'_>,
        checkpoints: &mut InvisibleTextScanCheckpoint,
        source_position: BufferTextSourcePosition,
    ) -> BufferTextWindowSourcePositionConsumption<BufferInvisibleTextScanAction> {
        let mut source_position = source_position;
        let action = context.consume_at_checkpoint(buffer, checkpoints, &mut source_position);
        BufferTextWindowSourcePositionConsumption::new(action, source_position)
    }

    pub(crate) fn consume_selective_display_tail(
        &mut self,
        selective_display: BufferSelectiveDisplayContext<'_>,
        source_position: BufferTextSourcePosition,
    ) -> BufferTextWindowSourcePositionConsumption<BufferSelectiveDisplayLineTailAction> {
        let mut source_position = source_position;
        let action =
            selective_display.skip_rest_of_line_after_carriage_return(&mut source_position);
        BufferTextWindowSourcePositionConsumption::new(action, source_position)
    }

    pub(crate) fn consume_hidden_indented_lines_after_line_break(
        &mut self,
        selective_display: BufferSelectiveDisplayContext<'_>,
        source_position: BufferTextSourcePosition,
        line_numbers: &mut LineNumberRenderState,
    ) -> BufferTextWindowSourcePositionConsumption<BufferSelectiveDisplayHiddenLines> {
        let mut source_position = source_position;
        let hidden_lines = selective_display
            .apply_hidden_indented_lines_after_line_break(&mut source_position, line_numbers);
        BufferTextWindowSourcePositionConsumption::new(hidden_lines, source_position)
    }

    pub(crate) fn consume_truncation_skip(
        &mut self,
        text: &[u8],
        source_position: BufferTextSourcePosition,
    ) -> BufferTextWindowSourcePositionConsumption<BufferTextTruncationSkipAction> {
        let mut source_position = source_position;
        let action = BufferTextTruncationSkipAction::consume_source_step_char_and_rest_of_line(
            text,
            &mut source_position,
        );
        BufferTextWindowSourcePositionConsumption::new(action, source_position)
    }

    pub(crate) fn source_position_update(
        &mut self,
        source_position: BufferTextSourcePosition,
    ) -> BufferTextWindowSourcePositionConsumption<()> {
        BufferTextWindowSourcePositionConsumption::new((), source_position)
    }
}

pub(crate) struct BufferTextWindowSourcePositionConsumption<T> {
    value: T,
    source_position: BufferTextSourcePosition,
}

impl<T> BufferTextWindowSourcePositionConsumption<T> {
    fn new(value: T, source_position: BufferTextSourcePosition) -> Self {
        Self {
            value,
            source_position,
        }
    }

    pub(crate) fn apply_to_progress(self, progress: &mut BufferTextWindowProgressState<'_>) -> T {
        progress.apply_source_position(self.source_position);
        self.value
    }
}
