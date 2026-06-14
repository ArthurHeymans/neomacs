//! widget / custom coverage (currently faithful).
//!
//! Probes defcustom/custom-variable-p/custom-type/standard-value,
//! widget-create (editable-field, item) + widget-get/value/apply/delete,
//! define-widget + widget-put/get on a type, built-in widget-type predicates,
//! and custom-facep. All run under --batch (widget-create works headless here).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

fn _u() {}

#[test]
fn div_wc_defcustom_custom_variable_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    _u();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn (defcustom neo-wc-dc 5 "doc")
           (list (custom-variable-p 'neo-wc-dc)
                 (get 'neo-wc-dc 'custom-type)
                 (consp (get 'neo-wc-dc 'standard-value))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_wc_widget_create_editable_field() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((w (widget-create 'editable-field :value "hi")))
        (list (widgetp w) (widget-value w))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_wc_widget_get_put_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((w (widget-create 'item :value "xval" :tag "T")))
        (widget-put w :neo-prop 99)
        (list (widget-get w :tag) (widget-get w :neo-prop) (widget-value w))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_wc_widget_apply_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((w (widget-create 'item :value "y")))
        (widget-apply w :delete)
        :done))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_wc_define_widget_type_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (define-widget 'neo-wc-item 'item "custom widget")
      (widget-put 'neo-wc-item :neo-prop 99)
      (list (widget-get 'neo-wc-item :neo-prop)
            (widget-get 'neo-wc-item :tag)))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_wc_widget_type_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (widgetp 'editable-field) (widgetp 'item) (widgetp 'button)
      (widgetp 'menu-choice) (widgetp 'checkbox) (widgetp 'toggle))
"##,
    );
}

#[test]
fn div_wc_custom_facep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (custom-facep 'default) (custom-facep 'bold)
      (custom-facep 'nonexistent-face) (facep 'default))
"##,
    );
}

#[test]
fn div_wc_widget_child_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((w (widget-create 'editable-field :value "abc")))
        (list (length (widget-value w))
              (widget-apply w :value))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_wc_defface_custom_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (defface neo-wc-face '((t :foreground "red")) "doc")
  (list (get 'neo-wc-face 'face-defface-spec)
        (facep 'neo-wc-face)
        (face-attribute 'neo-wc-face :foreground)))
"##,
    );
}
