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
fn legacy_display_row_geometry_vars_include_glyph_vertical_metrics_by_name() {
    let mut row = 4;
    let mut y = 80.0;
    let mut row_extra_y = 9.0;
    let mut row_max_height = 16.0;
    let mut row_max_ascent = 12.0;

    LegacyDisplayRowGeometryVars {
        row: &mut row,
        y: &mut y,
        row_extra_y: &mut row_extra_y,
        row_max_height: &mut row_max_height,
        row_max_ascent: &mut row_max_ascent,
    }
    .include_glyph_vertical_metrics(24.0, 18.0);

    assert_eq!(row, 4);
    assert_eq!(y, 80.0);
    assert_eq!(row_extra_y, 9.0);
    assert_eq!(row_max_height, 24.0);
    assert_eq!(row_max_ascent, 18.0);
}

#[test]
fn legacy_display_row_geometry_vars_include_row_extents_by_name() {
    let mut row = 4;
    let mut y = 80.0;
    let mut row_extra_y = 9.0;
    let mut row_max_height = 16.0;
    let mut row_max_ascent = 12.0;

    LegacyDisplayRowGeometryVars {
        row: &mut row,
        y: &mut y,
        row_extra_y: &mut row_extra_y,
        row_max_height: &mut row_max_height,
        row_max_ascent: &mut row_max_ascent,
    }
    .include_row_extents(24.0, 24.0);

    assert_eq!(row, 4);
    assert_eq!(y, 80.0);
    assert_eq!(row_extra_y, 9.0);
    assert_eq!(row_max_height, 24.0);
    assert_eq!(row_max_ascent, 24.0);
}

#[test]
fn legacy_display_row_geometry_reports_current_row_visibility_by_limit() {
    let geometry = LegacyDisplayRowGeometry {
        row: 4,
        y: 80.0,
        row_extra_y: 9.0,
        row_max_height: 24.0,
        row_max_ascent: 18.0,
    };

    assert!(geometry.current_row_is_visible(DisplayRowVisibilityLimit {
        max_rows: 5,
        bottom_y: 104.0,
    }));
    assert!(!geometry.current_row_is_visible(DisplayRowVisibilityLimit {
        max_rows: 4,
        bottom_y: 104.0,
    }));
    assert!(!geometry.current_row_is_visible(DisplayRowVisibilityLimit {
        max_rows: 5,
        bottom_y: 103.9,
    }));
}

#[test]
fn legacy_display_row_geometry_vars_report_current_row_visibility_by_name() {
    let mut row = 4;
    let mut y = 80.0;
    let mut row_extra_y = 9.0;
    let mut row_max_height = 24.0;
    let mut row_max_ascent = 18.0;

    let vars = LegacyDisplayRowGeometryVars {
        row: &mut row,
        y: &mut y,
        row_extra_y: &mut row_extra_y,
        row_max_height: &mut row_max_height,
        row_max_ascent: &mut row_max_ascent,
    };

    assert!(vars.current_row_is_visible(DisplayRowVisibilityLimit {
        max_rows: 5,
        bottom_y: 104.0,
    }));
    assert!(!vars.current_row_is_visible(DisplayRowVisibilityLimit {
        max_rows: 5,
        bottom_y: 103.9,
    }));
}

#[test]
fn legacy_display_row_geometry_vars_record_current_row_y_by_name() {
    let mut row = 3;
    let mut y = 69.0;
    let mut row_extra_y = 11.0;
    let mut row_max_height = 16.0;
    let mut row_max_ascent = 12.0;
    let mut row_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);

    LegacyDisplayRowGeometryVars {
        row: &mut row,
        y: &mut y,
        row_extra_y: &mut row_extra_y,
        row_max_height: &mut row_max_height,
        row_max_ascent: &mut row_max_ascent,
    }
    .record_current_row_y(&mut row_y_positions);

    assert_eq!(row_y_positions.recorded(), &[8.0, 69.0]);
    assert_eq!(row, 3);
    assert_eq!(y, 69.0);
}

#[test]
fn display_row_y_positions_preserve_recorded_rows_and_fallback_by_geometry() {
    let mut positions = DisplayRowYPositions::with_first_row(10.0, 16.0);
    positions.record(1, 30.0);

    assert_eq!(
        positions.y_for_row(
            0,
            DisplayRowYFallback {
                text_y: 10.0,
                default_height: 16.0,
                row_extra_y: 9.0,
            }
        ),
        10.0
    );
    assert_eq!(
        positions.y_for_row(
            1,
            DisplayRowYFallback {
                text_y: 10.0,
                default_height: 16.0,
                row_extra_y: 9.0,
            }
        ),
        30.0
    );
    assert_eq!(
        positions.y_for_row(
            3,
            DisplayRowYFallback {
                text_y: 10.0,
                default_height: 16.0,
                row_extra_y: 9.0,
            }
        ),
        67.0
    );
}

#[test]
fn display_row_y_positions_expose_recording_target_without_engine_vec_access() {
    let mut positions = DisplayRowYPositions::with_first_row(10.0, 16.0);
    {
        let recording = positions.recording();
        let DisplayRowYRecording::RowYPositions(raw) = recording else {
            panic!("expected row-y recording target");
        };
        raw.push(30.0);
    }

    assert_eq!(positions.recorded(), &[10.0, 30.0]);
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
    let mut row_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);

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
    assert_eq!(row_y_positions.recorded(), &[8.0, 120.0]);
}

#[test]
fn display_row_geometry_commit_target_records_row_y_through_positions_wrapper() {
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
    let mut row_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);

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

    assert_eq!(row_y_positions.recorded(), &[8.0, 120.0]);
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
fn display_row_geometry_transition_target_groups_truncation_transition_and_commit_inputs() {
    let mut row = 2;
    let mut y = 42.0;
    let mut row_extra_y = 3.0;
    let mut row_max_height = 24.0;
    let mut row_max_ascent = 18.0;
    let mut row_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);

    let transition = LegacyDisplayRowGeometryVars {
        row: &mut row,
        y: &mut y,
        row_extra_y: &mut row_extra_y,
        row_max_height: &mut row_max_height,
        row_max_ascent: &mut row_max_ascent,
    }
    .finish_boundary(DisplayRowBoundaryTarget::new(
        DisplayRowHitRange {
            charpos_start: 0,
            charpos_end: 0,
        },
        DisplayRowGeometryTransitionTarget::truncation(
            DisplayRowGeometryDefaults {
                text_y: 10.0,
                height: 16.0,
                ascent: 12.0,
            },
            5,
            7,
            13.0,
            row_y_positions.recording(),
        ),
    ))
    .transition;

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
                y: 10.0 + 3.0 * 16.0 + 11.0,
                x: 13.0,
            },
        }
    );
    assert_eq!(row, 3);
    assert_eq!(y, 10.0 + 3.0 * 16.0 + 11.0);
    assert_eq!(row_extra_y, 11.0);
    assert_eq!(row_max_height, 16.0);
    assert_eq!(row_max_ascent, 12.0);
    assert_eq!(row_y_positions.recorded(), &[8.0, 10.0 + 3.0 * 16.0 + 11.0]);
}

#[test]
fn display_row_geometry_transition_target_line_break_constructor_sets_kind() {
    let mut row = 2;
    let mut y = 42.0;
    let mut row_extra_y = 3.0;
    let mut row_max_height = 24.0;
    let mut row_max_ascent = 18.0;
    let mut row_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);

    let transition = LegacyDisplayRowGeometryVars {
        row: &mut row,
        y: &mut y,
        row_extra_y: &mut row_extra_y,
        row_max_height: &mut row_max_height,
        row_max_ascent: &mut row_max_ascent,
    }
    .finish_boundary(DisplayRowBoundaryTarget::new(
        DisplayRowHitRange {
            charpos_start: 0,
            charpos_end: 0,
        },
        DisplayRowGeometryTransitionTarget::line_break(
            DisplayRowGeometryDefaults {
                text_y: 10.0,
                height: 16.0,
                ascent: 12.0,
            },
            5,
            7,
            13.0,
            4.0,
            row_y_positions.recording(),
        ),
    ))
    .transition;

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
    assert_eq!(row_y_positions.recorded(), &[8.0, 10.0 + 3.0 * 16.0 + 15.0]);
}

#[test]
fn legacy_display_row_geometry_vars_can_advance_and_record_row_y_in_one_request() {
    let mut row = 2;
    let mut y = 42.0;
    let mut row_extra_y = 3.0;
    let mut row_max_height = 24.0;
    let mut row_max_ascent = 18.0;
    let mut row_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);

    let transition = LegacyDisplayRowGeometryVars {
        row: &mut row,
        y: &mut y,
        row_extra_y: &mut row_extra_y,
        row_max_height: &mut row_max_height,
        row_max_ascent: &mut row_max_ascent,
    }
    .finish_boundary(DisplayRowBoundaryTarget::new(
        DisplayRowHitRange {
            charpos_start: 0,
            charpos_end: 0,
        },
        DisplayRowGeometryTransitionTarget::line_break(
            DisplayRowGeometryDefaults {
                text_y: 10.0,
                height: 16.0,
                ascent: 12.0,
            },
            5,
            7,
            13.0,
            4.0,
            row_y_positions.recording(),
        ),
    ))
    .transition;

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
    assert_eq!(row_y_positions.recorded(), &[8.0, 10.0 + 3.0 * 16.0 + 15.0]);
}

#[test]
fn legacy_display_row_geometry_vars_can_finish_row_boundary_in_one_request() {
    let mut row = 2;
    let mut y = 42.0;
    let mut row_extra_y = 3.0;
    let mut row_max_height = 24.0;
    let mut row_max_ascent = 18.0;
    let mut row_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);

    let boundary = LegacyDisplayRowGeometryVars {
        row: &mut row,
        y: &mut y,
        row_extra_y: &mut row_extra_y,
        row_max_height: &mut row_max_height,
        row_max_ascent: &mut row_max_ascent,
    }
    .finish_boundary(DisplayRowBoundaryTarget::new(
        DisplayRowHitRange {
            charpos_start: 11,
            charpos_end: 22,
        },
        DisplayRowGeometryTransitionTarget::visual_wrap(
            DisplayRowGeometryDefaults {
                text_y: 10.0,
                height: 16.0,
                ascent: 12.0,
            },
            5,
            7,
            13.0,
            row_y_positions.recording(),
        ),
    ));

    assert_eq!(boundary.hit_row.y_start, 42.0);
    assert_eq!(boundary.hit_row.y_end, 66.0);
    assert_eq!(boundary.hit_row.charpos_start, 11);
    assert_eq!(boundary.hit_row.charpos_end, 22);
    assert_eq!(
        boundary.transition,
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
                y: 10.0 + 3.0 * 16.0 + 11.0,
                x: 13.0,
            },
        }
    );
    assert_eq!(row, 3);
    assert_eq!(y, 10.0 + 3.0 * 16.0 + 11.0);
    assert_eq!(row_extra_y, 11.0);
    assert_eq!(row_max_height, 16.0);
    assert_eq!(row_max_ascent, 12.0);
    assert_eq!(row_y_positions.recorded(), &[8.0, 10.0 + 3.0 * 16.0 + 11.0]);
}

#[test]
fn display_row_boundary_transition_records_hit_row_and_returns_geometry_transition() {
    let boundary = DisplayRowBoundaryTransition {
        hit_row: HitRow {
            y_start: 42.0,
            y_end: 66.0,
            charpos_start: 11,
            charpos_end: 22,
        },
        transition: TextMatrixRowGeometryTransition {
            finished_row: TextMatrixRowMetrics {
                y: 42.0,
                height: 24.0,
                ascent: 18.0,
            },
            begin_row: TextMatrixRowBegin {
                matrix_row: 8,
                row: 3,
                col: 7,
                y: 69.0,
                x: 13.0,
            },
        },
    };
    let mut hit_rows = Vec::new();

    let transition = boundary.record_hit_row(&mut hit_rows);

    assert_eq!(hit_rows.len(), 1);
    assert_eq!(hit_rows[0].y_start, 42.0);
    assert_eq!(hit_rows[0].y_end, 66.0);
    assert_eq!(hit_rows[0].charpos_start, 11);
    assert_eq!(hit_rows[0].charpos_end, 22);
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
                y: 69.0,
                x: 13.0,
            },
        }
    );
}

#[test]
fn display_row_boundary_target_constructors_encode_boundary_kind_and_hit_range() {
    let defaults = DisplayRowGeometryDefaults {
        text_y: 10.0,
        height: 16.0,
        ascent: 12.0,
    };

    let mut line_break_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);
    let line_break = DisplayRowBoundaryTarget::line_break(
        DisplayRowHitRange {
            charpos_start: 11,
            charpos_end: 22,
        },
        defaults,
        5,
        7,
        13.0,
        4.0,
        line_break_y_positions.recording(),
    );
    let mut truncation_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);
    let truncation = DisplayRowBoundaryTarget::truncation(
        DisplayRowHitRange {
            charpos_start: 11,
            charpos_end: 22,
        },
        defaults,
        5,
        7,
        13.0,
        truncation_y_positions.recording(),
    );
    let mut visual_wrap_y_positions = DisplayRowYPositions::with_first_row(8.0, 16.0);
    let visual_wrap = DisplayRowBoundaryTarget::visual_wrap(
        DisplayRowHitRange {
            charpos_start: 11,
            charpos_end: 22,
        },
        defaults,
        5,
        7,
        13.0,
        visual_wrap_y_positions.recording(),
    );

    assert_eq!(line_break.hit_range.charpos_start, 11);
    assert_eq!(line_break.hit_range.charpos_end, 22);
    assert!(matches!(
        line_break.transition.kind,
        DisplayRowAdvanceKind::LineBreak { line_spacing: 4.0 }
    ));
    assert!(matches!(
        truncation.transition.kind,
        DisplayRowAdvanceKind::Truncation
    ));
    assert!(matches!(
        visual_wrap.transition.kind,
        DisplayRowAdvanceKind::VisualWrap
    ));
}
