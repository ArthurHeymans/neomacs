/// Batch 480: info, man, woman, helpful, widget, wid-edit deep, customize deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx480_info_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'info)
  (list (fboundp 'info) (boundp 'Info-mode-map)))
"##,
    );
}

#[test]
fn div_cx480_man_background() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'man)
  (list (fboundp 'man) (boundp 'Man-mode-map)))
"##,
    );
}

#[test]
fn div_cx480_widget_create_editable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (with-temp-buffer
    (let ((w (widget-create 'editable-field "default")))
      (widget-value w))))
"##,
    );
}

#[test]
fn div_cx480_widget_value_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (with-temp-buffer
    (let ((w (widget-create 'editable-field "initial")))
      (widget-value-set w "updated")
      (widget-value w))))
"##,
    );
}

#[test]
fn div_cx480_customize_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cus-edit)
  (list (fboundp 'customize) (boundp 'custom-buffer-style)))
"##,
    );
}

#[test]
fn div_cx480_customize_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cus-edit)
  (list (fboundp 'customize-option) (fboundp 'customize-save-customized)))
"##,
    );
}

#[test]
fn div_cx480_widget_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (widget-match-inline (widget-create 'editable-field "hello") '(hello)))
"##,
    );
}

#[test]
fn div_cx480_widget_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (widget-documentation "test"))
"##,
    );
}

#[test]
fn div_cx480_custom_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'custom-theme)
  (list (boundp 'custom-theme-load-path) (fboundp 'custom-theme-set-faces)))
"##,
    );
}

#[test]
fn div_cx480_custom_save() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cus-edit)
  (fboundp 'custom-save-all))
"##,
    );
}

#[test]
fn div_cx480_widget_color() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (widget-type (widget-create 'color "red")))
"##,
    );
}

#[test]
fn div_cx480_widget_checkbox() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (widget-value (widget-create 'checkbox nil)))
"##,
    );
}

#[test]
fn div_cx480_widget_radio() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (let ((w (widget-create 'radio-button-choice
            '(item "A") '(item "B") '(item "C"))))
    (widget-value w)))
"##,
    );
}

#[test]
fn div_cx480_widget_menu_choice() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (let ((w (widget-create 'menu-choice '(item "A") '(item "B"))))
    (widget-value w)))
"##,
    );
}

#[test]
fn div_cx480_widget_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (widget-create 'link :button-prefix "" :button-suffix "" "click me"))
"##,
    );
}
