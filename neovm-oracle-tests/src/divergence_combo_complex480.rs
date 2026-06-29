/// Batch 480: info, man, woman, helpful, widget, wid-edit deep, customize deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx480_info_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'info)
  (list (fboundp 'info) (boundp 'Info-mode-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx480_man_background() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'man)
  (list (fboundp 'man) (boundp 'Man-mode-map)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx480_widget_create_editable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (with-temp-buffer
    (let ((w (widget-create 'editable-field "default")))
      (widget-value w))))
"##,
        expect_test::expect![[r#""OK \"default\"""#]],
    );
}

#[test]
fn div_cx480_widget_value_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (with-temp-buffer
    (let ((w (widget-create 'editable-field "initial")))
      (widget-value-set w "updated")
      (widget-value w))))
"##,
        expect_test::expect![[r#""OK \"updated\"""#]],
    );
}

#[test]
fn div_cx480_customize_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'cus-edit)
  (list (fboundp 'customize) (boundp 'custom-buffer-style)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx480_customize_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'cus-edit)
  (list (fboundp 'customize-option) (fboundp 'customize-save-customized)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx480_widget_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (widget-match-inline (widget-create 'editable-field "hello") '(hello)))
"##,
        expect_test::expect![[r#""hello\nOK nil""#]],
    );
}

#[test]
fn div_cx480_widget_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (widget-documentation "test"))
"##,
        expect_test::expect![[r#""ERR (void-function widget-documentation)""#]],
    );
}

#[test]
fn div_cx480_custom_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'custom-theme)
  (list (boundp 'custom-theme-load-path) (fboundp 'custom-theme-set-faces)))
"##,
        expect_test::expect![[
            r#""ERR (file-missing \"Cannot open load file\" \"No such file or directory\" \"custom-theme\")""#
        ]],
    );
}

#[test]
fn div_cx480_custom_save() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'cus-edit)
  (fboundp 'custom-save-all))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx480_widget_color() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (widget-type (widget-create 'color "red")))
"##,
        expect_test::expect![[r#""Color: red            [ Choose ]  (sample)\nOK color""#]],
    );
}

#[test]
fn div_cx480_widget_checkbox() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (widget-value (widget-create 'checkbox nil)))
"##,
        expect_test::expect![[r#""[ ]OK nil""#]],
    );
}

#[test]
fn div_cx480_widget_radio() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (let ((w (widget-create 'radio-button-choice
            '(item "A") '(item "B") '(item "C"))))
    (widget-value w)))
"##,
        expect_test::expect![[r#""( ) A\n( ) B\n( ) C\nOK nil""#]],
    );
}

#[test]
fn div_cx480_widget_menu_choice() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (let ((w (widget-create 'menu-choice '(item "A") '(item "B"))))
    (widget-value w)))
"##,
        expect_test::expect![[r#""choice: invalid (nil)\nOK nil""#]],
    );
}

#[test]
fn div_cx480_widget_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'wid-edit)
  (widget-create 'link :button-prefix "" :button-suffix "" "click me"))
"##,
        expect_test::expect![[
            r#""click meOK (link :args nil :value \"click me\" :button-prefix \"\" :button-suffix \"\" :button-overlay #<overlay from 1 to 9 in  *neovm-oracle-stdout*> :from #<marker (moves after insertion) at 1 in  *neovm-oracle-stdout*> :to #<marker at 9 in  *neovm-oracle-stdout*>)""#
        ]],
    );
}
