use super::*;
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
fn toolbar_theme_defaults_to_jetbrains_like() {
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
            .is_some_and(|path| path.ends_with("etc/toolbar-icons/jetbrains-like/search.svg")),
        "default image path: {themed:#?}"
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
