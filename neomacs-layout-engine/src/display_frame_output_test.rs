use super::*;
use crate::types::{FrameParams, LineWrapMode, WindowKind, WindowParams};
use neomacs_display_protocol::cursor::CursorBarWidth;
use neomacs_display_protocol::frame_chrome::FrameChromeKind;
use neomacs_display_protocol::frame_glyphs::{
    CursorKind, CursorStyle, DisplaySlotId, PresentedCellOrigin, PresentedWindowGeometry,
    PresentedWindowRegions, WindowEffectHint, WindowInfo, WindowTransitionKind,
};
use neomacs_display_protocol::types::{Color, DisplayWindowId, Rect};

fn install_skipped_geometry(
    builder: &mut crate::output::builder::DisplayOutputBuilder,
    window_id: DisplayWindowId,
    outer: Rect,
) {
    builder.install_window_metadata(
        crate::output::install_request::OutputPresentedWindowGeometryInstallRequest {
            window_id,
            geometry: PresentedWindowGeometry::Skipped {
                cell_origin: PresentedCellOrigin::default(),
                outer,
            },
        },
    );
}

fn window_params() -> WindowParams {
    WindowParams {
        space_image_catalog: None,
        window_id: 41,
        buffer_id: 7,
        bounds: Rect::new(10.0, 20.0, 120.0, 100.0),
        text_bounds: Rect::new(20.0, 30.0, 80.0, 70.0),
        selected: true,
        kind: WindowKind::Main,
        left_col: 0,
        top_line: 0,
        window_start: 10,
        force_start: false,
        previous_visible_end: None,
        point: 10,
        buffer_size: 210,
        buffer_begv: 0,
        hscroll: 4,
        vscroll: 0,
        wrap_mode: LineWrapMode::Wrap,
        word_wrap: false,
        tab_width: 8,
        scroll_conservatively: 0,
        scroll_step: 0,
        scroll_minibuffer_conservatively: true,
        scroll_margin: 0,
        tab_stop_list: Vec::new(),
        default_fg: 0,
        default_bg: 0,
        char_width: 8.0,
        char_height: 16.0,
        window_system: true,
        font_pixel_size: 14.0,
        image_scale_environment: Default::default(),
        font_ascent: 11.0,
        mode_line_height: 10.0,
        header_line_height: 6.0,
        tab_line_height: 4.0,
        cursor_kind: CursorKind::FilledBox,
        cursor_bar_width: CursorBarWidth::default(),
        x_stretch_cursor: false,
        cursor_color: 0,
        cursor_foreground: 0,
        cursor_effects: None,
        visual_cursors: Vec::new(),
        left_fringe_width: 0.0,
        right_fringe_width: 0.0,
        fringes_outside_margins: false,
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
        left_margin_columns: 0,
        right_margin_width: 0.0,
        right_margin_columns: 0,
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
        image_scale_environment: Default::default(),
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

#[test]
fn presented_window_regions_resolve_all_bands_from_measured_geometry() {
    use crate::window_layout::{WindowChromeMetrics, WindowDividerLayout, WindowLayoutBox};
    use neomacs_display_protocol::types::Rect as EvaluatorRect;

    let mut params = window_params();
    params.bounds = Rect::new(144.0, 24.0, 400.0, 300.0);
    params.text_bounds = Rect::new(180.0, 24.0, 330.0, 300.0);
    params.left_margin_width = 16.0;
    params.left_margin_columns = 2;
    params.right_margin_width = 24.0;
    params.right_margin_columns = 3;
    params.left_fringe_width = 8.0;
    params.right_fringe_width = 10.0;
    params.fringes_outside_margins = true;
    params.vertical_scroll_bar_side = Some("left".to_string());
    params.scroll_bar_pixel_width = 12.0;
    params.horizontal_scroll_bar = true;
    params.scroll_bar_pixel_height = 8.0;

    let frame = frame_params();
    let geometry = WindowFrameGeometry {
        right_edge: 544.0,
        bottom_edge: 324.0,
        is_rightmost: false,
        is_bottommost: false,
        reserve_terminal_right_border_col: false,
    };
    let measured = WindowChromeMetrics {
        mode_line_height: 17.0,
        header_line_height: 7.0,
        tab_line_height: 5.0,
    };

    let regions = WindowLayoutBox::resolve(
        &params,
        measured,
        WindowDividerLayout::resolve(&params, &frame, geometry),
    )
    .regions();

    assert_eq!(regions.outer, EvaluatorRect::new(144.0, 24.0, 400.0, 300.0));
    assert_eq!(regions.left_margin_columns, 2);
    assert_eq!(regions.right_margin_columns, 3);
    assert_eq!(
        regions.left_scroll_bar,
        Some(EvaluatorRect::new(144.0, 36.0, 12.0, 258.0))
    );
    assert_eq!(
        regions.left_fringe,
        Some(EvaluatorRect::new(156.0, 36.0, 8.0, 258.0))
    );
    assert_eq!(
        regions.left_margin,
        Some(EvaluatorRect::new(164.0, 36.0, 16.0, 258.0))
    );
    assert_eq!(
        regions.text_body,
        EvaluatorRect::new(180.0, 36.0, 324.0, 258.0)
    );
    assert_eq!(
        regions.right_margin,
        Some(EvaluatorRect::new(504.0, 36.0, 24.0, 258.0))
    );
    assert_eq!(
        regions.right_fringe,
        Some(EvaluatorRect::new(528.0, 36.0, 10.0, 258.0))
    );
    assert_eq!(regions.right_scroll_bar, None);
    assert_eq!(
        regions.tab_line,
        Some(EvaluatorRect::new(144.0, 24.0, 394.0, 5.0))
    );
    assert_eq!(
        regions.header_line,
        Some(EvaluatorRect::new(144.0, 29.0, 394.0, 7.0))
    );
    assert_eq!(
        regions.horizontal_scroll_bar,
        Some(EvaluatorRect::new(144.0, 294.0, 394.0, 8.0))
    );
    assert_eq!(
        regions.mode_line,
        Some(EvaluatorRect::new(144.0, 302.0, 394.0, 17.0))
    );
    assert_eq!(
        regions.right_divider,
        Some(EvaluatorRect::new(538.0, 24.0, 6.0, 295.0))
    );
    assert_eq!(
        regions.bottom_divider,
        Some(EvaluatorRect::new(144.0, 319.0, 394.0, 5.0))
    );
    let right_divider = regions.right_divider.expect("right divider");
    for region in [
        regions.tab_line.expect("tab line"),
        regions.header_line.expect("header line"),
        regions.mode_line.expect("mode line"),
        regions
            .horizontal_scroll_bar
            .expect("horizontal scroll bar"),
        regions.right_fringe.expect("right fringe"),
    ] {
        assert!(
            region.x + region.width <= right_divider.x,
            "window content must not overlap its right divider: {region:?} vs {right_divider:?}"
        );
    }
    let bottom_divider = regions.bottom_divider.expect("bottom divider");
    assert!(
        regions.mode_line.expect("mode line").y + regions.mode_line.expect("mode line").height
            <= bottom_divider.y,
        "window chrome must not overlap its bottom divider"
    );
}

#[test]
fn frame_output_owner_publishes_canonical_frame_chrome_order() {
    use neomacs_display_protocol::frame_chrome::{
        FrameChromeContent, MenuBarContent, ToolBarContent,
    };

    let mut owner = FrameOutputOwner::new();
    owner.add_frame_chrome_band(ChromeBandRequest::new(
        FrameChromeKind::TabBar,
        18.0,
        FrameChromeContent::DisplayRow(
            neomacs_display_protocol::frame_chrome::ChromeDisplayRow::empty_tab_bar(),
        ),
    ));
    owner.add_frame_chrome_band(ChromeBandRequest::new(
        FrameChromeKind::ToolBar,
        34.0,
        FrameChromeContent::ToolBar(ToolBarContent::empty()),
    ));
    owner.add_frame_chrome_band(ChromeBandRequest::new(
        FrameChromeKind::MenuBar,
        18.0,
        FrameChromeContent::MenuBar(MenuBarContent::empty()),
    ));

    let state = owner.finish(&frame_params()).expect("valid frame chrome");
    let bands = state.frame_chrome.bands();
    assert_eq!(bands.len(), 3);
    assert_eq!(bands[0].kind(), FrameChromeKind::MenuBar);
    assert_eq!(bands[0].bounds().y(), 0.0);
    assert_eq!(bands[1].kind(), FrameChromeKind::ToolBar);
    assert_eq!(bands[1].bounds().y(), 18.0);
    assert_eq!(bands[2].kind(), FrameChromeKind::TabBar);
    assert_eq!(bands[2].bounds().y(), 52.0);
}

#[test]
fn frame_output_owner_uses_compact_bar_instead_of_split_menu_and_tool_bars() {
    use neomacs_display_protocol::frame_chrome::{CompactBarContent, FrameChromeContent};

    let mut owner = FrameOutputOwner::new();
    owner.add_frame_chrome_band(ChromeBandRequest::new(
        FrameChromeKind::CompactBar,
        34.0,
        FrameChromeContent::CompactBar(CompactBarContent::empty()),
    ));
    owner.add_frame_chrome_band(ChromeBandRequest::new(
        FrameChromeKind::TabBar,
        18.0,
        FrameChromeContent::DisplayRow(
            neomacs_display_protocol::frame_chrome::ChromeDisplayRow::empty_tab_bar(),
        ),
    ));

    let state = owner.finish(&frame_params()).expect("valid frame chrome");
    let bands = state.frame_chrome.bands();
    assert_eq!(bands.len(), 2);
    assert_eq!(bands[0].kind(), FrameChromeKind::CompactBar);
    assert_eq!(bands[0].bounds().y(), 0.0);
    assert_eq!(bands[1].kind(), FrameChromeKind::TabBar);
    assert_eq!(bands[1].bounds().y(), 34.0);
}

fn window_info(params: &WindowParams) -> WindowInfo {
    let body_y = params.bounds.y + params.tab_line_height + params.header_line_height;
    let body_height = params.bounds.height
        - params.tab_line_height
        - params.header_line_height
        - params.mode_line_height
        - if params.horizontal_scroll_bar {
            params.scroll_bar_pixel_height
        } else {
            0.0
        };
    let regions = PresentedWindowRegions {
        outer: params.bounds,
        text_body: Rect::new(
            params.text_bounds.x,
            body_y,
            params.text_bounds.width,
            body_height,
        ),
        right_scroll_bar: (body_height > 0.0).then(|| {
            Rect::new(
                params.bounds.x + params.bounds.width - params.scroll_bar_pixel_width,
                body_y,
                params.scroll_bar_pixel_width,
                body_height,
            )
        }),
        horizontal_scroll_bar: params.horizontal_scroll_bar.then(|| {
            Rect::new(
                params.bounds.x,
                body_y + body_height,
                params.bounds.width,
                params.scroll_bar_pixel_height,
            )
        }),
        ..PresentedWindowRegions::default()
    };
    WindowInfo {
        window_id: DisplayWindowId::new(params.window_id),
        buffer_id: params.buffer_id,
        buffer_name: String::new(),
        window_start: 31,
        window_end: 101,
        buffer_size: params.buffer_size,
        bounds: params.bounds,
        geometry: neomacs_display_protocol::frame_glyphs::PresentedWindowGeometry::Complete {
            cell_origin: Default::default(),
            regions,
        },
        mode_line_height: params.mode_line_height,
        header_line_height: params.header_line_height,
        tab_line_height: params.tab_line_height,
        selected: params.selected,
        is_minibuffer: params.is_minibuffer(),
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
        buffer_name: "notes".to_string(),
        buffer_file_name: "notes.org".to_string(),
        modified: true,
    };
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();

    WindowFrameInfoRenderRequest::new(&params, metadata)
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));
    install_skipped_geometry(
        &mut builder,
        DisplayWindowId::new(params.window_id),
        params.bounds,
    );

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.backgrounds.len(), 1);
    assert_eq!(state.backgrounds[0].bounds, params.bounds);
    assert_eq!(state.backgrounds[0].color.r, 0.0);
    assert_eq!(state.window_infos.len(), 1);
    assert_eq!(state.window_infos[0].window_id.get(), params.window_id);
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
    let mut prev_infos = std::collections::HashMap::default();
    prev_infos.insert(prev.window_id, prev);
    let mut curr_infos = std::collections::HashMap::default();
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();
    builder.add_output_window_info(curr.clone());
    install_skipped_geometry(&mut builder, curr.window_id, curr.bounds);

    WindowFrameInfoEffectsRenderRequest::new(&prev_infos).render_latest_and_apply(
        FrameOutputTarget::from_builder(&mut builder),
        &mut curr_infos,
    );

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(curr_infos.len(), 1);
    assert_eq!(state.effect_hints.len(), 4);
}

#[test]
fn frame_line_animation_request_uses_cursor_y_for_buffer_size_change() {
    let params = window_params();
    let mut prev = window_info(&params);
    prev.buffer_size = 200;
    let mut curr = window_info(&params);
    curr.buffer_size = 210;
    let mut prev_infos = std::collections::HashMap::default();
    prev_infos.insert(prev.window_id, prev);
    let mut curr_infos = std::collections::HashMap::default();
    curr_infos.insert(curr.window_id, curr);
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();
    builder.add_output_cursor(
        params.window_id,
        DisplaySlotId {
            window_id: DisplayWindowId::new(params.window_id),
            row: 1,
            col: 2,
        },
        24.0,
        48.0,
        8.0,
        16.0,
        CursorStyle::FilledBox,
        Color::WHITE,
    );

    FrameLineAnimationHintsRenderRequest::new(&prev_infos, &curr_infos)
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.effect_hints.len(), 1);
    assert!(matches!(
        state.effect_hints[0],
        WindowEffectHint::LineAnimation {
            window_id,
            edit_y,
            offset: -16.0,
            ..
        } if window_id.get() == 41 && (edit_y - 64.0).abs() < f32::EPSILON
    ));
}

#[test]
fn frame_window_switch_request_emits_fade_and_updates_selected_state() {
    let params = window_params();
    let info = window_info(&params);
    let mut prev_selected = DisplayWindowId::new(7);
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();
    builder.add_output_window_info(info.clone());
    install_skipped_geometry(&mut builder, info.window_id, info.bounds);

    FrameWindowSwitchHintRenderRequest::new(&mut prev_selected)
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(prev_selected.get(), 41);
    assert!(matches!(
        state.effect_hints.as_slice(),
        [WindowEffectHint::WindowSwitchFade {
            window_id,
            ..
        }] if window_id.get() == 41
    ));
}

#[test]
fn frame_theme_transition_request_uses_content_height_before_minibuffer() {
    let params = window_params();
    let info = window_info(&params);
    let mut mini = info.clone();
    mini.window_id = DisplayWindowId::new(99);
    mini.is_minibuffer = true;
    mini.bounds = Rect::new(0.0, 96.0, 180.0, 24.0);
    let mut prev_background = Some((0.0, 0.0, 0.0, 1.0));
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();
    builder.set_output_background_color(Color::new(0.2, 0.0, 0.0, 1.0));
    builder.add_output_window_info(info.clone());
    builder.add_output_window_info(mini.clone());
    install_skipped_geometry(&mut builder, info.window_id, info.bounds);
    install_skipped_geometry(&mut builder, mini.window_id, mini.bounds);

    FrameThemeTransitionHintRenderRequest::new(&mut prev_background, 180.0, 140.0)
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(prev_background, Some((0.2, 0.0, 0.0, 1.0)));
    assert!(matches!(
        state.effect_hints.as_slice(),
        [WindowEffectHint::ThemeTransition { bounds }] if bounds.height == 96.0
    ));
}

#[test]
fn frame_topology_transition_request_emits_frame_crossfade() {
    let params = window_params();
    let prev = window_info(&params);
    let mut curr = prev.clone();
    curr.window_id = DisplayWindowId::new(42);
    let mut prev_infos = std::collections::HashMap::default();
    prev_infos.insert(prev.window_id, prev);
    let mut curr_infos = std::collections::HashMap::default();
    curr_infos.insert(curr.window_id, curr);
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();

    FrameTopologyTransitionHintRenderRequest::new(&prev_infos, &curr_infos, 180.0, 140.0)
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.transition_hints.len(), 1);
    assert_eq!(state.transition_hints[0].window_id.get(), 0);
    assert_eq!(
        state.transition_hints[0].kind,
        WindowTransitionKind::Crossfade
    );
}

#[test]
fn window_divider_request_splits_wide_vertical_divider() {
    let frame = frame_params();
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();

    WindowDividerRectsRenderRequest::new(
        41,
        118.0,
        20.0,
        6.0,
        80.0,
        WindowDividerOrientation::Vertical,
        &frame,
    )
    .render_and_apply(FrameOutputTarget::from_builder(&mut builder));

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
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();

    WindowScrollBarsRenderRequest::new(&params, &info)
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.scroll_bars.len(), 2);

    let vertical = state
        .scroll_bars
        .iter()
        .find(|bar| !bar.horizontal)
        .expect("vertical scroll bar");
    assert_eq!(vertical.window_id.get(), 41);
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
    assert_eq!(horizontal.window_id.get(), 41);
    assert_eq!(horizontal.x, 10.0);
    assert_eq!(horizontal.y, 102.0);
    assert_eq!(horizontal.width, 120.0);
    assert_eq!(horizontal.height, 8.0);
    assert_eq!(horizontal.position, 4);
}

#[test]
fn window_scroll_bars_request_skips_empty_vertical_track() {
    let mut params = window_params();
    params.bounds.height =
        params.header_line_height + params.tab_line_height + params.mode_line_height;
    params.horizontal_scroll_bar = false;
    let info = window_info(&params);
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();

    WindowScrollBarsRenderRequest::new(&params, &info)
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert!(state.scroll_bars.is_empty());
}

#[test]
#[should_panic(
    expected = "every output window must install complete or skipped presented geometry"
)]
fn frame_output_rejects_window_without_installed_geometry() {
    let params = window_params();
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();
    WindowFrameInfoRenderRequest::new(&params, WindowFrameMetadata::default())
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));
    let _ = builder.finish(80, 24, 8.0, 16.0);
}

#[test]
#[should_panic(expected = "duplicate output window identity")]
fn frame_output_rejects_duplicate_window_identity() {
    let params = window_params();
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();
    for _ in 0..2 {
        WindowFrameInfoRenderRequest::new(&params, WindowFrameMetadata::default())
            .render_and_apply(FrameOutputTarget::from_builder(&mut builder));
    }
}

#[test]
fn missing_vertical_track_does_not_suppress_horizontal_scroll_bar() {
    let params = window_params();
    let mut info = window_info(&params);
    let neomacs_display_protocol::frame_glyphs::PresentedWindowGeometry::Complete {
        regions, ..
    } = &mut info.geometry
    else {
        panic!("complete presented regions");
    };
    regions.right_scroll_bar = None;
    let mut builder = crate::output::builder::DisplayOutputBuilder::new();

    WindowScrollBarsRenderRequest::new(&params, &info)
        .render_and_apply(FrameOutputTarget::from_builder(&mut builder));

    let state = builder.finish(80, 24, 8.0, 16.0);
    assert_eq!(state.scroll_bars.len(), 1);
    assert!(state.scroll_bars[0].horizontal);
}
