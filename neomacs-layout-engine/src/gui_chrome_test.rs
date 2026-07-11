use super::*;
use neomacs_display_protocol::frame_chrome::ChromeAction;
use neomacs_display_protocol::types::Color;
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::load::{
    apply_runtime_startup_state, create_bootstrap_evaluator_cached_with_features,
};
use neovm_core::heap_types::LispString;

#[test]
fn parse_tool_bar_item_preserves_raw_unibyte_label_and_help() {
    let mut eval = Context::new();
    eval.setup_thread_locals();
    let raw = Value::heap_string(LispString::from_unibyte(vec![0xFF]));
    let expected = raw
        .as_runtime_string_owned()
        .expect("runtime string for raw label");
    let def = Value::list(vec![
        Value::symbol("menu-item"),
        raw,
        Value::symbol("ignore"),
        Value::symbol(":help"),
        raw,
    ]);

    let item = parse_tool_bar_item(&mut eval, "raw-item", &def, 0).expect("tool-bar item");
    assert_eq!(item.label, expected);
    assert_eq!(item.help, expected);
}

#[test]
fn parse_tool_bar_item_keeps_wrap_separate_from_button_type() {
    let mut eval = Context::new();
    eval.setup_thread_locals();
    let def = Value::list(vec![
        Value::symbol("menu-item"),
        Value::string("Wrapped"),
        Value::symbol("ignore"),
        Value::symbol(":button"),
        Value::cons(Value::symbol(":toggle"), Value::T),
        Value::symbol(":wrap"),
        Value::T,
        Value::symbol(":enable"),
        Value::T,
    ]);

    let item = parse_tool_bar_item(&mut eval, "wrapped-toggle", &def, 0).expect("tool-bar item");
    assert_eq!(item.item_type, ToolBarItemType::Toggle);
    assert_eq!(item.item_type.gnu_type_name(), ":toggle");
    assert!(item.selected);
    assert!(item.wrap);
    assert!(!item.enabled);
}

#[test]
fn toolbar_image_extensions_use_typed_gnu_image_domain() {
    assert!(is_supported_toolbar_image_file("open.xpm"));
    assert!(is_supported_toolbar_image_file("photo.JPG"));
    assert!(is_supported_toolbar_image_file("diagram.svgz"));
    assert!(!is_supported_toolbar_image_file("unknown.bmp"));
    assert!(toolbar_image_score("open.xpm") < toolbar_image_score("photo.jpg"));
    assert!(toolbar_image_score("open.pbm") < toolbar_image_score("open.svg"));
}

#[test]
fn toolbar_icon_name_keeps_gnu_image_base_name() {
    assert_eq!(
        tool_bar_icon_name_from_path("search.xpm").as_deref(),
        Some("search")
    );
    assert_eq!(
        tool_bar_icon_name_from_path("low-color/search.xpm").as_deref(),
        Some("search")
    );
    assert_eq!(
        tool_bar_icon_name_from_path("/tmp/neomacs/etc/images/mail/compose.xpm").as_deref(),
        Some("mail/compose")
    );
}

#[test]
fn toolbar_theme_resolves_themed_svg_and_preserves_gnu_fallback() {
    let mut eval = Context::new();
    eval.setup_thread_locals();
    let spec = Value::list(vec![
        Value::symbol("image"),
        Value::symbol(":type"),
        Value::symbol("xpm"),
        Value::symbol(":file"),
        Value::string("search.xpm"),
    ]);

    eval.eval_str("(setq neomacs-toolbar-icon-theme 'material)")
        .expect("set toolbar icon theme");
    let themed = tool_bar_image_source(&eval, &spec).expect("themed image source");
    assert!(
        themed
            .file_path()
            .is_some_and(|path| path.ends_with("etc/toolbar-icons/material/search.svg")),
        "material image path: {themed:#?}"
    );

    eval.eval_str("(setq neomacs-toolbar-icon-theme 'gnu)")
        .expect("set GNU toolbar icon theme");
    let gnu = tool_bar_image_source(&eval, &spec).expect("GNU image source");
    assert!(
        gnu.file_path()
            .is_some_and(|path| path.ends_with("search.xpm") && !path.contains("toolbar-icons")),
        "GNU image path: {gnu:#?}"
    );
}

#[test]
fn toolbar_theme_defaults_to_vscode_like() {
    let mut eval = Context::new();
    eval.setup_thread_locals();
    let spec = Value::list(vec![
        Value::symbol("image"),
        Value::symbol(":type"),
        Value::symbol("xpm"),
        Value::symbol(":file"),
        Value::string("search.xpm"),
    ]);

    let themed = tool_bar_image_source(&eval, &spec).expect("default themed image source");
    assert!(
        themed
            .file_path()
            .is_some_and(|path| path.ends_with("etc/toolbar-icons/vscode-like/search.svg")),
        "default image path: {themed:#?}"
    );
}

#[test]
fn toolbar_theme_resolves_gnu_find_image_expression_to_default_theme() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["neomacs"]).expect("bootstrap evaluator");
    let expression = eval
        .eval_str(
            r#"
            '(find-image
              '((:type xpm :file "search.xpm")
                (:type pbm :file "search.pbm")
                (:type xbm :file "search.xbm")))
            "#,
        )
        .expect("GNU find-image expression");

    let themed = tool_bar_image_source(&eval, &expression).expect("default themed image source");
    assert!(
        themed
            .file_path()
            .is_some_and(|path| path.ends_with("etc/toolbar-icons/vscode-like/search.svg")),
        "default image path from GNU find-image expression: {themed:#?}"
    );
}

#[test]
fn collect_gui_menu_bar_items_runtime_frame_has_help_menu() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["neomacs"]).expect("bootstrap evaluator");
    apply_runtime_startup_state(&mut eval).expect("runtime startup state");
    let items = collect_gui_menu_bar_items(&eval);
    assert!(!items.is_empty());
    assert!(items.iter().any(|item| item.key == "help-menu"));
}

#[test]
fn collect_gui_tool_bar_items_after_setup_has_search_item_and_separator() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["neomacs"]).expect("bootstrap evaluator");
    eval.eval_str("(tool-bar-setup)")
        .expect("run GNU tool-bar setup");
    eval.eval_str("(setq neomacs-toolbar-icon-theme 'gnu)")
        .expect("set GNU toolbar icon theme");
    let items = collect_gui_tool_bar_items(&mut eval);
    assert!(
        items.iter().any(|item| item
            .image
            .as_ref()
            .and_then(|image| image.file_path())
            .is_some_and(|path| path.ends_with("/search.xpm") || path == "search.xpm")),
        "tool-bar items: {items:#?}"
    );
    assert!(items.iter().any(|item| item.is_separator()));
}

#[test]
fn collect_gui_tool_bar_items_after_setup_uses_default_theme() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["neomacs"]).expect("bootstrap evaluator");
    eval.eval_str("(tool-bar-setup)")
        .expect("run GNU tool-bar setup");

    let items = collect_gui_tool_bar_items(&mut eval);
    assert!(
        items.iter().any(|item| item
            .image
            .as_ref()
            .and_then(|image| image.file_path())
            .is_some_and(|path| path.ends_with("etc/toolbar-icons/vscode-like/search.svg"))),
        "tool-bar items: {items:#?}"
    );
}

#[test]
fn layout_gui_menu_bar_content_assigns_local_bounds_and_actions() {
    let content = layout_gui_menu_bar_content(
        vec![
            MenuBarItem {
                index: 0,
                label: "File".to_string(),
                key: "file".to_string(),
            },
            MenuBarItem {
                index: 1,
                label: "Edit".to_string(),
                key: "edit".to_string(),
            },
        ],
        200.0,
        18.0,
        8.0,
        Color::WHITE,
        Color::BLACK,
    );

    assert_eq!(
        content.items()[0].local_bounds().raw(),
        neomacs_display_protocol::types::Rect::new(8.0, 0.0, 48.0, 18.0)
    );
    assert_eq!(
        content.items()[0].action(),
        Some(&ChromeAction::OpenMenu {
            index: 0,
            key: "file".to_string(),
        })
    );
    assert_eq!(content.items()[1].local_bounds().raw().x, 56.0);
}

#[test]
fn layout_gui_tool_bar_content_uses_one_height_policy() {
    let items = vec![
        ToolBarItem {
            index: 0,
            key: "save".to_string(),
            image: None,
            label: "Save".to_string(),
            help: String::new(),
            enabled: true,
            selected: false,
            item_type: ToolBarItemType::Button,
            wrap: false,
        },
        ToolBarItem {
            index: 1,
            key: "separator".to_string(),
            image: None,
            label: String::new(),
            help: String::new(),
            enabled: false,
            selected: false,
            item_type: ToolBarItemType::Separator,
            wrap: false,
        },
        ToolBarItem {
            index: 2,
            key: "disabled".to_string(),
            image: None,
            label: "Disabled".to_string(),
            help: String::new(),
            enabled: false,
            selected: false,
            item_type: ToolBarItemType::Button,
            wrap: false,
        },
    ];
    let content = layout_gui_tool_bar_content(items, 200.0, 34.0, Color::WHITE, Color::BLACK);

    assert_eq!(content.icon_size(), 24);
    assert_eq!(content.padding(), 5);
    assert_eq!(
        content.items()[0].local_bounds().raw(),
        neomacs_display_protocol::types::Rect::new(5.0, 0.0, 34.0, 34.0)
    );
    assert_eq!(
        content.items()[0].action(),
        Some(&ChromeAction::InvokeToolBarItem { index: 0 })
    );
    assert_eq!(content.items()[1].action(), None);
    assert_eq!(content.items()[2].action(), None);
}

#[test]
fn layout_gui_compact_bar_content_places_tools_after_menu_items() {
    let menu_items = vec![MenuBarItem {
        index: 0,
        label: "File".to_string(),
        key: "file".to_string(),
    }];
    let tool_items = vec![ToolBarItem {
        index: 0,
        key: "save".to_string(),
        image: None,
        label: "Save".to_string(),
        help: String::new(),
        enabled: true,
        selected: false,
        item_type: ToolBarItemType::Button,
        wrap: false,
    }];
    let content = layout_gui_compact_bar_content(
        menu_items,
        tool_items,
        240.0,
        34.0,
        8.0,
        Color::WHITE,
        Color::BLACK,
        Color::WHITE,
        Color::BLACK,
    );

    let menu_right = {
        let bounds = content.menu_items()[0].local_bounds().raw();
        bounds.x + bounds.width
    };
    assert!(content.tool_items()[0].local_bounds().raw().x > menu_right);
}
