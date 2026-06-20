use super::*;
use crate::display_row::DisplayRowFace;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
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
        window_chrome_row_height_for_face(&mut font_metrics, &face, 8.0, 12.0, 20.0),
        20.0
    );
}

#[test]
fn window_chrome_display_text_preserves_lisp_value_for_source_renderer() {
    let _eval = Context::new();

    assert_eq!(
        WindowChromeDisplayText::new(Value::string("tab-line"), true)
            .value()
            .as_utf8_str(),
        Some("tab-line")
    );
    assert_eq!(
        WindowChromeDisplayText::new(Value::string("header"), true)
            .value()
            .as_utf8_str(),
        Some("header")
    );
    assert_eq!(
        WindowChromeDisplayText::new(Value::string("mode"), false)
            .value()
            .as_utf8_str(),
        Some("mode")
    );
}

#[test]
fn chrome_lisp_string_row_request_preserves_policy_inputs() {
    let _eval = Context::new();
    let base_face = ResolvedFace::default();
    let mut symbol_values = std::collections::HashMap::new();
    let align_value = Value::make_int(12);
    symbol_values.insert("align-to".to_string(), align_value);

    let policy = ChromeLispStringRowRequest::new(
        3.0,
        80.0,
        16.0,
        8.0,
        12.0,
        DisplayTabPolicy::every(4),
        DisplayOrigin::ModeLine { selected: true },
        &base_face,
        Value::string("mode"),
    )
    .with_symbol_values(symbol_values)
    .into_source_request_policy();
    let geometry = policy.geometry();

    assert_eq!(policy.role(), GlyphRowRole::ModeLine);
    assert_eq!(geometry.y, 3.0);
    assert_eq!(geometry.width, 80.0);
    assert_eq!(geometry.height, 16.0);
    assert_eq!(geometry.char_width, 8.0);
    assert_eq!(geometry.ascent, 12.0);
    assert_eq!(
        policy.symbol_values().get("align-to").copied(),
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
        output: ChromeRowOutput { row: 3, y: 24.0 },
        bounds: neomacs_display_protocol::types::Rect::new(0.0, 24.0, 96.0, 16.0),
        char_width: 8.0,
        ascent: 12.0,
        tab_policy: DisplayTabPolicy::every(4),
        base_face: &base_face,
        symbol_values,
        text: WindowChromeDisplayText::new(Value::string("mode"), true),
    }
    .into_render_request(render_services.face_ids())
    .render_measured(&mut render_services, None)
    .expect("chrome row should render");

    assert_eq!(render.output, ChromeRowOutput { row: 3, y: 24.0 });
    assert_eq!(
        render.measured.owner,
        DisplayRowOwner::WindowChrome {
            window_id: 42,
            kind: WindowChromeKind::ModeLine,
        }
    );
    assert_eq!(render.measured.row_index, 3);
    assert_eq!(render.measured.bounds.y, 24.0);
    assert_eq!(render.measured.output_progress().y, 24.0);
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
        let row_face = DisplayRowFace::from_resolved(1, &resolved);
        assert_eq!(row_face.box_type, box_type);
        assert_eq!(row_face.render_face().box_type, box_type);
    }
}
