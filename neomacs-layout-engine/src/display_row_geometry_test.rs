use super::*;
use crate::window_output::{
    TextMatrixRowBegin, TextMatrixRowGeometryTransition, TextMatrixRowMetrics,
};

#[test]
fn current_display_row_metrics_tracks_glyph_extents_and_overflow() {
    let mut metrics = CurrentDisplayRowMetrics::new(16.0, 12.0);

    metrics.include_glyph(24.0, 18.0);

    assert_eq!(metrics.height(), 24.0);
    assert_eq!(metrics.ascent(), 18.0);
    assert_eq!(metrics.extra_height_over_default(16.0), 8.0);
    assert_eq!(
        metrics.finish_current_row(7.0),
        TextMatrixRowMetrics {
            y: 7.0,
            height: 24.0,
            ascent: 18.0,
        }
    );
}

#[test]
fn current_display_row_metrics_builds_next_row_vertical_delta() {
    let mut metrics = CurrentDisplayRowMetrics::new(16.0, 12.0);
    metrics.include_glyph(24.0, 18.0);

    assert_eq!(metrics.next_row_vertical_delta(16.0, 3.0), 11.0);
}

#[test]
fn current_display_row_metrics_resets_to_default_extents() {
    let mut metrics = CurrentDisplayRowMetrics::new(16.0, 12.0);
    metrics.include_glyph(24.0, 18.0);

    metrics.reset(14.0, 10.0);

    assert_eq!(metrics.height(), 14.0);
    assert_eq!(metrics.ascent(), 10.0);
    assert_eq!(metrics.extra_height_over_default(16.0), 0.0);
}

#[test]
fn current_display_row_metrics_finishes_row_and_resets_to_default_extents() {
    let mut metrics = CurrentDisplayRowMetrics::new(16.0, 12.0);
    metrics.include_glyph(24.0, 18.0);

    let finished = metrics.finish_and_reset(7.0, 14.0, 10.0);

    assert_eq!(
        finished,
        TextMatrixRowMetrics {
            y: 7.0,
            height: 24.0,
            ascent: 18.0,
        }
    );
    assert_eq!(metrics.height(), 14.0);
    assert_eq!(metrics.ascent(), 10.0);
}

#[test]
fn current_display_row_metrics_finishes_current_row_without_resetting_extents() {
    let mut metrics = CurrentDisplayRowMetrics::new(16.0, 12.0);
    metrics.include_glyph(24.0, 18.0);

    let finished = metrics.finish_current_row(7.0);

    assert_eq!(
        finished,
        TextMatrixRowMetrics {
            y: 7.0,
            height: 24.0,
            ascent: 18.0,
        }
    );
    assert_eq!(metrics.height(), 24.0);
    assert_eq!(metrics.ascent(), 18.0);
}

#[test]
fn current_display_row_metrics_advances_to_next_row_from_finished_extents() {
    let mut metrics = CurrentDisplayRowMetrics::new(16.0, 12.0);
    metrics.include_glyph(24.0, 18.0);

    let advance = metrics.finish_and_advance_to_next_row(CurrentDisplayRowAdvance {
        y: 7.0,
        next_row: 3,
        text_y: 10.0,
        row_extra_y: 2.0,
        default_height: 16.0,
        default_ascent: 12.0,
        kind: DisplayRowAdvanceKind::LineBreak { line_spacing: 3.0 },
    });

    assert_eq!(
        advance,
        DisplayRowAdvance {
            finished: TextMatrixRowMetrics {
                y: 7.0,
                height: 24.0,
                ascent: 18.0,
            },
            next_y: 10.0 + 3.0 * 16.0 + 13.0,
            row_extra_y: 13.0,
            next_height: 16.0,
            next_ascent: 12.0,
        }
    );
    assert_eq!(metrics.height(), 16.0);
    assert_eq!(metrics.ascent(), 12.0);
}

#[test]
fn current_display_row_metrics_advances_visual_wrap_without_line_spacing() {
    let mut metrics = CurrentDisplayRowMetrics::new(16.0, 12.0);
    metrics.include_glyph(24.0, 18.0);

    let advance = metrics.finish_and_advance_to_next_row(CurrentDisplayRowAdvance {
        y: 7.0,
        next_row: 2,
        text_y: 10.0,
        row_extra_y: 2.0,
        default_height: 16.0,
        default_ascent: 12.0,
        kind: DisplayRowAdvanceKind::VisualWrap,
    });

    assert_eq!(advance.row_extra_y, 10.0);
    assert_eq!(advance.next_y, 10.0 + 2.0 * 16.0 + 10.0);
    assert_eq!(metrics.height(), 16.0);
    assert_eq!(metrics.ascent(), 12.0);
}

#[test]
fn display_row_geometry_cursor_advances_row_position_and_resets_metrics() {
    let mut cursor = DisplayRowGeometryCursor::from_state(DisplayRowGeometryState {
        row: 2,
        y: 42.0,
        row_extra_y: 3.0,
        height: 24.0,
        ascent: 18.0,
    });

    let hit_row = cursor.hit_row(11, 22);
    assert_eq!(hit_row.y_start, 42.0);
    assert_eq!(hit_row.y_end, 66.0);
    assert_eq!(hit_row.charpos_start, 11);
    assert_eq!(hit_row.charpos_end, 22);

    let finished = cursor.finish_and_advance_to_next_row(
        DisplayRowGeometryDefaults {
            text_y: 10.0,
            height: 16.0,
            ascent: 12.0,
        },
        DisplayRowAdvanceKind::LineBreak { line_spacing: 4.0 },
    );

    assert_eq!(
        finished,
        TextMatrixRowMetrics {
            y: 42.0,
            height: 24.0,
            ascent: 18.0,
        }
    );
    assert_eq!(
        cursor.state(),
        DisplayRowGeometryState {
            row: 3,
            y: 10.0 + 3.0 * 16.0 + 15.0,
            row_extra_y: 15.0,
            height: 16.0,
            ascent: 12.0,
        }
    );
    assert_eq!(
        cursor.text_matrix_row_begin(5, 7, 13.0),
        TextMatrixRowBegin {
            matrix_row: 8,
            row: 3,
            col: 7,
            y: 10.0 + 3.0 * 16.0 + 15.0,
            x: 13.0,
        }
    );
}

#[test]
fn display_row_geometry_cursor_finishes_current_row_without_advancing() {
    let cursor = DisplayRowGeometryCursor::from_state(DisplayRowGeometryState {
        row: 2,
        y: 42.0,
        row_extra_y: 3.0,
        height: 24.0,
        ascent: 18.0,
    });

    assert_eq!(
        cursor.finish_current_row(),
        TextMatrixRowMetrics {
            y: 42.0,
            height: 24.0,
            ascent: 18.0,
        }
    );
    assert_eq!(
        cursor.state(),
        DisplayRowGeometryState {
            row: 2,
            y: 42.0,
            row_extra_y: 3.0,
            height: 24.0,
            ascent: 18.0,
        }
    );
}

#[test]
fn display_row_geometry_state_builds_from_legacy_row_variables_by_name() {
    assert_eq!(
        DisplayRowGeometryState::from_legacy(LegacyDisplayRowGeometry {
            row: 4,
            y: 80.0,
            row_extra_y: 9.0,
            row_max_height: 20.0,
            row_max_ascent: 14.0,
        }),
        DisplayRowGeometryState {
            row: 4,
            y: 80.0,
            row_extra_y: 9.0,
            height: 20.0,
            ascent: 14.0,
        }
    );
}

#[test]
fn legacy_display_row_geometry_vars_snapshots_and_applies_by_name() {
    let mut row = 4;
    let mut y = 80.0;
    let mut row_extra_y = 9.0;
    let mut row_max_height = 20.0;
    let mut row_max_ascent = 14.0;

    {
        let mut vars = LegacyDisplayRowGeometryVars {
            row: &mut row,
            y: &mut y,
            row_extra_y: &mut row_extra_y,
            row_max_height: &mut row_max_height,
            row_max_ascent: &mut row_max_ascent,
        };

        assert_eq!(
            vars.snapshot(),
            LegacyDisplayRowGeometry {
                row: 4,
                y: 80.0,
                row_extra_y: 9.0,
                row_max_height: 20.0,
                row_max_ascent: 14.0,
            }
        );

        vars.apply(DisplayRowGeometryState {
            row: 5,
            y: 120.0,
            row_extra_y: 13.0,
            height: 24.0,
            ascent: 18.0,
        });
    }

    assert_eq!(row, 5);
    assert_eq!(y, 120.0);
    assert_eq!(row_extra_y, 13.0);
    assert_eq!(row_max_height, 24.0);
    assert_eq!(row_max_ascent, 18.0);
}

#[test]
fn display_row_geometry_commit_target_groups_legacy_vars_and_row_y_recorder() {
    let cursor = DisplayRowGeometryCursor::from_state(DisplayRowGeometryState {
        row: 5,
        y: 120.0,
        row_extra_y: 13.0,
        height: 24.0,
        ascent: 18.0,
    });
    let mut row = 0;
    let mut y = 0.0;
    let mut row_extra_y = 0.0;
    let mut row_max_height = 1.0;
    let mut row_max_ascent = 1.0;
    let mut row_y_positions = vec![8.0];

    cursor.commit(DisplayRowGeometryCommitTarget::recording_row_y(
        LegacyDisplayRowGeometryVars {
            row: &mut row,
            y: &mut y,
            row_extra_y: &mut row_extra_y,
            row_max_height: &mut row_max_height,
            row_max_ascent: &mut row_max_ascent,
        },
        &mut row_y_positions,
    ));

    assert_eq!(row, 5);
    assert_eq!(y, 120.0);
    assert_eq!(row_extra_y, 13.0);
    assert_eq!(row_max_height, 24.0);
    assert_eq!(row_max_ascent, 18.0);
    assert_eq!(row_y_positions, vec![8.0, 120.0]);
}

#[test]
fn display_row_geometry_cursor_finishes_and_builds_next_text_matrix_row_begin() {
    let mut cursor = DisplayRowGeometryCursor::from_state(DisplayRowGeometryState {
        row: 2,
        y: 42.0,
        row_extra_y: 3.0,
        height: 24.0,
        ascent: 18.0,
    });

    let transition = cursor.finish_and_begin_next_text_matrix_row(
        DisplayRowGeometryDefaults {
            text_y: 10.0,
            height: 16.0,
            ascent: 12.0,
        },
        DisplayRowAdvanceKind::LineBreak { line_spacing: 4.0 },
        5,
        7,
        13.0,
    );

    assert_eq!(
        transition,
        TextMatrixRowGeometryTransition {
            finished_row: TextMatrixRowMetrics {
                y: 42.0,
                height: 24.0,
                ascent: 18.0,
            },
            begin_row: TextMatrixRowBegin {
                matrix_row: 8,
                row: 3,
                col: 7,
                y: 10.0 + 3.0 * 16.0 + 15.0,
                x: 13.0,
            },
        }
    );
    assert_eq!(
        cursor.state(),
        DisplayRowGeometryState {
            row: 3,
            y: 10.0 + 3.0 * 16.0 + 15.0,
            row_extra_y: 15.0,
            height: 16.0,
            ascent: 12.0,
        }
    );
}

#[test]
fn display_row_geometry_advance_target_groups_transition_and_commit_inputs() {
    let mut cursor = DisplayRowGeometryCursor::from_state(DisplayRowGeometryState {
        row: 2,
        y: 42.0,
        row_extra_y: 3.0,
        height: 24.0,
        ascent: 18.0,
    });
    let mut row = 0;
    let mut y = 0.0;
    let mut row_extra_y = 0.0;
    let mut row_max_height = 1.0;
    let mut row_max_ascent = 1.0;
    let mut row_y_positions = vec![8.0];

    let transition = cursor.finish_begin_and_commit_next_text_matrix_row(
        DisplayRowGeometryAdvanceTarget::line_break(
            DisplayRowGeometryDefaults {
                text_y: 10.0,
                height: 16.0,
                ascent: 12.0,
            },
            5,
            7,
            13.0,
            4.0,
            DisplayRowGeometryCommitTarget::recording_row_y(
                LegacyDisplayRowGeometryVars {
                    row: &mut row,
                    y: &mut y,
                    row_extra_y: &mut row_extra_y,
                    row_max_height: &mut row_max_height,
                    row_max_ascent: &mut row_max_ascent,
                },
                &mut row_y_positions,
            ),
        ),
    );

    assert_eq!(
        transition,
        TextMatrixRowGeometryTransition {
            finished_row: TextMatrixRowMetrics {
                y: 42.0,
                height: 24.0,
                ascent: 18.0,
            },
            begin_row: TextMatrixRowBegin {
                matrix_row: 8,
                row: 3,
                col: 7,
                y: 10.0 + 3.0 * 16.0 + 15.0,
                x: 13.0,
            },
        }
    );
    assert_eq!(row, 3);
    assert_eq!(y, 10.0 + 3.0 * 16.0 + 15.0);
    assert_eq!(row_extra_y, 15.0);
    assert_eq!(row_max_height, 16.0);
    assert_eq!(row_max_ascent, 12.0);
    assert_eq!(row_y_positions, vec![8.0, 10.0 + 3.0 * 16.0 + 15.0]);
}

#[test]
fn display_row_geometry_advance_target_line_break_constructor_sets_kind() {
    let mut cursor = DisplayRowGeometryCursor::from_state(DisplayRowGeometryState {
        row: 2,
        y: 42.0,
        row_extra_y: 3.0,
        height: 24.0,
        ascent: 18.0,
    });
    let mut row = 0;
    let mut y = 0.0;
    let mut row_extra_y = 0.0;
    let mut row_max_height = 1.0;
    let mut row_max_ascent = 1.0;
    let mut row_y_positions = vec![8.0];

    let transition = cursor.finish_begin_and_commit_next_text_matrix_row(
        DisplayRowGeometryAdvanceTarget::line_break(
            DisplayRowGeometryDefaults {
                text_y: 10.0,
                height: 16.0,
                ascent: 12.0,
            },
            5,
            7,
            13.0,
            4.0,
            DisplayRowGeometryCommitTarget::recording_row_y(
                LegacyDisplayRowGeometryVars {
                    row: &mut row,
                    y: &mut y,
                    row_extra_y: &mut row_extra_y,
                    row_max_height: &mut row_max_height,
                    row_max_ascent: &mut row_max_ascent,
                },
                &mut row_y_positions,
            ),
        ),
    );

    assert_eq!(
        transition.begin_row,
        TextMatrixRowBegin {
            matrix_row: 8,
            row: 3,
            col: 7,
            y: 10.0 + 3.0 * 16.0 + 15.0,
            x: 13.0,
        }
    );
    assert_eq!(row_extra_y, 15.0);
    assert_eq!(row_y_positions, vec![8.0, 10.0 + 3.0 * 16.0 + 15.0]);
}
