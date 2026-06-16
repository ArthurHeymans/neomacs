//! Complex combo batch 141 — `widget` / `custom` / `customize` /
//! `custom-widgets` buffer-local persistence and validation.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx141_widget_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'widget)
      (list (fboundp 'widget-create)
            (fboundp 'widget-insert)
            (boundp 'widget-push-button)
            (boundp 'widget-editable-list)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_custom_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'cus-edit)
      (list (fboundp 'customize)
            (fboundp 'customize-option)
            (boundp 'custom-file)
            (boundp 'custom-buffer-verbose-help)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_defcustom_with_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (defcustom neo-cx141-var :default
        "docstring"
        :type 'symbol
        :group 'neo-cx141)
      (list (custom-variable-p 'neo-cx141-var)
            (default-value 'neo-cx141-var)
            (custom-variable-type 'neo-cx141-var)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_custom_variable_persistence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((before (default-value 'neo-cx141-persist)))
      (setq-default neo-cx141-persist :changed)
      (let ((after-set (default-value 'neo-cx141-persist)))
        (setq-default neo-cx141-persist before)
        (list before after-set)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_custom_theme_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'cus-theme)
      (list (fboundp 'customize-create-theme)
            (boundp 'custom-theme-directory)
            (boundp 'custom-safe-themes)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_widget_button_creation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((btn (widget-create 'push-button
                                 :notify (lambda (&rest _) (message "clicked"))
                                 "Click Me")))
        (list (widgetp btn)
              (widget-get btn :notify)
              (widget-get btn :format))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_widget_editable_field() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((field (widget-create 'editable-field
                                   :value "initial text"
                                   :size 30)))
        (list (widgetp field)
              (widget-value field))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_widget_checkbox_creation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((chk (widget-create 'checkbox)))
        (list (widgetp chk)
              (widget-value chk)
              (widget-apply chk :toggle)
              (widget-value chk))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_widget_choice_creation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((ch (widget-create 'menu-choice
                                :value :a
                                :help-echo "choose"
                                '(item :a) '(item :b) '(item :c))))
        (list (widgetp ch)
              (widget-value ch))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_custom_group_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (defgroup neo-cx141-group nil
        "Test custom group"
        :group 'neo-cx141-parent)
      (list (custom-group-p 'neo-cx141-group)
            (get 'neo-cx141-group 'custom-group)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_custom_save_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'custom-save-all)
          (fboundp 'custom-save-customized)
          (boundp 'custom-saved))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx141_widget_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'widget)
      (with-temp-buffer
        (buffer-enable-undo)
        (insert "Widget mega test buffer content")
        (put-text-property 1 7 'face 'bold)
        (let ((m (set-marker (make-marker) 8))
              (ov (make-overlay 4 14)))
          (overlay-put ov 'face 'italic)
          (overlay-put ov 'evaporate t)
          (narrow-to-region 2 18)
          (let ((state (list (fboundp 'widget-create)
                             (buffer-string)
                             (marker-position m)
                             (overlay-start ov) (overlay-end ov)
                             (text-properties-at 1))))
            (undo)
            (widen)
            (list state (buffer-string) (marker-position m)
                  (overlay-start ov) (overlay-end ov)
                  (text-properties-at 1))))))
  (error (list :errored (car e))))
"##,
    );
}
