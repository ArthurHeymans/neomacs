use super::*;
use crate::display_row_geometry::DisplayRowMarker;
use crate::types::WindowKind;
use neomacs_display_protocol::types::Rect;
use neovm_core::window::{FrameId, WindowId};

fn window_params() -> WindowParams {
    WindowParams {
        window_id: 1,
        buffer_id: 1,
        bounds: Rect::new(0.0, 8.0, 240.0, 120.0),
        text_bounds: Rect::new(16.0, 32.0, 160.0, 80.0),
        selected: true,
        kind: WindowKind::Main,
        window_start: 17,
        window_end: 0,
        point: 17,
        buffer_size: 80,
        buffer_begv: 1,
        hscroll: 0,
        vscroll: 0,
        wrap_mode: LineWrapMode::Wrap,
        word_wrap: false,
        tab_width: 8,
        tab_stop_list: vec![],
        default_fg: 0x00ff_ffff,
        default_bg: 0,
        char_width: 8.0,
        char_height: 16.0,
        window_system: true,
        font_pixel_size: 14.0,
        font_ascent: 11.0,
        mode_line_height: 0.0,
        header_line_height: 0.0,
        tab_line_height: 0.0,
        cursor_kind: neomacs_display_protocol::frame_glyphs::CursorKind::FilledBox,
        cursor_bar_width: neomacs_display_protocol::cursor::CursorBarWidth::TWO,
        x_stretch_cursor: false,
        cursor_color: 0x00ff_ffff,
        cursor_effects: None,
        visual_cursors: Vec::new(),
        left_fringe_width: 8.0,
        right_fringe_width: 8.0,
        indicate_empty_lines: 2,
        show_trailing_whitespace: false,
        trailing_ws_bg: 0,
        fill_column_indicator: 3,
        fill_column_indicator_char: '|',
        fill_column_indicator_fg: 0,
        extra_line_spacing: 0.0,
        selective_display: 0,
        escape_glyph_fg: 0,
        nobreak_char_display: 0,
        nobreak_char_fg: 0,
        glyphless_char_fg: 0,
        wrap_prefix: vec![],
        line_prefix: vec![],
        left_margin_width: 0.0,
        right_margin_width: 0.0,
        vertical_scroll_bar_side: None,
        horizontal_scroll_bar: false,
        scroll_bar_pixel_width: 0.0,
        scroll_bar_pixel_height: 0.0,
    }
}

fn setup_request() -> BufferTextWindowWalkSetupRequest<'static> {
    BufferTextWindowWalkSetupRequest::new(
        17,
        24.0,
        16.0,
        160.0,
        32.0,
        8.0,
        12.0,
        5,
        8.0,
        16.0,
        11.0,
        LineWrapMode::Truncate,
        3,
        true,
        true,
        true,
        true,
        false,
        4,
        &[4, 12],
        true,
        0x00ff00,
    )
}

#[test]
fn geometry_request_derives_text_area_and_matrix_rows() {
    let params = window_params();
    let request = BufferTextWindowGeometryRequest::new(&params, 8.0, 16.0, 12.0, 10.0, 6.0);

    assert_eq!(request.line_number_row_capacity(), 5);

    let geometry = request.into_geometry(3);

    assert_eq!(geometry.text_x, 16.0);
    assert_eq!(geometry.text_y, 48.0);
    assert_eq!(geometry.text_width, 160.0);
    assert_eq!(geometry.text_height, 92.0);
    assert_eq!(geometry.char_width, 8.0);
    assert_eq!(geometry.char_height, 16.0);
    assert_eq!(geometry.max_rows, 5);
    assert_eq!(geometry.text_matrix_row_base, 2);
    assert_eq!(geometry.text_matrix_rows, 5);
    assert_eq!(geometry.bottom_chrome_rows, 1);
    assert_eq!(geometry.mode_line_matrix_row, 7);
    assert_eq!(geometry.line_number_pixel_width, 24.0);
    assert_eq!(geometry.content_x, 40.0);
    assert_eq!(geometry.cols, 17);
}

#[test]
fn geometry_request_only_forces_fractional_row_for_minibuffer() {
    let mut params = window_params();
    params.bounds.height = 15.0;
    params.text_bounds.height = 15.0;

    let ordinary =
        BufferTextWindowGeometryRequest::new(&params, 8.0, 16.0, 0.0, 0.0, 0.0).into_geometry(0);
    assert_eq!(ordinary.max_rows, 0);

    params.kind = WindowKind::Minibuffer;
    let minibuffer =
        BufferTextWindowGeometryRequest::new(&params, 8.0, 16.0, 0.0, 0.0, 0.0).into_geometry(0);
    assert_eq!(minibuffer.max_rows, 1);
}

#[test]
fn geometry_request_measures_minibuffer_up_to_max_mini_window_rows() {
    // GNU `resize_mini_window` measures the mini-window content unclamped and
    // clips only to `max-mini-window-height`. `with_max_mini_window_rows` lets
    // the walk emit up to that ceiling even when the window is one row tall.
    let mut params = window_params();
    params.kind = WindowKind::Minibuffer;
    // One physical row tall (16px), but a ceiling of 3 rows.
    params.bounds.height = 16.0;
    params.text_bounds.height = 16.0;
    let request = BufferTextWindowGeometryRequest::new(&params, 8.0, 16.0, 0.0, 0.0, 0.0)
        .with_max_mini_window_rows(3);

    let geometry = request.into_geometry(0);

    assert_eq!(geometry.max_rows, 3);
    assert_eq!(geometry.text_matrix_rows, 3);
    assert_eq!(geometry.mode_line_matrix_row, 3);
    // The visibility bottom is lifted so the walk can emit all three rows even
    // though the window is physically one row tall.
    assert_eq!(geometry.visibility_bottom_y, geometry.text_y + 3.0 * 16.0);
}

#[test]
fn geometry_request_does_not_apply_max_mini_window_rows_to_ordinary_windows() {
    let mut params = window_params();
    params.bounds.height = 16.0;
    params.text_bounds.height = 16.0;
    let request = BufferTextWindowGeometryRequest::new(&params, 8.0, 16.0, 0.0, 0.0, 0.0)
        .with_max_mini_window_rows(5);

    let geometry = request.into_geometry(0);

    // Ordinary windows ignore the minibuffer ceiling and keep the physical
    // row count and physical visibility bottom.
    assert_eq!(geometry.max_rows, 1);
    assert_eq!(geometry.visibility_bottom_y, geometry.text_y + 16.0);
}

#[test]
fn walk_setup_initializes_source_position_and_geometry_state() {
    let setup = setup_request().into_setup();

    assert_eq!(setup.byte_idx, 0);
    assert_eq!(setup.charpos, 17);
    assert_eq!(setup.x, 24.0);
    assert_eq!(setup.col, 0);
    assert_eq!(setup.text_area_left, 16.0);
    assert_eq!(setup.window_top, 8.0);
    assert_eq!(setup.row_flags.len(), 5);
    assert_eq!(setup.row_geometry.row(), 0);
    assert_eq!(setup.row_geometry.y(), 32.0);
    assert_eq!(setup.row_geometry.height(), 16.0);
    assert_eq!(setup.row_geometry.ascent(), 11.0);
    assert_eq!(setup.hit_row_range.start(), 17);
}

#[test]
fn walk_setup_applies_hscroll_prefix_and_reserved_surface_policy() {
    let setup = setup_request().into_setup();

    assert!(setup.hscroll_skip.should_skip());
    assert_eq!(setup.hscroll_skip.consumed_columns(), 0);
    assert!(setup.prefix_request.is_requested());
    assert_eq!(setup.text_append_surface.content_x(), 24.0);
    assert_eq!(setup.text_append_surface.right_edge(), 164.0);
    assert!(setup.trailing_whitespace.background().is_some());
}

#[test]
fn output_setup_derives_begin_request_and_row_limits_from_walk_setup() {
    let walk_setup = setup_request().into_setup();
    let output_setup = BufferTextWindowOutputSetupRequest::new(
        FrameId(3),
        WindowId(9),
        99,
        2,
        6,
        1,
        0,
        Rect::new(0.0, 8.0, 240.0, 120.0),
        Rect::new(16.0, 32.0, 160.0, 80.0),
        true,
        32.0,
        48.0,
        80.0,
    )
    .into_setup(5, &walk_setup);

    assert_eq!(output_setup.row_visibility_limit.max_rows, 5);
    assert_eq!(output_setup.row_visibility_limit.bottom_y, 80.0);
    assert_eq!(output_setup.row_limit.max_rows, 5);
    assert_eq!(output_setup.body_install_context.matrix_cols(), 1);
    assert_eq!(output_setup.retry_bounds.text_area_top, 24);
    assert_eq!(output_setup.retry_bounds.text_area_bottom, 72);
}

#[test]
fn loop_request_context_carries_buffer_and_window_policy() {
    let params = window_params();
    let walk_setup = setup_request().into_setup();
    let output_setup = BufferTextWindowOutputSetupRequest::new(
        FrameId(3),
        WindowId(9),
        99,
        2,
        6,
        1,
        20,
        params.bounds,
        params.text_bounds,
        params.selected,
        32.0,
        48.0,
        80.0,
    )
    .into_setup(5, &walk_setup);
    let context = BufferTextWindowLoopRequestContext::new(
        neovm_core::buffer::BufferId(42),
        11,
        80,
        17,
        &params,
        24.0,
        true,
        11.0,
        16.0,
        8.0,
        output_setup.row_visibility_limit,
        walk_setup.row_geometry_defaults,
        2,
        5,
        output_setup.row_limit,
    );

    assert_eq!(context.buffer_id(), neovm_core::buffer::BufferId(42));
    assert_eq!(context.text_start_byte(), 11);
    assert_eq!(context.accessible_end(), 80);
    assert_eq!(context.selective_display(), params.selective_display);
    assert_eq!(context.tab_width(), params.tab_width);
    assert_eq!(context.row_limit(), output_setup.row_limit);
}

#[test]
fn row_prelude_request_context_carries_margin_and_prefix_policy() {
    let prefix_values =
        crate::display_row_append::DisplayRowPrefixValues::default_values(None, None);
    let context =
        BufferTextWindowRowPreludeRequestContext::new(2, true, 3, 4, 5, prefix_values, 8.0, 16.0);

    assert_eq!(context.line_number_mode(), 2);
    assert_eq!(context.prefix_values(), prefix_values);
    assert_eq!(context.char_width(), 8.0);
}

#[test]
fn local_display_policy_builds_row_prelude_context() {
    let prefix_values =
        crate::display_row_append::DisplayRowPrefixValues::default_values(None, None);
    let policy = BufferTextWindowLocalDisplayPolicy::from_parts(2, false, 3, prefix_values);
    let context = policy.row_prelude_context(6, 8.0, 16.0);

    assert!(!policy.has_prefix());
    assert!(!policy.has_line_default_prefix());
    assert_eq!(context.line_number_mode(), 2);
    assert_eq!(context.prefix_values(), prefix_values);
    assert_eq!(context.char_width(), 8.0);
}

#[test]
fn tail_decoration_request_reports_rows_considered_for_decorations() {
    let mut setup = setup_request().into_setup();
    setup.row_geometry = DisplayRowGeometryState::new(2, 64.0, 0.0, 16.0, 11.0);
    setup.row_y_positions.record(1, 48.0);
    setup.row_y_positions.record(2, 64.0);
    setup.row_flags.mark(0, DisplayRowFlagKind::Continued);
    setup.row_flags.mark(0, DisplayRowFlagKind::Truncated);
    setup.row_flags.mark(1, DisplayRowFlagKind::Continuation);
    setup
        .row_extend
        .activate(DisplayRowMarker::Row(2), (Color::from_pixel(0x101010), 7));
    setup.box_face.activate(DisplayRowMarker::Row(2), 24.0);

    let params = window_params();
    let output_setup = BufferTextWindowOutputSetupRequest::new(
        FrameId(3),
        WindowId(9),
        99,
        2,
        6,
        1,
        20,
        params.bounds,
        params.text_bounds,
        params.selected,
        32.0,
        80.0,
        112.0,
    )
    .into_setup(5, &setup);

    let context = BufferTextWindowTailRequestContext::new(
        &params,
        11,
        1,
        80,
        4,
        2,
        setup.text_area_left,
        setup.window_top,
        32.0,
        80.0,
        24.0,
        20,
        8.0,
        16.0,
        Color::from_pixel(0x00ff_ffff),
        5,
        output_setup.row_limit,
        setup.row_geometry_defaults,
        output_setup.retry_bounds,
        output_setup.body_install_context,
        true,
        false,
        12.0,
        0.0,
        0.0,
    );

    assert_eq!(context.window_start(), 11);
    assert_eq!(context.accessible_range(), (1, 80));

    let outcome = context
        .tail_decoration_request()
        .apply(BufferTextWindowTailDecorationState {
            x: 40.0,
            text_append_surface: &setup.text_append_surface,
            row_geometry: &setup.row_geometry,
            row_y_positions: &setup.row_y_positions,
            row_flags: &setup.row_flags,
            row_extend: &setup.row_extend,
            box_face: &setup.box_face,
        });

    assert!(outcome.box_face_active);
    assert!(outcome.row_extend_active);
    assert!(outcome.current_row_extended);
    assert_eq!(outcome.empty_extend_rows, 2);
    assert_eq!(outcome.fringe_rows, 2);
    assert_eq!(outcome.right_continuation_rows, 1);
    assert_eq!(outcome.right_truncation_rows, 1);
    assert_eq!(outcome.left_continuation_rows, 1);
    assert_eq!(outcome.empty_line_fringe_rows, 3);
    assert_eq!(outcome.fill_column_rows, 2);
}
