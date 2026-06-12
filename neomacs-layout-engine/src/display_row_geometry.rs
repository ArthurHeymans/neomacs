use crate::hit_test::HitRow;
use crate::window_output::{
    TextMatrixRowBegin, TextMatrixRowGeometryTransition, TextMatrixRowMetrics,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CurrentDisplayRowMetrics {
    height: f32,
    ascent: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisplayRowAdvanceKind {
    LineBreak { line_spacing: f32 },
    Truncation,
    VisualWrap,
}

impl DisplayRowAdvanceKind {
    fn line_spacing(self) -> f32 {
        match self {
            Self::LineBreak { line_spacing } => line_spacing,
            Self::Truncation | Self::VisualWrap => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CurrentDisplayRowAdvance {
    pub(crate) y: f32,
    pub(crate) next_row: usize,
    pub(crate) text_y: f32,
    pub(crate) row_extra_y: f32,
    pub(crate) default_height: f32,
    pub(crate) default_ascent: f32,
    pub(crate) kind: DisplayRowAdvanceKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowAdvance {
    pub(crate) finished: TextMatrixRowMetrics,
    pub(crate) next_y: f32,
    pub(crate) row_extra_y: f32,
    pub(crate) next_height: f32,
    pub(crate) next_ascent: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowGeometryDefaults {
    pub(crate) text_y: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowGeometryCursor {
    row: usize,
    y: f32,
    row_extra_y: f32,
    metrics: CurrentDisplayRowMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LegacyDisplayRowGeometry {
    pub(crate) row: usize,
    pub(crate) y: f32,
    pub(crate) row_extra_y: f32,
    pub(crate) row_max_height: f32,
    pub(crate) row_max_ascent: f32,
}

pub(crate) struct LegacyDisplayRowGeometryVars<'a> {
    pub(crate) row: &'a mut usize,
    pub(crate) y: &'a mut f32,
    pub(crate) row_extra_y: &'a mut f32,
    pub(crate) row_max_height: &'a mut f32,
    pub(crate) row_max_ascent: &'a mut f32,
}

impl LegacyDisplayRowGeometryVars<'_> {
    pub(crate) fn snapshot(&self) -> LegacyDisplayRowGeometry {
        LegacyDisplayRowGeometry {
            row: *self.row,
            y: *self.y,
            row_extra_y: *self.row_extra_y,
            row_max_height: *self.row_max_height,
            row_max_ascent: *self.row_max_ascent,
        }
    }

    pub(crate) fn apply(&mut self, state: DisplayRowGeometryState) {
        *self.row = state.row;
        *self.y = state.y;
        *self.row_extra_y = state.row_extra_y;
        *self.row_max_height = state.height;
        *self.row_max_ascent = state.ascent;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowGeometryState {
    pub(crate) row: usize,
    pub(crate) y: f32,
    pub(crate) row_extra_y: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
}

enum DisplayRowYRecorder<'a> {
    None,
    RowYPositions(&'a mut Vec<f32>),
}

pub(crate) struct DisplayRowGeometryCommitTarget<'a> {
    vars: LegacyDisplayRowGeometryVars<'a>,
    row_y_recorder: DisplayRowYRecorder<'a>,
}

pub(crate) struct DisplayRowGeometryAdvanceTarget<'a> {
    defaults: DisplayRowGeometryDefaults,
    kind: DisplayRowAdvanceKind,
    row_base: usize,
    col: usize,
    x: f32,
    commit_target: DisplayRowGeometryCommitTarget<'a>,
}

impl DisplayRowYRecorder<'_> {
    fn record(self, y: f32) {
        match self {
            Self::None => {}
            Self::RowYPositions(row_y_positions) => row_y_positions.push(y),
        }
    }
}

impl<'a> DisplayRowGeometryCommitTarget<'a> {
    pub(crate) fn silent(vars: LegacyDisplayRowGeometryVars<'a>) -> Self {
        Self {
            vars,
            row_y_recorder: DisplayRowYRecorder::None,
        }
    }

    pub(crate) fn recording_row_y(
        vars: LegacyDisplayRowGeometryVars<'a>,
        row_y_positions: &'a mut Vec<f32>,
    ) -> Self {
        Self {
            vars,
            row_y_recorder: DisplayRowYRecorder::RowYPositions(row_y_positions),
        }
    }
}

impl<'a> DisplayRowGeometryAdvanceTarget<'a> {
    fn new(
        defaults: DisplayRowGeometryDefaults,
        kind: DisplayRowAdvanceKind,
        row_base: usize,
        col: usize,
        x: f32,
        commit_target: DisplayRowGeometryCommitTarget<'a>,
    ) -> Self {
        Self {
            defaults,
            kind,
            row_base,
            col,
            x,
            commit_target,
        }
    }

    pub(crate) fn line_break(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        line_spacing: f32,
        commit_target: DisplayRowGeometryCommitTarget<'a>,
    ) -> Self {
        Self::new(
            defaults,
            DisplayRowAdvanceKind::LineBreak { line_spacing },
            row_base,
            col,
            x,
            commit_target,
        )
    }

    pub(crate) fn truncation(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        commit_target: DisplayRowGeometryCommitTarget<'a>,
    ) -> Self {
        Self::new(
            defaults,
            DisplayRowAdvanceKind::Truncation,
            row_base,
            col,
            x,
            commit_target,
        )
    }

    pub(crate) fn visual_wrap(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        commit_target: DisplayRowGeometryCommitTarget<'a>,
    ) -> Self {
        Self::new(
            defaults,
            DisplayRowAdvanceKind::VisualWrap,
            row_base,
            col,
            x,
            commit_target,
        )
    }
}

impl DisplayRowGeometryState {
    pub(crate) fn from_legacy(legacy: LegacyDisplayRowGeometry) -> Self {
        Self {
            row: legacy.row,
            y: legacy.y,
            row_extra_y: legacy.row_extra_y,
            height: legacy.row_max_height,
            ascent: legacy.row_max_ascent,
        }
    }
}

impl CurrentDisplayRowMetrics {
    pub(crate) fn new(height: f32, ascent: f32) -> Self {
        Self { height, ascent }
    }

    pub(crate) fn height(&self) -> f32 {
        self.height
    }

    pub(crate) fn ascent(&self) -> f32 {
        self.ascent
    }

    pub(crate) fn include_glyph(&mut self, glyph_height: f32, glyph_ascent: f32) {
        let glyph_height = glyph_height.max(1.0);
        let glyph_ascent = glyph_ascent.max(0.0).min(glyph_height);
        let row_descent = (self.height - self.ascent).max(0.0);
        let glyph_descent = (glyph_height - glyph_ascent).max(0.0);
        self.ascent = self.ascent.max(glyph_ascent);
        self.height = (self.ascent + row_descent.max(glyph_descent)).max(glyph_height);
    }

    pub(crate) fn extra_height_over_default(&self, default_height: f32) -> f32 {
        (self.height - default_height).max(0.0)
    }

    pub(crate) fn next_row_vertical_delta(&self, default_height: f32, line_spacing: f32) -> f32 {
        self.extra_height_over_default(default_height) + line_spacing.max(0.0)
    }

    pub(crate) fn finish_current_row(&self, y: f32) -> TextMatrixRowMetrics {
        TextMatrixRowMetrics {
            y,
            height: self.height,
            ascent: self.ascent,
        }
    }

    pub(crate) fn reset(&mut self, height: f32, ascent: f32) {
        self.height = height;
        self.ascent = ascent;
    }

    pub(crate) fn finish_and_reset(
        &mut self,
        y: f32,
        default_height: f32,
        default_ascent: f32,
    ) -> TextMatrixRowMetrics {
        let finished = self.finish_current_row(y);
        self.reset(default_height, default_ascent);
        finished
    }

    pub(crate) fn finish_and_advance_to_next_row(
        &mut self,
        advance: CurrentDisplayRowAdvance,
    ) -> DisplayRowAdvance {
        let row_extra_y = advance.row_extra_y
            + self.next_row_vertical_delta(advance.default_height, advance.kind.line_spacing());
        let finished =
            self.finish_and_reset(advance.y, advance.default_height, advance.default_ascent);
        DisplayRowAdvance {
            finished,
            next_y: advance.text_y + advance.next_row as f32 * advance.default_height + row_extra_y,
            row_extra_y,
            next_height: self.height(),
            next_ascent: self.ascent(),
        }
    }
}

impl DisplayRowGeometryCursor {
    pub(crate) fn from_state(state: DisplayRowGeometryState) -> Self {
        Self {
            row: state.row,
            y: state.y,
            row_extra_y: state.row_extra_y,
            metrics: CurrentDisplayRowMetrics::new(state.height, state.ascent),
        }
    }

    pub(crate) fn from_legacy_vars(vars: LegacyDisplayRowGeometryVars<'_>) -> Self {
        Self::from_state(DisplayRowGeometryState::from_legacy(vars.snapshot()))
    }

    pub(crate) fn commit(&self, mut target: DisplayRowGeometryCommitTarget<'_>) {
        let state = self.state();
        target.vars.apply(state);
        target.row_y_recorder.record(state.y);
    }

    pub(crate) fn hit_row(&self, charpos_start: i64, charpos_end: i64) -> HitRow {
        HitRow {
            y_start: self.y,
            y_end: self.y + self.metrics.height(),
            charpos_start,
            charpos_end,
        }
    }

    pub(crate) fn finish_current_row(&self) -> TextMatrixRowMetrics {
        self.metrics.finish_current_row(self.y)
    }

    pub(crate) fn finish_and_advance_to_next_row(
        &mut self,
        defaults: DisplayRowGeometryDefaults,
        kind: DisplayRowAdvanceKind,
    ) -> TextMatrixRowMetrics {
        let row_advance = self
            .metrics
            .finish_and_advance_to_next_row(CurrentDisplayRowAdvance {
                y: self.y,
                next_row: self.row + 1,
                text_y: defaults.text_y,
                row_extra_y: self.row_extra_y,
                default_height: defaults.height,
                default_ascent: defaults.ascent,
                kind,
            });
        self.row += 1;
        self.y = row_advance.next_y;
        self.row_extra_y = row_advance.row_extra_y;
        self.metrics =
            CurrentDisplayRowMetrics::new(row_advance.next_height, row_advance.next_ascent);
        row_advance.finished
    }

    pub(crate) fn finish_and_begin_next_text_matrix_row(
        &mut self,
        defaults: DisplayRowGeometryDefaults,
        kind: DisplayRowAdvanceKind,
        row_base: usize,
        col: usize,
        x: f32,
    ) -> TextMatrixRowGeometryTransition {
        let finished_row = self.finish_and_advance_to_next_row(defaults, kind);
        let begin_row = self.text_matrix_row_begin(row_base, col, x);
        TextMatrixRowGeometryTransition {
            finished_row,
            begin_row,
        }
    }

    pub(crate) fn finish_begin_and_commit_next_text_matrix_row(
        &mut self,
        target: DisplayRowGeometryAdvanceTarget<'_>,
    ) -> TextMatrixRowGeometryTransition {
        let transition = self.finish_and_begin_next_text_matrix_row(
            target.defaults,
            target.kind,
            target.row_base,
            target.col,
            target.x,
        );
        self.commit(target.commit_target);
        transition
    }

    pub(crate) fn text_matrix_row_begin(
        &self,
        row_base: usize,
        col: usize,
        x: f32,
    ) -> TextMatrixRowBegin {
        TextMatrixRowBegin {
            matrix_row: row_base + self.row,
            row: self.row,
            col,
            y: self.y,
            x,
        }
    }

    pub(crate) fn state(&self) -> DisplayRowGeometryState {
        DisplayRowGeometryState {
            row: self.row,
            y: self.y,
            row_extra_y: self.row_extra_y,
            height: self.metrics.height(),
            ascent: self.metrics.ascent(),
        }
    }
}
