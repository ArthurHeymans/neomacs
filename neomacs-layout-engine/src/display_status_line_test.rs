use super::*;
use crate::display_item::DisplaySourcePosition;
use crate::display_row::DisplayRowFace;
use crate::display_row_builder::DisplayRowGlyphSlot;
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_render_state::{DisplayRowOutputProgress, RenderedDisplayRow};
use neomacs_display_protocol::frame_chrome::ChromeAction;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow};
use neomacs_display_protocol::ui_types::TabBarItem;
use neovm_core::face::FaceTable;

#[test]
fn display_row_height_for_face_uses_realized_line_height_and_box() {
    let mut font_metrics = None;
    let mut face = ResolvedFace::default();
    face.font_family = "monospace".to_string();
    face.font_size = 14.0;
    face.font_ascent = 9.0;
    face.font_line_height = 12.0;
    face.box_type = 1;
    face.box_line_width = 1;

    assert_eq!(
        window_chrome_row_height_for_face(
            &mut font_metrics,
            &face,
            DisplayRowFallbackMetrics::from_default_face_extents(8.0, 20.0, 12.0),
        ),
        20.0
    );
}

#[test]
fn chrome_lisp_string_row_request_preserves_policy_inputs() {
    let _eval = Context::new();
    let base_face = ResolvedFace::default();
    let mut symbol_values = std::collections::HashMap::new();
    let align_value = Value::make_int(12);
    symbol_values.insert("align-to".to_string(), align_value);

    let snapshot = ChromeLispStringRowRequest::new(
        3.0,
        80.0,
        DisplayRowFallbackMetrics::from_default_face_extents(8.0, 16.0, 12.0),
        DisplayTabPolicy::every(4),
        DisplayOrigin::ModeLine { selected: true },
        &base_face,
        Value::string("mode"),
    )
    .with_symbol_values(symbol_values)
    .into_test_snapshot();

    assert_eq!(snapshot.role, GlyphRowRole::ModeLine);
    assert_eq!(snapshot.y, 3.0);
    assert_eq!(snapshot.width, 80.0);
    assert_eq!(snapshot.height, 16.0);
    assert_eq!(snapshot.char_width, 8.0);
    assert_eq!(snapshot.ascent, 12.0);
    assert_eq!(
        snapshot.symbol_values.get("align-to").copied(),
        Some(align_value)
    );
}

#[test]
fn window_chrome_display_row_request_renders_measured_lifecycle_row() {
    let _eval = Context::new();
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver
        .default_base_face_for_origin_without_buffer(&DisplayOrigin::ModeLine { selected: true });
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let mut render_services =
        ChromeRowRenderServices::new(&mut font_metrics, &face_resolver, &mut face_ids);
    let mut symbol_values = std::collections::HashMap::new();
    symbol_values.insert("align-to".to_string(), Value::make_int(12));

    let render = WindowChromeDisplayRowRequest {
        window_id: 42,
        kind: WindowChromeKind::ModeLine,
        selected: true,
        display_row_index: 3,
        output: ChromeRowOutput::new(3, 24.0),
        bounds: neomacs_display_protocol::types::Rect::new(0.0, 24.0, 96.0, 16.0),
        text_area_left_px: 0.0,
        metrics: crate::display_row_metrics::DisplayRowFallbackMetrics::from_default_face_extents(
            8.0, 16.0, 12.0,
        ),
        tab_policy: DisplayTabPolicy::every(4),
        base_face: &base_face,
        symbol_values,
        text: Value::string("mode"),
    }
    .into_render_request(render_services.face_ids())
    .render_measured(&mut render_services, None)
    .expect("chrome row should render");

    assert_eq!(render.output, ChromeRowOutput::new(3, 24.0));
    assert_eq!(
        render.measured.owner(),
        DisplayRowOwner::WindowChrome {
            window_id: 42,
            kind: WindowChromeKind::ModeLine,
        }
    );
    assert_eq!(render.measured.row_index(), 3);
    assert_eq!(render.measured.bounds().y, 24.0);
    assert_eq!(render.measured.output_progress().y(), 24.0);
}

fn proportional_chrome_test_face(
    face_resolver: &FaceResolver,
    origin: &DisplayOrigin,
) -> ResolvedFace {
    let mut face = face_resolver.default_base_face_for_origin_without_buffer(origin);
    face.font_family = "Noto Sans".to_string();
    face.font_size = 9.12871;
    face.font_weight = 400;
    face.set_measured_char_width_px(7.2);
    face.font_ascent = 10.0;
    face.font_line_height = 17.0;
    face
}

fn assert_matches_proportional_dot_width(actual: f32, label: &str) {
    let mut metrics = FontMetricsService::new();
    let expected = metrics.char_width('.', "Noto Sans", 400, false, 9.12871);
    assert!(
        expected > 0.0 && expected < 7.2,
        "test requires Noto Sans dot to be narrower than the fallback cell, got {expected}"
    );
    assert!(
        (actual - expected).abs() < 0.25,
        "{label} should use the GUI font-backed glyph advance for '.', got {actual}, expected {expected}"
    );
}

#[test]
fn window_chrome_gui_tab_and_mode_lines_use_font_backed_glyph_advances() {
    let _eval = Context::new();
    let table = FaceTable::new();
    let face_resolver =
        FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, Some("neo".to_string()));

    for (kind, origin, selected, label) in [
        (
            WindowChromeKind::TabLine,
            DisplayOrigin::TabLine,
            true,
            "tab-line",
        ),
        (
            WindowChromeKind::ModeLine,
            DisplayOrigin::ModeLine { selected: true },
            true,
            "mode-line",
        ),
    ] {
        let base_face = proportional_chrome_test_face(&face_resolver, &origin);
        let mut font_metrics = Some(FontMetricsService::new());
        let mut face_ids = FrameFaceIdAllocator::new(1);
        let mut render_services =
            ChromeRowRenderServices::new(&mut font_metrics, &face_resolver, &mut face_ids);

        let render = WindowChromeDisplayRowRequest {
            window_id: 42,
            kind,
            selected,
            display_row_index: 0,
            output: ChromeRowOutput::new(0, 0.0),
            bounds: neomacs_display_protocol::types::Rect::new(0.0, 0.0, 240.0, 17.0),
            text_area_left_px: 0.0,
            metrics: DisplayRowFallbackMetrics::from_default_face_extents(7.2, 17.0, 10.0),
            tab_policy: DisplayTabPolicy::every(8),
            base_face: &base_face,
            symbol_values: std::collections::HashMap::new(),
            text: Value::string(".agent-sh"),
        }
        .into_render_request(render_services.face_ids())
        .render_measured(&mut render_services, None)
        .expect("window chrome row should render");
        let first_width =
            render.measured.rendered().row().glyphs[GlyphArea::Text.index()][0].pixel_width;

        assert_matches_proportional_dot_width(first_width, label);
    }
}

#[test]
fn frame_tab_bar_gui_uses_font_backed_glyph_advances() {
    let _eval = Context::new();
    let table = FaceTable::new();
    let face_resolver =
        FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, Some("neo".to_string()));
    let base_face = proportional_chrome_test_face(&face_resolver, &DisplayOrigin::TabBar);
    let mut font_metrics = Some(FontMetricsService::new());
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let mut render_services =
        ChromeRowRenderServices::new(&mut font_metrics, &face_resolver, &mut face_ids);

    let measured = FrameTabBarDisplayRowRequest {
        row_index: 0,
        y: 0.0,
        width: 240.0,
        height: 17.0,
        metrics: DisplayRowFallbackMetrics::from_default_face_extents(7.2, 17.0, 10.0),
        base_face: &base_face,
        text: Value::string(".agent-sh"),
    }
    .into_chrome_render_request(render_services.face_ids())
    .render_row(&mut render_services, None)
    .expect("frame tab-bar row should render")
    .measure();
    let first_width = measured.rendered().row().glyphs[GlyphArea::Text.index()][0].pixel_width;

    assert_matches_proportional_dot_width(first_width, "tab-bar");
}

#[test]
fn tab_bar_hit_regions_follow_rendered_caption_bounds() {
    let slots = vec![
        DisplayRowGlyphSlot::new(DisplaySourcePosition::lisp_string(1, 0, 0), 0.0, 0, 4.0, 1),
        DisplayRowGlyphSlot::new(DisplaySourcePosition::lisp_string(1, 1, 1), 4.0, 1, 6.0, 1),
        DisplayRowGlyphSlot::new(DisplaySourcePosition::lisp_string(1, 2, 2), 10.0, 2, 8.0, 1),
        DisplayRowGlyphSlot::new(DisplaySourcePosition::lisp_string(1, 3, 3), 18.0, 3, 5.0, 1),
    ];
    let rendered = RenderedDisplayRow::new(
        GlyphRow::new(GlyphRowRole::TabBar),
        DisplayRowOutputProgress::new(23.0, 4, 0.0, 18.0),
        slots,
        Vec::new(),
        Vec::new(),
    );
    let items = vec![
        TabBarItem {
            index: 3,
            label: "ab".to_string(),
            help: String::new(),
            enabled: true,
            selected: true,
            is_separator: false,
        },
        TabBarItem {
            index: 7,
            label: "cd".to_string(),
            help: String::new(),
            enabled: true,
            selected: false,
            is_separator: false,
        },
    ];

    let regions = tab_bar_hit_regions_for_rendered_captions(&rendered, &items, &[0..2, 2..4], 18.0);

    assert_eq!(regions[0].local_bounds().raw().x, 0.0);
    assert_eq!(regions[0].local_bounds().raw().width, 10.0);
    assert_eq!(regions[0].action(), &ChromeAction::SelectTab { index: 3 });
    assert_eq!(regions[1].local_bounds().raw().x, 10.0);
    assert_eq!(regions[1].local_bounds().raw().width, 13.0);
}

#[test]
fn built_tab_bar_preserves_concatenated_caption_ranges() {
    let mut eval = Context::new();
    eval.setup_thread_locals();
    let source = TabBarDisplaySource {
        captions: vec![Value::string("ab"), Value::string("cde")],
        items: vec![
            TabBarItem {
                index: 0,
                label: "ab".to_string(),
                help: String::new(),
                enabled: true,
                selected: true,
                is_separator: false,
            },
            TabBarItem {
                index: 1,
                label: "cde".to_string(),
                help: String::new(),
                enabled: true,
                selected: false,
                is_separator: false,
            },
        ],
    };

    let built = source.into_built_tab_bar(&mut eval).expect("built tab bar");

    assert_eq!(built.item_char_ranges, vec![0..2, 2..5]);
}

/// A mode-line whose text carries a tall `display` element (here a glyph with
/// `(display (height 2.0))`, the same shape doom-modeline's bar uses) must
/// produce a measured row height taller than the bare font/char height — GNU's
/// `display_mode_line` returns the row's max ascent+descent. The single-layout
/// fix reserves this measured height for the mode line instead of the fixed
/// face height, and reuses the same built row at render time.
#[test]
fn window_chrome_mode_line_row_grows_for_tall_display_element() {
    let _eval = Context::new();
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver
        .default_base_face_for_origin_without_buffer(&DisplayOrigin::ModeLine { selected: true });
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let mut render_services =
        ChromeRowRenderServices::new(&mut font_metrics, &face_resolver, &mut face_ids);

    // Allocated bounds use a 16px char height (the face estimate).
    let allocated_height = 16.0_f32;

    // Plain mode line: measured height stays at the allocated/char height.
    let plain = WindowChromeDisplayRowRequest {
        window_id: 7,
        kind: WindowChromeKind::ModeLine,
        selected: true,
        display_row_index: 1,
        output: ChromeRowOutput::new(1, 0.0),
        bounds: neomacs_display_protocol::types::Rect::new(0.0, 0.0, 240.0, allocated_height),
        text_area_left_px: 0.0,
        metrics: DisplayRowFallbackMetrics::from_default_face_extents(8.0, allocated_height, 12.0),
        tab_policy: DisplayTabPolicy::every(8),
        base_face: &base_face,
        symbol_values: std::collections::HashMap::new(),
        text: Value::string("AB"),
    }
    .into_render_request(render_services.face_ids())
    .render_measured(&mut render_services, None)
    .expect("plain mode-line row should render");
    assert_eq!(
        plain.measured.row_height(),
        allocated_height,
        "plain mode line stays at the face/char height"
    );

    // Mode line with a tall display element on the 'B' glyph.
    let tall_text = Value::string_with_text_properties(
        "AB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![Value::symbol("height"), Value::make_float(2.0)]),
            ]),
        }],
    );
    let tall = WindowChromeDisplayRowRequest {
        window_id: 7,
        kind: WindowChromeKind::ModeLine,
        selected: true,
        display_row_index: 1,
        output: ChromeRowOutput::new(1, 0.0),
        bounds: neomacs_display_protocol::types::Rect::new(0.0, 0.0, 240.0, allocated_height),
        text_area_left_px: 0.0,
        metrics: DisplayRowFallbackMetrics::from_default_face_extents(8.0, allocated_height, 12.0),
        tab_policy: DisplayTabPolicy::every(8),
        base_face: &base_face,
        symbol_values: std::collections::HashMap::new(),
        text: tall_text,
    }
    .into_render_request(render_services.face_ids())
    .render_measured(&mut render_services, None)
    .expect("tall mode-line row should render");

    assert!(
        tall.measured.row_height() > allocated_height,
        "tall display element must grow the mode-line row beyond the face/char height \
         (got {} <= {})",
        tall.measured.row_height(),
        allocated_height
    );
}

#[test]
fn tab_bar_display_source_extracts_menu_items_until_nested_keymap() {
    let _eval = Context::new();
    let keymap = Value::list(vec![
        KeymapMarker::Keymap.symbol_value(),
        Value::list(vec![
            Value::symbol("current-tab"),
            KeymapMarker::MenuItem.symbol_value(),
            Value::string("One"),
        ]),
        Value::cons(
            Value::symbol("next-tab"),
            Value::list(vec![
                KeymapMarker::MenuItem.symbol_value(),
                Value::string("Two"),
            ]),
        ),
        Value::list(vec![KeymapMarker::Keymap.symbol_value()]),
        Value::list(vec![
            Value::symbol("after-nested-map"),
            KeymapMarker::MenuItem.symbol_value(),
            Value::string("After"),
        ]),
    ]);

    let source = TabBarDisplaySource::from_keymap(keymap).expect("tab-bar source");

    assert_eq!(
        source
            .captions
            .iter()
            .map(|caption| caption.as_runtime_string_owned().unwrap())
            .collect::<Vec<_>>(),
        vec!["One".to_string(), "Two".to_string()]
    );
    assert_eq!(source.items.len(), 2);
    assert_eq!(source.items[0].index, 0);
    assert_eq!(source.items[0].label, "One");
    assert_eq!(source.items[1].index, 1);
    assert_eq!(source.items[1].label, "Two");
}

#[test]
fn tab_bar_display_source_builds_concat_text_and_preserves_items() {
    let mut eval = Context::new();
    let source = TabBarDisplaySource {
        captions: vec![Value::string("One"), Value::string("Two")],
        items: vec![
            TabBarItem {
                index: 0,
                label: "One".to_string(),
                help: String::new(),
                enabled: true,
                selected: false,
                is_separator: false,
            },
            TabBarItem {
                index: 1,
                label: "Two".to_string(),
                help: String::new(),
                enabled: true,
                selected: false,
                is_separator: false,
            },
        ],
    };

    let built = source.into_built_tab_bar(&mut eval).expect("built tab bar");

    assert_eq!(
        built.text.as_runtime_string_owned().as_deref(),
        Some("OneTwo")
    );
    assert_eq!(built.items.len(), 2);
    assert_eq!(built.items[0].label, "One");
    assert_eq!(built.items[1].label, "Two");
}

#[test]
fn window_chrome_target_cols_reserves_right_border_column() {
    assert_eq!(
        WindowChromeTargetColumns::new(80.0, 8.0, false).columns(),
        10
    );
    assert_eq!(WindowChromeTargetColumns::new(80.0, 8.0, true).columns(), 9);
    assert_eq!(WindowChromeTargetColumns::new(3.0, 8.0, true).columns(), 1);
    assert_eq!(
        WindowChromeTargetColumns::new(80.0, 0.0, false).columns(),
        80
    );
}

#[test]
fn display_row_face_preserves_gnu_box_type_codes() {
    let mut resolved = ResolvedFace::default();
    let boxes = [
        (0, BoxType::None),
        (1, BoxType::Line),
        (2, BoxType::Raised3D),
        (3, BoxType::Sunken3D),
    ];

    for (code, box_type) in boxes {
        resolved.box_type = code;
        let row_face = DisplayRowFace::from_resolved(FaceId::new(1), &resolved);
        assert_eq!(row_face.box_type, box_type);
        assert_eq!(row_face.render_face().box_type, box_type);
    }
}
