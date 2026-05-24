use super::*;
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::keymap::{
    list_keymap_define, list_keymap_set_parent, make_sparse_list_keymap,
};
use neovm_core::heap_types::LispString;

#[test]
fn extract_menu_label_preserves_raw_unibyte_strings() {
    let mut eval = Context::new();
    eval.setup_thread_locals();
    let raw = Value::heap_string(LispString::from_unibyte(vec![0xFF]));
    let expected = raw
        .as_runtime_string_owned()
        .expect("runtime string for raw label");

    let plain = Value::cons(raw, Value::symbol("ignore"));
    assert_eq!(extract_menu_label(&plain), Some(expected.clone()));

    let menu_item = Value::list(vec![
        Value::symbol("menu-item"),
        raw,
        Value::symbol("ignore"),
    ]);
    assert_eq!(extract_menu_label(&menu_item), Some(expected));
}

#[test]
fn collect_from_keymap_includes_inherited_menu_bar_items() {
    let mut eval = Context::new();
    eval.setup_thread_locals();

    let parent = make_sparse_list_keymap();
    let child = make_sparse_list_keymap();
    let parent_menu = make_sparse_list_keymap();
    let child_menu = make_sparse_list_keymap();

    list_keymap_define(
        parent_menu,
        Value::symbol("text"),
        Value::cons(Value::string("Text"), Value::symbol("ignore")),
    );
    list_keymap_define(
        child_menu,
        Value::symbol("org"),
        Value::cons(Value::string("Org"), Value::symbol("ignore")),
    );
    list_keymap_set_parent(child_menu, parent_menu);

    list_keymap_define(parent, Value::symbol("menu-bar"), parent_menu);
    list_keymap_define(child, Value::symbol("menu-bar"), child_menu);
    list_keymap_set_parent(child, parent);

    let mut items = Vec::new();
    collect_from_keymap(&eval, &child, &mut items);

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].key, "org");
    assert_eq!(items[0].label, "Org");
    assert_eq!(items[1].key, "text");
    assert_eq!(items[1].label, "Text");
}

#[test]
fn collect_from_keymap_hides_inherited_undefined_menu_items() {
    let mut eval = Context::new();
    eval.setup_thread_locals();

    let keymap = make_sparse_list_keymap();
    let parent_menu = make_sparse_list_keymap();
    let child_menu = make_sparse_list_keymap();

    for (key, label) in [
        ("headings", "Headings"),
        ("show", "Show"),
        ("hide", "Hide"),
        ("text", "Text"),
    ] {
        list_keymap_define(
            parent_menu,
            Value::symbol(key),
            Value::cons(Value::string(label), Value::symbol("ignore")),
        );
    }

    list_keymap_define(
        child_menu,
        Value::symbol("org"),
        Value::cons(Value::string("Org"), Value::symbol("ignore")),
    );
    for key in ["headings", "show", "hide"] {
        list_keymap_define(child_menu, Value::symbol(key), Value::symbol("undefined"));
    }
    list_keymap_set_parent(child_menu, parent_menu);

    list_keymap_define(keymap, Value::symbol("menu-bar"), child_menu);

    let mut items = Vec::new();
    collect_from_keymap(&eval, &keymap, &mut items);
    let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();

    assert_eq!(labels, vec!["Org", "Text"]);
}

#[test]
fn collect_from_keymap_descends_embedded_menu_bar_keymaps() {
    let mut eval = Context::new();
    eval.setup_thread_locals();

    let keymap = make_sparse_list_keymap();
    let org_group = make_sparse_list_keymap();
    let text_group = make_sparse_list_keymap();

    list_keymap_define(
        org_group,
        Value::symbol("org"),
        Value::list(vec![
            Value::symbol("menu-item"),
            Value::string("Org"),
            make_sparse_list_keymap(),
        ]),
    );
    list_keymap_define(
        org_group,
        Value::symbol("table"),
        Value::list(vec![
            Value::symbol("menu-item"),
            Value::string("Table"),
            make_sparse_list_keymap(),
        ]),
    );
    list_keymap_define(
        text_group,
        Value::symbol("text"),
        Value::list(vec![
            Value::symbol("menu-item"),
            Value::string("Text"),
            make_sparse_list_keymap(),
        ]),
    );

    let menu_bar = Value::list(vec![Value::symbol("keymap"), org_group, text_group]);
    list_keymap_define(keymap, Value::symbol("menu-bar"), menu_bar);

    let mut items = Vec::new();
    collect_from_keymap(&eval, &keymap, &mut items);
    let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();

    assert_eq!(labels, vec!["Table", "Org", "Text"]);
}

#[test]
fn collect_tty_menu_bar_items_uses_selected_window_local_map() {
    let mut eval = Context::new();
    eval.setup_thread_locals();

    let selected_buffer = eval.buffer_manager_mut().create_buffer("selected");
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("menu-local-map", 800, 600, selected_buffer);
    let local_map = make_sparse_list_keymap();
    let local_menu = make_sparse_list_keymap();
    list_keymap_define(
        local_menu,
        Value::symbol("mode-menu"),
        Value::cons(Value::string("Mode Menu"), Value::symbol("ignore")),
    );
    list_keymap_define(local_map, Value::symbol("menu-bar"), local_menu);
    eval.buffer_manager_mut()
        .set_buffer_local_map(selected_buffer, local_map)
        .expect("set selected buffer local map");

    let labels: Vec<_> = collect_tty_menu_bar_items_for_frame(&eval, frame_id)
        .into_iter()
        .map(|item| item.label)
        .collect();
    assert!(
        labels.iter().any(|label| label == "Mode Menu"),
        "{labels:?}"
    );
}
