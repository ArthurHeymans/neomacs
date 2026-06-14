use super::ChromeRowOutput;
use super::DisplayProgressSink;
use super::TextMatrixRowBegin;
use super::TextMatrixRowGeometryTransition;
use super::TextMatrixRowMetrics;
use super::TextMatrixRowOutput;
use super::TextMatrixRowTransition;
use super::TextRowOutput;
use super::TextWindowCursor;
use super::TextWindowCursorEffects;
use super::TextWindowDecorativeCursor;
use super::TextWindowDisplayRange;
use super::TextWindowLineNumberMargin;
use super::TextWindowRightBorder;
use super::TextWindowRightEdgeMarkerColumn;
use super::TextWindowRightEdgeMarkers;
use super::WindowOutputEmitter;
use super::close_text_window_output;
use super::current_text_window_cluster_tail;
use super::emit_text_matrix_row_transition;
use super::emit_text_matrix_row_transition_with_limit;
use super::emit_text_window_line_number_margin;
use super::finish_and_end_text_matrix_row_output;
use super::finish_text_matrix_row_output;
use super::install_last_window_right_border;
use super::install_text_window_cursor_effects;
use super::install_text_window_right_edge_markers;
use super::mark_current_text_row_truncated_left;
use super::publish_text_window_cursor;
use super::publish_text_window_decorative_cursor;
use super::record_text_window_display_range;
use crate::display_item::DisplaySourcePosition;
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowGlyphSlot, DisplayRowPosition,
};
use crate::display_row_geometry::{DisplayRowFlagKind, DisplayRowFlags};
use crate::display_status_line::DisplayRowOutputProgress;
use crate::matrix_builder::GlyphMatrixBuilder;
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, GlyphRowRole, WindowInfo,
};
use neomacs_display_protocol::types::{Color, Rect};
use neomacs_display_protocol::{Glyph, GlyphArea, GlyphType};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::Context;

fn assert_char_glyph(glyph: &Glyph, ch: char, face_id: u32) {
    assert_eq!(glyph.glyph_type, GlyphType::Char { ch });
    assert_eq!(glyph.face_id, face_id);
}

fn write_char_to_current_row(
    builder: &mut GlyphMatrixBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
) {
    builder
        .with_current_row_mut(|row| {
            crate::glyph_row_writer::push_char_to_row(row, ch, face_id, charpos, 0.0);
        })
        .expect("current row");
}

fn write_wide_char_to_current_row(
    builder: &mut GlyphMatrixBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
) {
    builder
        .with_current_row_mut(|row| {
            crate::glyph_row_writer::push_wide_char_to_row(row, ch, face_id, charpos, 0.0);
        })
        .expect("current row");
}

fn write_cluster_continuation_to_current_row(
    builder: &mut GlyphMatrixBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
) {
    builder
        .with_current_row_mut(|row| {
            crate::glyph_row_writer::push_cluster_continuation_to_row(row, ch, face_id, charpos);
        })
        .expect("current row");
}

fn write_left_margin_char_to_current_row(builder: &mut GlyphMatrixBuilder, ch: char, face_id: u32) {
    builder
        .with_current_row_mut(|row| {
            row.glyphs[GlyphArea::LeftMargin.index()].push(Glyph::char(ch, face_id, 0));
        })
        .expect("current row");
}

fn write_left_margin_stretch_to_current_row(
    builder: &mut GlyphMatrixBuilder,
    width_cols: u16,
    face_id: u32,
) {
    builder
        .with_current_row_mut(|row| {
            row.glyphs[GlyphArea::LeftMargin.index()].push(Glyph::stretch(width_cols, face_id));
        })
        .expect("current row");
}

fn window_info(window_id: i64) -> WindowInfo {
    WindowInfo {
        window_id,
        buffer_id: 9,
        window_start: 1,
        window_end: 1,
        buffer_size: 100,
        bounds: Rect::new(0.0, 0.0, 80.0, 16.0),
        mode_line_height: 0.0,
        header_line_height: 0.0,
        tab_line_height: 0.0,
        selected: true,
        is_minibuffer: false,
        char_height: 16.0,
        buffer_file_name: String::new(),
        modified: false,
    }
}

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
        &DisplayRowAppendProgress::from_positions(
            DisplayRowPosition { x_px: 8.0, col: 1 },
            DisplayRowPosition { x_px: 24.0, col: 3 },
            DisplayRowAppendStatus::Complete,
            vec![DisplayRowGlyphSlot {
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
        ),
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
fn text_source_slot_emission_accepts_rendered_row_slots() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval.frame_manager_mut().create_frame(
        "output-emitter-rendered-row-slots",
        320,
        120,
        buf_id,
    );
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    emitter.begin_text_row(&mut eval, 0, 1, 0.0, 4.0);
    emitter.emit_text_source_slots(
        &mut eval,
        TextRowOutput {
            row: 0,
            row_y: 0.0,
            glyph_y: 0.0,
            height: 16.0,
        },
        &[DisplayRowGlyphSlot {
            source: DisplaySourcePosition::buffer(BufferId(7), CharPos0::ZERO, EmacsBytePos::ZERO),
            x_px: 4.0,
            col: 1,
            width_px: 16.0,
            width_cols: 2,
        }],
        DisplayRowPosition { x_px: 20.0, col: 3 },
    );

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");
    let point = emitter
        .point_for_lisp_buffer_pos(LispCharPos1::ONE)
        .expect("buffer display point");

    assert_eq!(point.x, 4);
    assert_eq!(point.width, 16);
    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 20,
            y: 0,
            row: 0,
            col: 3,
        })
    );
}

#[test]
fn display_progress_sink_merges_contiguous_slots_for_same_buffer_position() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-merged-slots", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let source = DisplaySourcePosition::buffer(BufferId(7), CharPos0::new(0), EmacsBytePos::new(0));
    emitter.emit_text_progress(
        &mut eval,
        TextRowOutput {
            row: 0,
            row_y: 0.0,
            glyph_y: 0.0,
            height: 16.0,
        },
        &DisplayRowAppendProgress::from_positions(
            DisplayRowPosition { x_px: 8.0, col: 1 },
            DisplayRowPosition { x_px: 24.0, col: 3 },
            DisplayRowAppendStatus::Complete,
            vec![
                DisplayRowGlyphSlot {
                    source: source.clone(),
                    x_px: 8.0,
                    col: 1,
                    width_px: 8.0,
                    width_cols: 1,
                },
                DisplayRowGlyphSlot {
                    source,
                    x_px: 16.0,
                    col: 2,
                    width_px: 8.0,
                    width_cols: 1,
                },
            ],
        ),
    );

    let point = emitter
        .point_for_buffer_pos(LispCharPos1::ONE)
        .expect("merged display point");
    assert_eq!(emitter.display_point_len(), 1);
    assert_eq!(point.x, 8);
    assert_eq!(point.width, 16);
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
        &DisplayRowAppendProgress::from_positions(
            DisplayRowPosition { x_px: 0.0, col: 0 },
            DisplayRowPosition { x_px: 24.0, col: 3 },
            DisplayRowAppendStatus::Complete,
            vec![DisplayRowGlyphSlot {
                source: DisplaySourcePosition::lisp_string(3, 0, 0),
                x_px: 0.0,
                col: 0,
                width_px: 24.0,
                width_cols: 3,
            }],
        ),
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
fn text_matrix_row_output_surface_begins_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-row-surface", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    TextMatrixRowOutput::new(&mut builder, &mut emitter, &mut eval).begin(TextMatrixRowBegin {
        matrix_row: 0,
        row: 0,
        col: 0,
        y: 0.0,
        x: 0.0,
    });

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");
    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 0,
            y: 0,
            row: 0,
            col: 0,
        })
    );

    builder.end_row();
    builder.end_window();
}

#[test]
fn record_text_window_display_range_updates_matching_last_window_info() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.push_window_info(window_info(41));

    record_text_window_display_range(
        &mut builder,
        TextWindowDisplayRange {
            window_id: 41,
            window_start: LispCharPos1::new(7),
            window_end: LispCharPos1::new(19),
        },
    );

    let info = builder.window_infos().last().expect("window info");
    assert_eq!(info.window_start, 7);
    assert_eq!(info.window_end, 19);

    record_text_window_display_range(
        &mut builder,
        TextWindowDisplayRange {
            window_id: 42,
            window_start: LispCharPos1::new(11),
            window_end: LispCharPos1::new(23),
        },
    );

    let info = builder.window_infos().last().expect("window info");
    assert_eq!(info.window_start, 7);
    assert_eq!(info.window_end, 19);
}

#[test]
fn close_text_window_output_closes_active_matrix_window() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(9, 1, 5, Rect::new(0.0, 0.0, 40.0, 16.0), true);

    close_text_window_output(&mut builder);

    assert_eq!(builder.windows().len(), 1);
    assert_eq!(builder.windows()[0].window_id, 9);
}

#[test]
fn emit_text_window_line_number_margin_right_aligns_text_and_trailing_separator() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(9, 1, 5, Rect::new(0.0, 0.0, 40.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);

    emit_text_window_line_number_margin(
        &mut builder,
        TextWindowLineNumberMargin {
            text: "42",
            cols: 4,
            face_id: 7,
            row_y: 0.0,
            row_height: 16.0,
            row_ascent: 12.0,
            char_width: 8.0,
        },
    );

    builder.end_row();
    builder.end_window();

    let state = builder.finish(5, 1, 8.0, 16.0);
    let margin = &state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::LeftMargin as usize];

    assert_eq!(margin.len(), 4);
    assert_eq!(margin[0].glyph_type, GlyphType::Stretch { width_cols: 1 });
    assert_char_glyph(&margin[1], '4', 7);
    assert_char_glyph(&margin[2], '2', 7);
    assert_eq!(margin[3].glyph_type, GlyphType::Stretch { width_cols: 1 });
    assert!(margin.iter().all(|glyph| glyph.face_id == 7));
}

#[test]
fn publish_text_window_cursor_installs_selected_phys_cursor_without_window_cursor_item() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-selected-cursor", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(window_id.0, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_left_margin_char_to_current_row(&mut builder, '1', 7);
    write_left_margin_stretch_to_current_row(&mut builder, 1, 7);
    write_char_to_current_row(&mut builder, 'H', 3, 100);

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 16.0, 8.0);
    publish_text_window_cursor(
        &mut builder,
        &mut emitter,
        TextWindowCursor {
            selected: true,
            window_id: window_id.0 as i64,
            charpos: 100,
            slot_id: DisplaySlotId {
                window_id: window_id.0 as i64,
                row: 0,
                col: 0,
            },
            x: 40.0,
            y: 24.0,
            width: 8.0,
            height: 16.0,
            ascent: 12.0,
            style: CursorStyle::FilledBox,
            color: Color::WHITE,
            cursor_fg: Color::BLACK,
            text_area_left: 16.0,
            window_top: 8.0,
        },
    );

    builder.end_row();
    builder.end_window();
    let snapshot = emitter.finish_snapshot(&mut eval, 0, 0, 0, 0);
    let state = builder.finish(10, 1, 8.0, 16.0);

    assert!(state.cursors.is_empty());
    let phys = state.phys_cursor.expect("selected phys cursor");
    assert_eq!(phys.slot_id.col, 2);
    assert_eq!(state.window_matrices[0].matrix.rows[0].cursor_col, Some(2));

    let live = snapshot.phys_cursor.expect("live phys cursor");
    assert_eq!(live.x, 24);
    assert_eq!(live.y, 16);
    assert_eq!(live.row, 0);
    assert_eq!(live.col, 0);
}

#[test]
fn publish_text_window_decorative_cursor_installs_cursor_item_and_effects_only() {
    let mut builder = GlyphMatrixBuilder::new();
    let effects = EffectsConfig::default();

    publish_text_window_decorative_cursor(
        &mut builder,
        TextWindowDecorativeCursor {
            window_id: 77,
            slot_id: DisplaySlotId {
                window_id: 77,
                row: 3,
                col: 5,
            },
            x: 40.0,
            y: 24.0,
            width: 8.0,
            height: 16.0,
            style: CursorStyle::Bar(2.0),
            color: Color::WHITE,
            effects: Some(effects.clone()),
        },
    );

    let state = builder.finish(10, 1, 8.0, 16.0);
    assert!(state.phys_cursor.is_none());
    assert_eq!(state.cursors.len(), 1);
    assert_eq!(state.cursors[0].window_id, 77);
    assert_eq!(state.cursors[0].slot_id.row, 3);
    assert_eq!(state.cursors[0].slot_id.col, 5);
    assert_eq!(state.cursor_effects_by_window.get(&77), Some(&effects));
}

#[test]
fn install_text_window_cursor_effects_records_window_effect_profile() {
    let mut builder = GlyphMatrixBuilder::new();
    let effects = EffectsConfig::default();

    install_text_window_cursor_effects(
        &mut builder,
        TextWindowCursorEffects {
            window_id: 42,
            effects: effects.clone(),
        },
    );

    let state = builder.finish(10, 1, 8.0, 16.0);
    assert_eq!(state.cursor_effects_by_window.get(&42), Some(&effects));
    assert!(state.cursors.is_empty());
    assert!(state.phys_cursor.is_none());
}

#[test]
fn current_text_window_cluster_tail_reports_live_text_row_tail() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 5, Rect::new(0.0, 0.0, 40.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);

    assert_eq!(current_text_window_cluster_tail(&builder), None);

    write_wide_char_to_current_row(&mut builder, '\u{1F1EF}', 3, 100);
    assert_eq!(
        current_text_window_cluster_tail(&builder),
        Some(('\u{1F1EF}', true))
    );

    write_cluster_continuation_to_current_row(&mut builder, '\u{1F1F5}', 3, 101);
    assert_eq!(
        current_text_window_cluster_tail(&builder),
        Some(('\u{1F1F5}', false))
    );
}

#[test]
fn text_matrix_row_commands_begin_and_finish_output() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-row-commands", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    TextMatrixRowOutput::new(&mut builder, &mut emitter, &mut eval).begin(TextMatrixRowBegin {
        matrix_row: 0,
        row: 0,
        col: 0,
        y: 0.0,
        x: 0.0,
    });

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");
    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 0,
            y: 0,
            row: 0,
            col: 0,
        })
    );

    finish_text_matrix_row_output(
        &mut builder,
        &mut emitter,
        &mut eval,
        TextMatrixRowMetrics {
            y: 0.0,
            height: 16.0,
            ascent: 12.0,
        },
    );

    assert_eq!(emitter.rows().len(), 1);
    assert_eq!(emitter.rows()[0].row, 0);

    builder.end_row();
    builder.end_window();
    let state = builder.finish(10, 1, 8.0, 16.0);
    assert_eq!(state.window_matrices[0].matrix.rows.len(), 1);
}

#[test]
fn text_matrix_row_metrics_finish_and_end_closes_matrix_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-row-finish-end", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    TextMatrixRowOutput::new(&mut builder, &mut emitter, &mut eval).begin(TextMatrixRowBegin {
        matrix_row: 0,
        row: 0,
        col: 0,
        y: 0.0,
        x: 0.0,
    });

    finish_and_end_text_matrix_row_output(
        &mut builder,
        &mut emitter,
        &mut eval,
        TextMatrixRowMetrics {
            y: 0.0,
            height: 16.0,
            ascent: 12.0,
        },
    );

    assert_eq!(emitter.rows().len(), 1);

    builder.end_window();
    let state = builder.finish(10, 1, 8.0, 16.0);
    assert_eq!(state.window_matrices[0].matrix.rows.len(), 1);
}

#[test]
fn text_window_right_edge_markers_use_row_flags() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 3, 5, Rect::new(0.0, 0.0, 40.0, 48.0), true);
    for row in 0..3 {
        builder.begin_row(row, GlyphRowRole::Text);
        write_char_to_current_row(&mut builder, 'x', 7, row);
        builder.end_row();
    }

    let mut row_flags = DisplayRowFlags::new(3);
    row_flags.mark(0, DisplayRowFlagKind::Truncated);
    row_flags.mark(1, DisplayRowFlagKind::Continued);
    row_flags.mark(2, DisplayRowFlagKind::Continuation);

    install_text_window_right_edge_markers(
        &mut builder,
        TextWindowRightEdgeMarkers {
            text_matrix_row_base: 0,
            matrix_cols: 5,
            column: TextWindowRightEdgeMarkerColumn::BeforeRightBorder,
            row_flags: &row_flags,
            face_id: 9,
            char_width: 8.0,
        },
    );

    builder.end_window();
    let state = builder.finish(10, 3, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;
    let row0 = &matrix.rows[0].glyphs[GlyphArea::Text.index()];
    let row1 = &matrix.rows[1].glyphs[GlyphArea::Text.index()];
    let row2 = &matrix.rows[2].glyphs[GlyphArea::Text.index()];

    assert_char_glyph(&row0[3], '$', 9);
    assert_char_glyph(&row1[3], '\\', 9);
    assert_eq!(row2.len(), 1);
    assert_char_glyph(&row2[0], 'x', 7);
}

#[test]
fn text_window_right_edge_markers_render_padding_and_truncation_as_text_items() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 2, 5, Rect::new(0.0, 0.0, 40.0, 32.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    for ch in "ABCDE".chars() {
        write_char_to_current_row(&mut builder, ch, 0, 0);
    }
    builder.end_row();
    builder.begin_row(1, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'X', 0, 0);
    builder.end_row();

    let mut row_flags = DisplayRowFlags::new(2);
    row_flags.mark(0, DisplayRowFlagKind::Truncated);
    row_flags.mark(1, DisplayRowFlagKind::Truncated);
    install_text_window_right_edge_markers(
        &mut builder,
        TextWindowRightEdgeMarkers {
            text_matrix_row_base: 0,
            matrix_cols: 5,
            column: TextWindowRightEdgeMarkerColumn::LastColumn,
            row_flags: &row_flags,
            face_id: 13,
            char_width: 8.0,
        },
    );
    builder.end_window();

    let state = builder.finish(10, 2, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;
    let row_text = |row: usize| -> String {
        matrix.rows[row].glyphs[GlyphArea::Text.index()]
            .iter()
            .map(|glyph| match &glyph.glyph_type {
                GlyphType::Char { ch } => *ch,
                _ => '?',
            })
            .collect()
    };

    assert_eq!(row_text(0), "ABCD$");
    assert_eq!(row_text(1), "X   $");
    assert!(
        matrix.rows[0].glyphs[GlyphArea::Text.index()]
            .iter()
            .all(|glyph| glyph.face_id == 0 || glyph.face_id == 13)
    );
    assert!(
        matrix.rows[1].glyphs[GlyphArea::Text.index()][1..]
            .iter()
            .all(|glyph| glyph.face_id == 13)
    );
}

#[test]
fn text_window_right_border_pads_and_replaces_text_with_row_items() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(0.0, 0.0, 80.0, 32.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    for ch in "0123456789".chars() {
        write_char_to_current_row(&mut builder, ch, 0, 0);
    }
    builder.end_row();
    builder.begin_row(1, GlyphRowRole::Text);
    for ch in "abcde".chars() {
        write_char_to_current_row(&mut builder, ch, 0, 0);
    }
    builder.end_row();
    builder.end_window();

    install_last_window_right_border(
        &mut builder,
        TextWindowRightBorder {
            ch: '|',
            face_id: 99,
            char_width: 8.0,
        },
    );

    let state = builder.finish(20, 5, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;
    let row0_text = &matrix.rows[0].glyphs[GlyphArea::Text.index()];
    let row0_right = &matrix.rows[0].glyphs[GlyphArea::RightMargin.index()];
    let row1_text = &matrix.rows[1].glyphs[GlyphArea::Text.index()];
    let row1_right = &matrix.rows[1].glyphs[GlyphArea::RightMargin.index()];
    let row_chars = |glyphs: &[Glyph]| -> String {
        glyphs
            .iter()
            .map(|glyph| match &glyph.glyph_type {
                GlyphType::Char { ch } => *ch,
                _ => '?',
            })
            .collect()
    };

    assert_eq!(row0_text.len(), 9);
    assert_eq!(row_chars(row0_text), "012345678");
    assert_eq!(row0_right.len(), 1);
    assert_eq!(row0_right[0].glyph_type, GlyphType::Char { ch: '|' });
    assert_eq!(row0_right[0].face_id, 99);
    assert_eq!(row1_text.len(), 9);
    assert_eq!(row_chars(row1_text), "abcde    ");
    assert_eq!(row1_right.len(), 1);
    assert_eq!(row1_right[0].glyph_type, GlyphType::Char { ch: '|' });
    assert_eq!(row1_right[0].face_id, 99);
    assert_eq!(row1_text[5].face_id, 99);
    assert_eq!(row1_text[8].face_id, 99);
}

#[test]
fn text_window_right_border_paints_blank_rows_without_marking_text_displayed() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 3, 5, Rect::new(0.0, 0.0, 40.0, 48.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'A', 0, 0);
    builder.end_row();
    builder.begin_row(2, GlyphRowRole::Text);
    write_char_to_current_row(&mut builder, 'Z', 0, 0);
    builder.end_row();
    builder.end_window();

    install_last_window_right_border(
        &mut builder,
        TextWindowRightBorder {
            ch: '|',
            face_id: 7,
            char_width: 8.0,
        },
    );

    let state = builder.finish(10, 3, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;
    let row0 = &matrix.rows[0].glyphs[GlyphArea::Text.index()];
    let row0_right = &matrix.rows[0].glyphs[GlyphArea::RightMargin.index()];
    let row1 = &matrix.rows[1].glyphs[GlyphArea::Text.index()];
    let row1_right = &matrix.rows[1].glyphs[GlyphArea::RightMargin.index()];
    let row2 = &matrix.rows[2].glyphs[GlyphArea::Text.index()];
    let row2_right = &matrix.rows[2].glyphs[GlyphArea::RightMargin.index()];

    assert_eq!(row0.len(), 4);
    assert_eq!(row0_right.len(), 1);
    assert_eq!(row1.len(), 4);
    assert_eq!(
        row1.iter()
            .map(|glyph| match &glyph.glyph_type {
                GlyphType::Char { ch } => *ch,
                _ => '?',
            })
            .collect::<String>(),
        "    "
    );
    assert_eq!(row1_right.len(), 1);
    assert_eq!(row1_right[0].glyph_type, GlyphType::Char { ch: '|' });
    assert!(!matrix.rows[1].displays_text);
    assert_eq!(row2.len(), 4);
    assert_eq!(row2_right.len(), 1);
}

#[test]
fn text_window_right_border_preserves_trailing_truncation_marker() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 5, Rect::new(0.0, 0.0, 40.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    for ch in "ABCD$".chars() {
        write_char_to_current_row(&mut builder, ch, 0, 0);
    }
    builder.end_row();
    builder.end_window();

    install_last_window_right_border(
        &mut builder,
        TextWindowRightBorder {
            ch: '|',
            face_id: 21,
            char_width: 8.0,
        },
    );

    let state = builder.finish(5, 1, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;
    let row0_text = &matrix.rows[0].glyphs[GlyphArea::Text.index()];
    let row0_right = &matrix.rows[0].glyphs[GlyphArea::RightMargin.index()];
    let row0_chars: String = row0_text
        .iter()
        .map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } => *ch,
            _ => '?',
        })
        .collect();

    assert_eq!(row0_chars, "ABC$");
    assert_eq!(row0_right.len(), 1);
    assert_eq!(row0_right[0].glyph_type, GlyphType::Char { ch: '|' });
    assert_eq!(row0_right[0].face_id, 21);
}

#[test]
fn mark_current_text_row_truncated_left_sets_current_row_flag() {
    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 2, 5, Rect::new(0.0, 0.0, 40.0, 32.0), true);
    builder.begin_row(1, GlyphRowRole::Text);

    mark_current_text_row_truncated_left(&mut builder);

    builder.end_row();
    builder.end_window();
    let state = builder.finish(10, 2, 8.0, 16.0);
    let matrix = &state.window_matrices[0].matrix;
    assert!(!matrix.rows[0].truncated_left);
    assert!(matrix.rows[1].truncated_left);
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
    TextMatrixRowOutput::new(&mut builder, &mut emitter, &mut eval).begin(TextMatrixRowBegin {
        matrix_row: 0,
        row: 0,
        col: 0,
        y: 0.0,
        x: 0.0,
    });

    let transition = emit_text_matrix_row_transition_with_limit(
        &mut builder,
        &mut emitter,
        &mut eval,
        TextMatrixRowGeometryTransition {
            finished_row: TextMatrixRowMetrics {
                y: 0.0,
                height: 16.0,
                ascent: 12.0,
            },
            begin_row: TextMatrixRowBegin {
                matrix_row: 1,
                row: 1,
                col: 0,
                y: 16.0,
                x: 0.0,
            },
        },
        1,
    );

    assert_eq!(transition, TextMatrixRowTransition::ExhaustedRows);
    assert_eq!(emitter.rows().len(), 1);

    builder.end_window();
    let state = builder.finish(10, 1, 8.0, 16.0);
    assert_eq!(state.window_matrices[0].matrix.rows.len(), 1);
}

#[test]
fn text_matrix_row_transition_emits_finish_and_begin() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("output-emitter-row-transition", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = GlyphMatrixBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(0.0, 0.0, 80.0, 32.0), true);

    let mut emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    emitter.begin_update(&mut eval);
    TextMatrixRowOutput::new(&mut builder, &mut emitter, &mut eval).begin(TextMatrixRowBegin {
        matrix_row: 0,
        row: 0,
        col: 0,
        y: 0.0,
        x: 0.0,
    });

    emit_text_matrix_row_transition(
        &mut builder,
        &mut emitter,
        &mut eval,
        TextMatrixRowGeometryTransition {
            finished_row: TextMatrixRowMetrics {
                y: 0.0,
                height: 16.0,
                ascent: 12.0,
            },
            begin_row: TextMatrixRowBegin {
                matrix_row: 1,
                row: 1,
                col: 0,
                y: 16.0,
                x: 0.0,
            },
        },
    );

    assert_eq!(emitter.rows().len(), 1);
    assert_eq!(emitter.rows()[0].row, 0);
    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");
    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 0,
            y: 16,
            row: 1,
            col: 0,
        })
    );

    builder.end_row();
    builder.end_window();
    let state = builder.finish(10, 1, 8.0, 16.0);
    assert_eq!(state.window_matrices[0].matrix.rows.len(), 2);
}

#[test]
fn text_matrix_row_transition_reports_exhausted_state() {
    assert!(TextMatrixRowTransition::ExhaustedRows.is_exhausted());
    assert!(!TextMatrixRowTransition::BeganNextRow.is_exhausted());
}
