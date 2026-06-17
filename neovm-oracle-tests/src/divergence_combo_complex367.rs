//! Complex combo batch 367 — `widget`/`button`/`browse-url` ultimate:
//! widget-create editable-field/checkbox/radio/menu/item/text,
//! make-button/insert-button/next-previous-button, browse-url/goto-address/ffap.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx367_widget_create_editable_field_with_validation() {
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
    )
}

#[test]
fn div_cx367_widget_checkbox_toggle_cycle() {
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
    )
}

#[test]
fn div_cx367_widget_radio_button_choice() {
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
    )
}

#[test]
fn div_cx367_widget_menu_choice_with_items() {
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
    )
}

#[test]
fn div_cx367_widget_field_navigation() {
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
    )
}

#[test]
fn div_cx367_button_make_and_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "Some text content here")
      (make-button 6 10 'action (lambda (b) (message "clicked"))
                   'help-echo "Click"
                   'face 'link
                   'mouse-face 'highlight)
      (let ((btn (button-at 7)))
        (list (buttonp btn)
              (when btn (button-start btn))
              (when btn (button-end btn))
              (when btn (button-get btn 'help-echo))
              (when btn (button-get btn 'face))
              (length (overlays-in 1 20)))))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx367_button_next_previous_navigation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "text one text two text three text four")
      (make-button 6 9)
      (make-button 16 19)
      (make-button 26 31)
      (goto-char 1)
      (let ((b1 (next-button (point))))
        (let ((b2 (when b1 (next-button (button-start b1)))))
          (let ((b3 (when b2 (next-button (button-start b2)))))
            (let ((back (when b3 (previous-button (button-start b3)))))
              (list (and b1 (button-start b1))
                    (and b2 (button-start b2))
                    (and b3 (button-start b3))
                    (and back (button-start back))))))))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx367_browse_url_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'browse-url)
      (list (fboundp 'browse-url)
            (fboundp 'browse-url-at-point)
            (boundp 'browse-url-browser-function)))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx367_thing_at_point_url_email_filename() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (with-temp-buffer
   (insert "see https://example.com/path for details")
   (goto-char 5)
   (thing-at-point 'url))
 (with-temp-buffer
   (insert "contact user@example.com for info")
   (goto-char 10)
   (thing-at-point 'email))
 (with-temp-buffer
   (insert "edit /home/user/file.txt for changes")
   (goto-char 6)
   (thing-at-point 'filename)))
"##,
    )
}

#[test]
fn div_cx367_widget_button_browse_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'widget)
      (require 'button)
      (require 'browse-url)
      (with-temp-buffer
        (buffer-enable-undo)
        (insert "Widget/button/browse-url mega test buffer content")
        (put-text-property 1 6 'face 'bold)
        (make-button 7 13 'action (lambda (_) :clicked) 'face 'link)
        (let ((m (set-marker (make-marker) 10))
              (ov (make-overlay 4 18)))
          (overlay-put ov 'face 'italic)
          (overlay-put ov 'evaporate t)
          (narrow-to-region 2 25)
          (let ((btn (button-at 8)))
            (let ((state (list (buttonp btn)
                               (when btn (button-start btn))
                               (when btn (button-end btn))
                               (fboundp 'browse-url)
                               (fboundp 'widget-create)
                               (buffer-string)
                               (marker-position m)
                               (overlay-start ov) (overlay-end ov)
                               (text-properties-at 1))))
              (undo)
              (widen()
              (list state (buffer-string) (marker-position m)
                    (overlay-start ov) (overlay-end ov)
                    (text-properties-at 1)))))))
  (error (list :errored (car e))))
"##,
    )
}
