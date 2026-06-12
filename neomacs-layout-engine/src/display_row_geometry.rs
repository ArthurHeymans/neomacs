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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowVisibilityLimit {
    pub(crate) max_rows: usize,
    pub(crate) bottom_y: f32,
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

    pub(crate) fn current_row_is_visible(&self, limit: DisplayRowVisibilityLimit) -> bool {
        self.snapshot().current_row_is_visible(limit)
    }

    pub(crate) fn include_glyph_vertical_metrics(&mut self, glyph_height: f32, glyph_ascent: f32) {
        let mut metrics = CurrentDisplayRowMetrics::new(*self.row_max_height, *self.row_max_ascent);
        metrics.include_glyph(glyph_height, glyph_ascent);
        *self.row_max_height = metrics.height();
        *self.row_max_ascent = metrics.ascent();
    }

    pub(crate) fn include_row_extents(&mut self, height: f32, ascent: f32) {
        *self.row_max_height = self.row_max_height.max(height);
        *self.row_max_ascent = self.row_max_ascent.max(ascent);
    }

    pub(crate) fn finish_boundary(
        self,
        target: DisplayRowBoundaryTarget<'_>,
    ) -> DisplayRowBoundaryTransition {
        let mut row_cursor = DisplayRowGeometryCursor::from_state(
            DisplayRowGeometryState::from_legacy(self.snapshot()),
        );
        let hit_row =
            row_cursor.hit_row(target.hit_range.charpos_start, target.hit_range.charpos_end);
        let transition = row_cursor.finish_and_begin_next_text_matrix_row(
            target.transition.defaults,
            target.transition.kind,
            target.transition.row_base,
            target.transition.col,
            target.transition.x,
        );
        let commit_target = match target.transition.row_y_recording {
            DisplayRowYRecording::None => DisplayRowGeometryCommitTarget::silent(self),
            DisplayRowYRecording::RowYPositions(row_y_positions) => {
                DisplayRowGeometryCommitTarget::recording_row_y(self, row_y_positions)
            }
        };
        row_cursor.commit(commit_target);
        DisplayRowBoundaryTransition {
            hit_row,
            transition,
        }
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

pub(crate) enum DisplayRowYRecording<'a> {
    None,
    RowYPositions(&'a mut Vec<f32>),
}

pub(crate) struct DisplayRowGeometryCommitTarget<'a> {
    vars: LegacyDisplayRowGeometryVars<'a>,
    row_y_recorder: DisplayRowYRecorder<'a>,
}

pub(crate) struct DisplayRowGeometryTransitionTarget<'a> {
    defaults: DisplayRowGeometryDefaults,
    kind: DisplayRowAdvanceKind,
    row_base: usize,
    col: usize,
    x: f32,
    row_y_recording: DisplayRowYRecording<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayRowHitRange {
    pub(crate) charpos_start: i64,
    pub(crate) charpos_end: i64,
}

pub(crate) struct DisplayRowBoundaryTarget<'a> {
    hit_range: DisplayRowHitRange,
    transition: DisplayRowGeometryTransitionTarget<'a>,
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRowBoundaryTransition {
    pub(crate) hit_row: HitRow,
    pub(crate) transition: TextMatrixRowGeometryTransition,
}

impl DisplayRowBoundaryTransition {
    pub(crate) fn record_hit_row(
        self,
        hit_rows: &mut Vec<HitRow>,
    ) -> TextMatrixRowGeometryTransition {
        hit_rows.push(self.hit_row);
        self.transition
    }
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

impl<'a> DisplayRowGeometryTransitionTarget<'a> {
    fn new(
        defaults: DisplayRowGeometryDefaults,
        kind: DisplayRowAdvanceKind,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
    ) -> Self {
        Self {
            defaults,
            kind,
            row_base,
            col,
            x,
            row_y_recording,
        }
    }

    pub(crate) fn line_break(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        line_spacing: f32,
        row_y_recording: DisplayRowYRecording<'a>,
    ) -> Self {
        Self::new(
            defaults,
            DisplayRowAdvanceKind::LineBreak { line_spacing },
            row_base,
            col,
            x,
            row_y_recording,
        )
    }

    pub(crate) fn truncation(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
    ) -> Self {
        Self::new(
            defaults,
            DisplayRowAdvanceKind::Truncation,
            row_base,
            col,
            x,
            row_y_recording,
        )
    }

    pub(crate) fn visual_wrap(
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
    ) -> Self {
        Self::new(
            defaults,
            DisplayRowAdvanceKind::VisualWrap,
            row_base,
            col,
            x,
            row_y_recording,
        )
    }
}

impl<'a> DisplayRowBoundaryTarget<'a> {
    pub(crate) fn new(
        hit_range: DisplayRowHitRange,
        transition: DisplayRowGeometryTransitionTarget<'a>,
    ) -> Self {
        Self {
            hit_range,
            transition,
        }
    }

    pub(crate) fn line_break(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        line_spacing: f32,
        row_y_recording: DisplayRowYRecording<'a>,
    ) -> Self {
        Self::new(
            hit_range,
            DisplayRowGeometryTransitionTarget::line_break(
                defaults,
                row_base,
                col,
                x,
                line_spacing,
                row_y_recording,
            ),
        )
    }

    pub(crate) fn truncation(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
    ) -> Self {
        Self::new(
            hit_range,
            DisplayRowGeometryTransitionTarget::truncation(
                defaults,
                row_base,
                col,
                x,
                row_y_recording,
            ),
        )
    }

    pub(crate) fn visual_wrap(
        hit_range: DisplayRowHitRange,
        defaults: DisplayRowGeometryDefaults,
        row_base: usize,
        col: usize,
        x: f32,
        row_y_recording: DisplayRowYRecording<'a>,
    ) -> Self {
        Self::new(
            hit_range,
            DisplayRowGeometryTransitionTarget::visual_wrap(
                defaults,
                row_base,
                col,
                x,
                row_y_recording,
            ),
        )
    }
}

impl LegacyDisplayRowGeometry {
    pub(crate) fn current_row_is_visible(&self, limit: DisplayRowVisibilityLimit) -> bool {
        self.row < limit.max_rows && self.y + self.row_max_height <= limit.bottom_y
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

#[cfg(test)]
#[path = "display_row_geometry_test.rs"]
mod tests;
