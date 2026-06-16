//! Complex combo batch 201 — `widget` deep: editable-field, checkbox,
//! radio-button-choice, menu-choice, tree-widget, item with validation.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx201_widget_create_editable_field_with_validation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((w (widget-create 'editable-field
                               :value "initial"
                               :size 30
                               :format "Prompt: %v"
                               :valid-regexp "^[a-z]+$"
                               :help-echo "Enter lowercase")))
        (list (widgetp w)
              (widget-value w)
              (widget-get w :size)
              (widget-get w :valid-regexp))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_checkbox_toggle_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((chk (widget-create 'checkbox)))
        (let ((v1 (widget-value chk)))
          (widget-apply chk :toggle)
          (let ((v2 (widget-value chk)))
            (widget-apply chk :toggle)
            (let ((v3 (widget-value chk)))
              (list (widgetp chk) v1 v2 v3))))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_radio_button_choice() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((rb (widget-create 'radio-button-choice
                                :value :b
                                :help-echo "Choose one"
                                '(:a) '(:b) '(:c))))
        (list (widgetp rb)
              (widget-value rb)
              (widget-apply rb :complete))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_menu_choice_with_items() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((mc (widget-create 'menu-choice
                                :value :b
                                :help-echo "Select"
                                '(item :a) '(item :b) '(item :c))))
        (list (widgetp mc)
              (widget-value mc))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_default_format_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (boundp 'widget-push-button)
          (boundp 'widget-editable-list)
          (boundp 'widget-image)
          (boundp 'widget-menu-max-short))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_field_constraints_and_move() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((w1 (widget-create 'editable-field :value "field1"))
            (w2 (widget-create 'editable-field :value "field2")))
        (widget-forward 1)
        (let ((at-w2 (eq (widget-at) w2)))
          (widget-backward 1)
          (let ((at-w1 (eq (widget-at) w1)))
            (list at-w1 at-w2)))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_item_with_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((it (widget-create 'item :value "Static label text")))
        (list (widgetp it)
              (widget-value it))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_text_create_with_multiline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((txt (widget-create 'text
                                 :value "line1\nline2\nline3")))
        (list (widgetp txt)
              (widget-value txt)
              (length (split-string (widget-value txt) "\n")))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_apply_set_get_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((w (widget-create 'editable-field :value "before")))
        (widget-value-set w "after")
        (list (widget-value w)
              (widget-apply w :complete))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx201_widget_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'widget)
      (with-temp-buffer
        (buffer-enable-undo)
        (insert "Widget mega test buffer content")
        (put-text-property 1 6 'face 'bold)
        (let ((m (set-marker (make-marker) 8))
              (ov (make-overlay 4 14)))
          (overlay-put ov 'face 'italic)
          (overlay-put ov 'evaporate t)
          (narrow-to-region 2 18)
          (let ((w (widget-create 'editable-field :value "test")))
            (let ((state (list (widgetp w)
                               (widget-value w)
                               (buffer-string)
                               (marker-position m)
                               (overlay-start ov) (overlay-end ov)
                               (text-properties-at 1))))
              (undo)
              (widen)
              (list state (buffer-string) (marker-position m)
                    (overlay-start ov) (overlay-end ov)
                    (text-properties-at 1)))))))
  (error (list :errored (car e))))
"##,
    );
}
