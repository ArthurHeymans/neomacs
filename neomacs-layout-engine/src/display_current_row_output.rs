use crate::composition::last_text_cluster_tail_in_row;
use crate::display_output_builder::DisplayOutputBuilder;
#[cfg(test)]
use crate::display_output_install_request::OutputFrameStateInstallRequest;
pub(crate) use crate::display_output_row_request::DisplayCurrentRowMutation;
#[cfg(test)]
use crate::display_rendered_row_output_install::install_rendered_display_row_fragment_assets;
#[cfg(test)]
use crate::display_row_builder::DisplayRowPosition;
#[cfg(test)]
use crate::display_row_render_state::RenderedDisplayRow;
#[cfg(test)]
use crate::display_row_text_output::TextRowOutput;
#[cfg(test)]
use crate::window_output::WindowOutputEmitter;
use neomacs_display_protocol::glyph_matrix::GlyphRow;
#[cfg(test)]
use neovm_core::emacs_core::Context;

pub(crate) struct DisplayRowCurrentRowOutput<'builder> {
    builder: &'builder mut DisplayOutputBuilder,
}

#[cfg(test)]
struct AppendRenderedDisplayRowFragmentMutation<'a> {
    rendered: &'a RenderedDisplayRow,
}

#[cfg(test)]
impl DisplayCurrentRowMutation for AppendRenderedDisplayRowFragmentMutation<'_> {
    type Output = DisplayRowPosition;

    fn apply(self, row: &mut GlyphRow) -> Self::Output {
        self.rendered.append_fragment_to_current_row(row)
    }
}

impl<'builder> DisplayRowCurrentRowOutput<'builder> {
    pub(crate) fn from_output_builder(builder: &'builder mut DisplayOutputBuilder) -> Self {
        Self { builder }
    }

    pub(crate) fn reborrow(&mut self) -> DisplayRowCurrentRowOutput<'_> {
        DisplayRowCurrentRowOutput {
            builder: self.builder,
        }
    }

    fn current_row_snapshot(&self) -> Option<GlyphRow> {
        self.builder.current_row_for_render().cloned()
    }

    pub(crate) fn apply_current_row_mutation<M>(&mut self, mutation: M) -> Option<M::Output>
    where
        M: DisplayCurrentRowMutation,
    {
        self.builder.apply_current_output_row_mutation(mutation)
    }

    pub(crate) fn apply_current_row_scratch_mutation<M>(&self, mutation: M) -> Option<M::Output>
    where
        M: DisplayCurrentRowMutation,
    {
        let mut row = self.current_row_snapshot()?;
        Some(mutation.apply(&mut row))
    }

    #[cfg(test)]
    fn append_rendered_fragment(
        &mut self,
        rendered: &RenderedDisplayRow,
    ) -> Option<DisplayRowPosition> {
        self.builder
            .apply_current_output_row_mutation(AppendRenderedDisplayRowFragmentMutation {
                rendered,
            })
    }

    pub(crate) fn cluster_tail(&self) -> Option<(char, bool)> {
        self.current_row_snapshot()
            .as_ref()
            .and_then(last_text_cluster_tail_in_row)
    }
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_current_row(
    builder: &mut DisplayOutputBuilder,
    rendered: &RenderedDisplayRow,
    _display_row_index: usize,
) -> DisplayRowPosition {
    for face in rendered.faces() {
        builder.install_output_frame_state(OutputFrameStateInstallRequest::face(
            face.id,
            face.clone(),
        ));
    }
    let end = DisplayRowCurrentRowOutput::from_output_builder(builder)
        .append_rendered_fragment(rendered)
        .expect("current row");
    install_rendered_display_row_fragment_assets(builder, &[]);
    end
}

#[cfg(test)]
pub(crate) fn append_rendered_display_row_fragment_to_text_row_and_emit(
    builder: &mut DisplayOutputBuilder,
    output_emitter: &mut WindowOutputEmitter,
    evaluator: &mut Context,
    rendered: &RenderedDisplayRow,
    output: TextRowOutput,
) -> DisplayRowPosition {
    let end = append_rendered_display_row_fragment_to_current_row(builder, rendered, output.row());
    output_emitter.emit_text_output_spans(
        evaluator,
        output,
        output.spans_for_source_slots(rendered.source_slots()),
        end,
    );
    end
}
