use super::ChromeRowOutput;
use super::DisplayProgressSink;
use super::TextMatrixRowBegin;
use super::TextMatrixRowMetrics;
use super::TextMatrixRowTransition;
use super::TextRowOutput;
use super::WindowOutputEmitter;
use super::begin_text_matrix_row;
use super::finish_and_maybe_begin_text_matrix_row;
use crate::display_item::DisplaySourcePosition;
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowGlyphSlot, DisplayRowPosition,
    DisplayRowWriteMetrics,
};
use crate::display_status_line::DisplayRowOutputProgress;
use crate::matrix_builder::GlyphMatrixBuilder;
use neomacs_display_protocol::types::Rect;
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::Context;

#[test]
fn emit_text_span_advances_live_output_before_row_finish() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("output-emitter-span", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    emitter.emit_text_span(
        &mut eval,
        LispCharPos1::new(1),
        0,
        0.0,
        0.0,
        0.0,
        24.0,
        16.0,
        0,
        3,
    );

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");

    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 24,
            y: 0,
            row: 0,
            col: 3,
        })
    );
}

#[test]
fn display_progress_sink_emits_buffer_slots_from_row_builder_progress() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-row-position-span", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    emitter.emit_text_progress(
        &mut eval,
        TextRowOutput {
            row: 0,
            row_y: 0.0,
            glyph_y: 0.0,
            height: 16.0,
        },
        &DisplayRowAppendProgress {
            start: DisplayRowPosition { x_px: 8.0, col: 1 },
            end: DisplayRowPosition { x_px: 24.0, col: 3 },
            metrics: DisplayRowWriteMetrics {
                width_px: 16.0,
                width_cols: 2,
            },
            status: DisplayRowAppendStatus::Complete,
            slots: vec![DisplayRowGlyphSlot {
                source: DisplaySourcePosition::buffer(
                    BufferId(7),
                    CharPos0::new(0),
                    EmacsBytePos::new(0),
                ),
                x_px: 8.0,
                col: 1,
                width_px: 16.0,
                width_cols: 2,
            }],
        },
    );

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");

    assert_eq!(emitter.display_point_len(), 1);
    assert_eq!(
        emitter
            .point_for_lisp_buffer_pos(LispCharPos1::ONE)
            .expect("buffer display point")
            .width,
        16
    );
    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 24,
            y: 0,
            row: 0,
            col: 3,
        })
    );
}

#[test]
fn display_progress_sink_advances_without_points_for_non_buffer_slots() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-lisp-string-slot", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    emitter.emit_text_progress(
        &mut eval,
        TextRowOutput {
            row: 0,
            row_y: 0.0,
            glyph_y: 0.0,
            height: 16.0,
        },
        &DisplayRowAppendProgress {
            start: DisplayRowPosition { x_px: 0.0, col: 0 },
            end: DisplayRowPosition { x_px: 24.0, col: 3 },
            metrics: DisplayRowWriteMetrics {
                width_px: 24.0,
                width_cols: 3,
            },
            status: DisplayRowAppendStatus::Complete,
            slots: vec![DisplayRowGlyphSlot {
                source: DisplaySourcePosition::lisp_string(3, 0, 0),
                x_px: 0.0,
                col: 0,
                width_px: 24.0,
                width_cols: 3,
            }],
        },
    );

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");

    assert_eq!(emitter.display_point_len(), 0);
    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 24,
            y: 0,
            row: 0,
            col: 3,
        })
    );
}

#[test]
fn display_progress_sink_records_chrome_row_progress() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-chrome-progress", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    let output = ChromeRowOutput { row: 2, y: 18.0 };
    let progress = DisplayRowOutputProgress {
        end_x: 40.0,
        end_col: 5,
        y: 18.0,
        height: 14.0,
    };

    emitter.begin_chrome_progress(&mut eval, output);
    emitter.emit_chrome_progress(&mut eval, output, progress);
    emitter.finish_chrome_progress(progress);

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");

    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 40,
            y: 18,
            row: 2,
            col: 5,
        })
    );
    assert_eq!(emitter.rows().len(), 1);
    assert_eq!(emitter.rows()[0].row, 2);
    assert_eq!(emitter.rows()[0].height, 14);
}

#[test]
fn text_matrix_row_transition_finishes_without_starting_past_max_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-row-exhaustion", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    begin_text_matrix_row(
        &mut builder,
        &mut emitter,
        &mut eval,
        TextMatrixRowBegin {
            matrix_row: 0,
            row: 0,
            col: 0,
            y: 0.0,
            x: 0.0,
        },
    );

    let transition = finish_and_maybe_begin_text_matrix_row(
        &mut builder,
        &mut emitter,
        &mut eval,
        TextMatrixRowMetrics {
            y: 0.0,
            height: 16.0,
            ascent: 12.0,
        },
        TextMatrixRowBegin {
            matrix_row: 1,
            row: 1,
            col: 0,
            y: 16.0,
            x: 0.0,
        },
        1,
    );

    assert_eq!(transition, TextMatrixRowTransition::ExhaustedRows);
    assert_eq!(emitter.rows().len(), 1);

    builder.end_window();
    let state = builder.finish(10, 1, 8.0, 16.0);
    assert_eq!(state.window_matrices[0].matrix.rows.len(), 1);
}
