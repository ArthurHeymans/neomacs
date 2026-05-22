//! Divergence tests: image, widget, tool-bar, menu stubs.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_image_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'create-image)
  (fboundp 'find-image)
  (fboundp 'insert-image)
  (fboundp 'image-type-available-p)
  (fboundp 'image-size))"#,
    );
}

#[test]
fn divergence_image_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (consp image-types)
  (listp image-types)
  (member 'png image-types)
  (member 'svg image-types))"#,
    );
}

#[test]
fn divergence_image_cache() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (integerp image-cache-eviction-delay)
  (fboundp 'clear-image-cache))"#,
    );
}

#[test]
fn divergence_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'tool-bar-add-item)
  (fboundp 'tool-bar-local-item)
  (fboundp 'tool-bar-mode)
  (boundp 'tool-bar-map))"#,
    );
}

#[test]
fn divergence_menu_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'menu-bar-mode)
  (boundp 'menu-bar-final-items)
  (listp menu-bar-final-items))"#,
    );
}

#[test]
fn divergence_widget_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'widget-create)
  (fboundp 'widget-delete)
  (fboundp 'widget-value)
  (fboundp 'widget-type)
  (featurep 'wid-edit))"#,
    );
}

#[test]
fn divergence_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'tab-bar-mode)
  (fboundp 'tab-bar-history-mode)
  (boundp 'tab-bar-tabs-function)
  (>= emacs-major-version 27))"#,
    );
}

#[test]
fn divergence_display_pixels() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'display-pixel-width)
  (fboundp 'display-pixel-height)
  (fboundp 'display-mm-width)
  (fboundp 'display-mm-height)
  (fboundp 'x-display-pixels))"#,
    );
}

#[test]
fn divergence_color_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'color-values)
  (fboundp 'color-defined-p)
  (fboundp 'defined-colors)
  (color-defined-p "red")
  (consp (color-values "red")))#" ,
    );
}

#[test]
fn divergence_face_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'set-face-attribute)
  (fboundp 'face-attribute)
  (fboundp 'face-spec-match-p)
  (fboundp 'face-spec-set))"#,
    );
}
