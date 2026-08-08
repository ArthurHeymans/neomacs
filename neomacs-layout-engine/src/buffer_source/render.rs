//! Buffer text source consumption with replacement application.

use crate::buffer_source::consumption::BufferSourceConsumedItem;
use crate::buffer_source::display_property_render::{
    BufferDisplayPropertyTextReplacementApplyOutcome,
    BufferDisplayPropertyTextReplacementRenderContext,
    BufferDisplayPropertyTextReplacementRenderState,
};
use crate::buffer_source::face_resolution::BufferSourceFaceResolutionContext;
use crate::buffer_source::face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::buffer_source::item_render::BufferSourceItemRenderRequest;
use crate::buffer_source::loop_context::BufferSourceLoopRequestContext;
use crate::buffer_source::loop_state::BufferSourceLoopMutableState;
use crate::buffer_source::text_source::BufferOverlayStringsItem;
use crate::buffer_source::walk::BufferSourceWalk;
use crate::display_item::BufferDisplayPropertyReplacementItem;
use crate::display_row::face_state::DisplayRowActiveFaceState;
use crate::display_source::DisplaySourceStepChar;
use crate::display_source::DisplaySourceStepItem;
use crate::neovm_bridge::LayoutBufferView;
use crate::types::WindowParams;

pub(crate) struct BufferSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face> {
    loop_context: BufferSourceLoopRequestContext,
    text: &'request [u8],
    params: &'request WindowParams,
    active_face_state: &'face DisplayRowActiveFaceState,
    state: BufferSourceLoopMutableState<'rows, 'emit, 'surface>,
}

impl<'rows, 'request, 'emit, 'surface, 'face>
    BufferSourceRenderRequest<'rows, 'request, 'emit, 'surface, 'face>
{
    pub(crate) fn new(
        loop_context: BufferSourceLoopRequestContext,
        text: &'request [u8],
        params: &'request WindowParams,
        active_face_state: &'face DisplayRowActiveFaceState,
        state: BufferSourceLoopMutableState<'rows, 'emit, 'surface>,
    ) -> Self {
        Self {
            loop_context,
            text,
            params,
            active_face_state,
            state,
        }
    }

    pub(crate) fn render_next_and_apply<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        face_resolution_context: BufferSourceFaceResolutionContext<'request, B>,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let layout_resolution_context =
            face_resolution_context.source_item_layout_resolution_context();
        let Some(consumed_item) = source_walk.consume_source_item_for_render(
            &mut self.state.progress,
            face_resolution_context,
            self.state.face_ids,
            &mut self.state.source_render.reborrow(),
            self.state.row_build.row_geometry,
        ) else {
            return false;
        };

        match consumed_item {
            BufferSourceConsumedItem::DisplayPropertyReplacement(replacement) => self
                .consume_replacement(source_walk, layout_resolution_context, replacement, buffer),
            BufferSourceConsumedItem::Renderable(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
            BufferSourceConsumedItem::OverlayStrings(strings) => {
                self.render_overlay_strings(strings, buffer)
            }
        }
    }

    /// Append the overlay strings the producer anchored at this position.
    ///
    /// The producer decided WHERE they belong and in WHICH order (GNU
    /// `compare_overlay_entries`); this arm owns only the append, which stays a
    /// per-string session because a string can break rows, clip against the
    /// right edge and carry its own `cursor` property. The element has insertion
    /// semantics, so the walk position is untouched and the next production is
    /// the buffer character at the same anchor.
    fn render_overlay_strings<B: LayoutBufferView>(
        &mut self,
        strings: BufferOverlayStringsItem,
        buffer: &B,
    ) -> bool {
        let anchor_charpos = strings.anchor_charpos().get() as i64;
        let (x, col) = self.state.progress.row_progress_mut().coordinates_mut();
        let continuation = self
            .state
            .surface
            .overlay_context
            .render_produced_strings_at_text_row(
                buffer,
                anchor_charpos,
                strings.strings(),
                self.state.source_render.reborrow(),
                x,
                col,
                self.state.row_build.row_geometry,
                self.state.cursor_info,
                self.state.hit_capture.hit_rows,
                self.state.hit_capture.hit_row_range,
                self.state.row_y_positions,
                self.state.face_ids,
                self.state.row_carryover.line_numbers,
                self.state.face_scan,
            );
        !continuation.should_break()
    }

    fn consume_replacement<B: LayoutBufferView>(
        mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        replacement: BufferDisplayPropertyReplacementItem,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let replacement_context = BufferDisplayPropertyTextReplacementRenderContext::new(
            replacement,
            self.loop_context.text_start_byte(),
            self.text,
            self.loop_context.content_x(),
            self.params,
            0.0,
            self.loop_context.char_height(),
            self.active_face_state,
            self.state.progress.row_progress().x(),
            self.state.progress.row_position(),
        );
        match replacement_context.render_and_apply(
            buffer,
            BufferDisplayPropertyTextReplacementRenderState::new(
                self.state.source_render.reborrow(),
                self.state.face_ids,
                self.state.surface.append_surface,
                self.state.row_build.row_geometry,
                self.active_face_state,
            ),
            &mut self.state.progress,
            self.state.cursor_info,
            self.loop_context.point_charpos(),
        ) {
            BufferDisplayPropertyTextReplacementApplyOutcome::Applied { produced_row_break } => {
                if produced_row_break {
                    self.emit_display_string_row_break(source_walk, buffer)
                } else {
                    true
                }
            }
            BufferDisplayPropertyTextReplacementApplyOutcome::Fallback(source_item) => {
                self.render_source_item(source_walk, layout_resolution_context, source_item, buffer)
            }
            BufferDisplayPropertyTextReplacementApplyOutcome::Stop => false,
        }
    }

    /// A `display` string that ended in a newline terminated the current row;
    /// emit that row break so the buffer text after the covered region (which
    /// may be a bare newline that must still produce its own blank row) starts
    /// on a fresh row. Returns `false` when the break exhausted the window and
    /// the buffer walk must stop. GNU: xdisp.c `display_line` ends a display
    /// line on a display-string '\n'.
    fn emit_display_string_row_break<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        let synthetic_newline = DisplaySourceStepChar::new(
            '\n',
            self.state.progress.byte_idx(),
            self.state.progress.charpos(),
        );
        !self
            .loop_context
            .line_break_request(
                synthetic_newline,
                self.text,
                self.state.surface.append_surface,
                self.active_face_state,
            )
            .render_display_string_break_and_apply(source_walk, buffer, self.state.reborrow())
            .should_break()
    }

    fn render_source_item<B: LayoutBufferView>(
        &mut self,
        source_walk: &mut BufferSourceWalk<'request, B>,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'request>,
        source_item: DisplaySourceStepItem,
        buffer: &B,
    ) -> bool
    where
        'surface: 'request,
    {
        // A `SourceMappedText` standing in for a display-table entry that ends in
        // a newline glyph (whitespace-mode `[$ \n]`) renders its leading glyphs
        // and then ends the row — GNU treats the trailing `\n` element as its own
        // end-of-line display element. The buffer newline is already consumed by
        // this item's span, so break WITHOUT consuming another char.
        let break_after_row = source_item.item().layout.break_after_row;
        let keep_going = BufferSourceItemRenderRequest::from_loop_context(
            layout_resolution_context,
            self.loop_context,
            self.text,
            self.state.surface.append_surface,
            self.active_face_state,
            self.params,
        )
        .render_and_apply(source_item, source_walk, buffer, self.state.reborrow());
        if keep_going && break_after_row {
            return self.emit_display_string_row_break(source_walk, buffer);
        }
        keep_going
    }
}
