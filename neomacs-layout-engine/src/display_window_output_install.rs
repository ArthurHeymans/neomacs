use crate::display_output_builder::{
    DisplayOutputBuilder, OutputRetryCheckpointRestoreRequest,
    OutputTextWindowDisplayRangeInstallRequest,
};
use crate::window_output::{
    TextWindowDisplayRange, TextWindowOutputBegin, TextWindowOutputRetryCheckpoint,
};

pub(crate) struct WindowOutputInstallSurface<'output> {
    output_builder: &'output mut DisplayOutputBuilder,
}

pub(crate) struct WindowOutputReadSurface<'output> {
    output_builder: &'output DisplayOutputBuilder,
}

impl<'output> WindowOutputInstallSurface<'output> {
    pub(crate) fn from_output_builder(output_builder: &'output mut DisplayOutputBuilder) -> Self {
        Self { output_builder }
    }

    pub(crate) fn begin_text_window_output(&mut self, request: TextWindowOutputBegin) {
        self.output_builder.window_installer().begin(
            request.window_id,
            request.rows,
            request.cols,
            request.bounds,
            request.text_bounds,
            request.selected,
        );
    }

    pub(crate) fn close_text_window_output(&mut self) {
        self.output_builder.window_installer().end();
    }

    pub(crate) fn record_display_range(&mut self, range: TextWindowDisplayRange) {
        self.output_builder.install_window_metadata(
            OutputTextWindowDisplayRangeInstallRequest::new(
                range.window_id as i64,
                range.window_start.as_i64(),
                range.window_end.as_i64(),
            ),
        );
    }

    pub(crate) fn restore_retry_checkpoint(&mut self, checkpoint: TextWindowOutputRetryCheckpoint) {
        self.output_builder
            .install_window_metadata(OutputRetryCheckpointRestoreRequest::new(
                checkpoint.transition_hints_len,
                checkpoint.effect_hints_len,
            ));
    }
}

impl<'output> WindowOutputReadSurface<'output> {
    pub(crate) fn from_output_builder(output_builder: &'output DisplayOutputBuilder) -> Self {
        Self { output_builder }
    }

    pub(crate) fn capture_retry_checkpoint(&self) -> TextWindowOutputRetryCheckpoint {
        TextWindowOutputRetryCheckpoint {
            transition_hints_len: self.output_builder.transition_hints().len(),
            effect_hints_len: self.output_builder.effect_hints().len(),
        }
    }
}
