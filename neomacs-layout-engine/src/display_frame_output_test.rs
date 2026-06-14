use super::*;
use crate::types::{FrameParams, WindowParams};
use neomacs_display_protocol::cursor::CursorBarWidth;
use neomacs_display_protocol::frame_glyphs::{CursorKind, WindowInfo};
use neomacs_display_protocol::types::Rect;

fn window_params() -> WindowParams {
    WindowParams {
        window_id: 41,
        buffer_id: 7,
        bounds: Rect::new(10.0, 20.0, 120.0, 100.0),
        text_bounds: Rect::new(20.0, 30.0, 80.0, 70.0),
        selected: true,
        is_minibuffer: false,
        window_start: 10,
        window_end: 80,
        point: 10,
        buffer_size: 210,
        buffer_begv: 0,
        hscroll: 4,
        vscroll: 0,
        truncate_lines: false,
        word_wrap: false,
        tab_width: 8,
        tab_stop_list: Vec::new(),
        default_fg: 0,
        default_bg: 0,
        char_width: 8.0,
        char_height: 16.0,
        window_system: true,
        font_pixel_size: 14.0,
        font_ascent: 11.0,
        mode_line_height: 10.0,
        header_line_height: 6.0,
        tab_line_height: 4.0,
        cursor_kind: CursorKind::FilledBox,
        cursor_bar_width: CursorBarWidth::default(),
        x_stretch_cursor: false,
        cursor_color: 0,
        cursor_effects: None,
        visual_cursors: Vec::new(),
        left_fringe_width: 0.0,
        right_fringe_width: 0.0,
        indicate_empty_lines: 0,
        show_trailing_whitespace: false,
        trailing_ws_bg: 0,
        fill_column_indicator: -1,
        fill_column_indicator_char: '|',
        fill_column_indicator_fg: 0,
        extra_line_spacing: 0.0,
        selective_display: 0,
        escape_glyph_fg: 0,
        nobreak_char_display: 0,
        nobreak_char_fg: 0,
        glyphless_char_fg: 0,
        wrap_prefix: Vec::new(),
        line_prefix: Vec::new(),
        left_margin_width: 0.0,
        right_margin_width: 0.0,
        vertical_scroll_bar_side: Some("right".to_string()),
        horizontal_scroll_bar: true,
        scroll_bar_pixel_width: 12.0,
        scroll_bar_pixel_height: 8.0,
    }
}

fn frame_params() -> FrameParams {
    FrameParams {
        width: 180.0,
        height: 140.0,
        menu_bar_height: 0.0,
        tool_bar_height: 0.0,
        compact_bar_height: 0.0,
        tab_bar_height: 0.0,
        char_width: 8.0,
        char_height: 16.0,
        font_pixel_size: 14.0,
        window_system: true,
        background: 0x101010,
        vertical_border_fg: 0x202020,
        right_divider_width: 6,
        bottom_divider_width: 5,
        divider_fg: 0x303030,
        divider_first_fg: 0x404040,
        divider_last_fg: 0x505050,
    }
}

fn window_info(params: &WindowParams) -> WindowInfo {
    WindowInfo {
        window_id: params.window_id,
        buffer_id: params.buffer_id,
        window_start: 31,
        window_end: 101,
        buffer_size: params.buffer_size,
        bounds: params.bounds,
        mode_line_height: params.mode_line_height,
        header_line_height: params.header_line_height,
        tab_line_height: params.tab_line_height,
        selected: params.selected,
        is_minibuffer: params.is_minibuffer,
        char_height: params.char_height,
        buffer_file_name: String::new(),
        modified: false,
    }
}

#[test]
fn window_frame_geometry_reserves_terminal_border_column() {
    let params = window_params();
    let mut frame = frame_params();
    frame.window_system = false;
    frame.right_divider_width = 0;

    let geometry = WindowFrameGeometryRequest::new(&params, &frame, 200.0).resolve();

    assert_eq!(geometry.right_edge, 130.0);
    assert_eq!(geometry.bottom_edge, 120.0);
    assert!(!geometry.is_rightmost);
    assert!(!geometry.is_bottommost);
    assert!(geometry.reserve_terminal_right_border_col);
}

#[test]
fn window_frame_info_request_emits_background_and_window_info() {
    let params = window_params();
    let metadata = WindowFrameMetadata {
        buffer_file_name: "notes.org".to_string(),
        modified: true,
    };
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();

    WindowFrameInfoRenderRequest::new(&params, metadata).render_and_apply(&mut builder);

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.backgrounds.len(), 1);
    assert_eq!(state.backgrounds[0].bounds, params.bounds);
    assert_eq!(state.backgrounds[0].color.r, 0.0);
    assert_eq!(state.window_infos.len(), 1);
    assert_eq!(state.window_infos[0].window_id, params.window_id);
    assert_eq!(state.window_infos[0].buffer_file_name, "notes.org");
    assert!(state.window_infos[0].modified);
}

#[test]
fn window_frame_info_effects_request_emits_scroll_effect_hints() {
    let params = window_params();
    let mut prev = window_info(&params);
    prev.window_start = 11;
    prev.window_end = 81;
    let curr = window_info(&params);
    let mut prev_infos = std::collections::HashMap::new();
    prev_infos.insert(prev.window_id, prev);
    let mut curr_infos = std::collections::HashMap::new();
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.push_window_info(curr);

    WindowFrameInfoEffectsRenderRequest::new(&prev_infos)
        .render_latest_and_apply(&mut builder, &mut curr_infos);

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(curr_infos.len(), 1);
    assert_eq!(state.effect_hints.len(), 4);
}

#[test]
fn window_divider_request_splits_wide_vertical_divider() {
    let frame = frame_params();
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();

    WindowDividerRectsRenderRequest::new(
        41,
        118.0,
        20.0,
        6.0,
        80.0,
        WindowDividerOrientation::Vertical,
        &frame,
    )
    .render_and_apply(&mut builder);

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.borders.len(), 3);
    assert_eq!(state.borders[0].width, 1.0);
    assert_eq!(state.borders[1].width, 4.0);
    assert_eq!(state.borders[2].width, 1.0);
}

#[test]
fn vertical_scroll_bar_metrics_follow_visible_buffer_span() {
    let metrics = WindowScrollBarMetrics::vertical(31, 101, 0, 210, 70.0);

    assert_eq!(metrics.position, 30);
    assert_eq!(metrics.portion, 70);
    assert_eq!(metrics.whole, 210);
    assert!((metrics.thumb_start - 10.0).abs() < f32::EPSILON);
    assert!((metrics.thumb_size - 23.333334).abs() < 0.0001);
}

#[test]
fn window_scroll_bars_request_emits_vertical_and_horizontal_items() {
    let params = window_params();
    let info = window_info(&params);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();

    WindowScrollBarsRenderRequest::new(&params, &info).render_and_apply(&mut builder);

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.scroll_bars.len(), 2);

    let vertical = state
        .scroll_bars
        .iter()
        .find(|bar| !bar.horizontal)
        .expect("vertical scroll bar");
    assert_eq!(vertical.window_id, 41);
    assert_eq!(vertical.x, 118.0);
    assert_eq!(vertical.y, 30.0);
    assert_eq!(vertical.width, 12.0);
    assert_eq!(vertical.height, 72.0);
    assert_eq!(vertical.position, 30);
    assert_eq!(vertical.portion, 70);

    let horizontal = state
        .scroll_bars
        .iter()
        .find(|bar| bar.horizontal)
        .expect("horizontal scroll bar");
    assert_eq!(horizontal.window_id, 41);
    assert_eq!(horizontal.x, 10.0);
    assert_eq!(horizontal.y, 102.0);
    assert_eq!(horizontal.width, 120.0);
    assert_eq!(horizontal.height, 8.0);
    assert_eq!(horizontal.position, 4);
}

#[test]
fn window_scroll_bars_request_skips_empty_vertical_track() {
    let mut params = window_params();
    params.bounds.height = params.header_line_height
        + params.tab_line_height
        + params.mode_line_height
        + params.scroll_bar_pixel_height;
    params.horizontal_scroll_bar = false;
    let info = window_info(&params);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();

    WindowScrollBarsRenderRequest::new(&params, &info).render_and_apply(&mut builder);

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert!(state.scroll_bars.is_empty());
}
