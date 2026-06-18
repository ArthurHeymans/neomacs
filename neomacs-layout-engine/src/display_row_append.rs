use crate::display_buffer_text_render::SyntheticTextSource;
use crate::display_face_id::FrameFaceIdAllocator;
#[cfg(test)]
use crate::display_row::DisplayRowRenderStop;
use crate::display_row::NaturalDisplayRowAppendRenderPolicy;
#[cfg(test)]
use crate::display_row::append_rendered_display_row_fragment_to_text_row_and_emit;
pub(crate) use crate::display_row_append_context::{
    DisplayRowActiveFaceAppendContext, DisplayRowAppendArea, DisplayRowAppendFrame,
    DisplayRowAppendKind, DisplayRowAppendMetrics, DisplayRowAppendPlacement,
    DisplayRowAppendSurface, DisplayRowTextAppendContext,
};
use crate::display_row_builder::{DisplayRowAppendProgress, DisplayRowPosition};
use crate::display_row_source_append::DisplayRowSourceAppendOperation;
use crate::display_row_source_render::TextRowSourceRenderState;
use crate::neovm_bridge::ResolvedFace;

pub(crate) fn append_synthetic_text_to_display_row(
    state: &mut TextRowSourceRenderState<'_>,
    base_face: &ResolvedFace,
    frame: DisplayRowAppendFrame,
    position: DisplayRowPosition,
    source: SyntheticTextSource,
    face_id: u32,
) -> Option<(DisplayRowAppendProgress, DisplayRowPosition)> {
    let source = source.into_item_source(face_id);
    let mut render_policy = NaturalDisplayRowAppendRenderPolicy;
    let start = position;
    let mut face_ids = FrameFaceIdAllocator::new(face_id.saturating_add(1));
    let outcome = DisplayRowSourceAppendOperation::new(
        base_face,
        face_id,
        frame,
        position,
        DisplayRowAppendKind::SourceText,
    )
    .render_source_to_text_row_and_emit(state, source, &mut face_ids, &mut render_policy)?;
    Some(outcome.into_append_progress_and_position(start))
}

#[cfg(test)]
#[path = "display_row_append_test.rs"]
mod tests;
