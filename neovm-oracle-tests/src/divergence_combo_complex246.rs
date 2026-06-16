//! Complex combo batch 246 — `clipboard` / `selection` / `interprogram-
//! cut-function` / `interprogram-paste-function` / `gui-select-text` /
//! `gui-selection-owner-p` deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx246_interprogram_cut_paste_functions_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'interprogram-cut-function)
      (boundp 'interprogram-paste-function)
      (or (null interprogram-cut-function) (functionp interprogram-cut-function))
      (or (null interprogram-paste-function) (functionp interprogram-paste-function)))
"##,
    )
}

#[test]
fn div_cx246_gui_selection_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'gui-set-selection)
          (fboundp 'gui-get-selection)
          (fboundp 'gui-selection-owner-p)
          (fboundp 'gui-backend-select-text)
          (fboundp 'gui-backend-get-selection)
          (fboundp 'gui-backend-selection-owner-p))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx246_x_selection_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'x-set-selection)
          (fboundp 'x-get-selection)
          (fboundp 'x-selection-owner-p)
          (fboundp 'x-own-selection-internal)
          (fboundp 'x-disown-selection-internal))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx246_selection_coding_system_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'selection-coding-system)
      (boundp 'next-selection-coding-system)
      (boundp 'gui-select-enable-clipboard)
      (boundp 'x-select-enable-clipboard))
"##,
    )
}

#[test]
fn div_cx246_clipboard_functions_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'clipboard-kill-ring-save)
      (fboundp 'clipboard-kill-region)
      (fboundp 'clipboard-yank)
      (fboundp 'clipboard-yank-rectangle))
"##,
    )
}

#[test]
fn div_cx246_kill_ring_and_clipboard_interaction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((kill-ring nil))
  (push "test-string-1" kill-ring)
  (push "test-string-2" kill-ring)
  (list kill-ring
        (current-kill 0 t)
        (current-kill 1 t)
        (car kill-ring)))
"##,
    )
}

#[test]
fn div_cx246_x_select_request_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (boundp 'x-select-request-type)
          (boundp 'x-select-enable-clipboard-manager))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx246_register_clipboard_connection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((kill-ring nil))
  (kill-new "clipboard test content")
  (list kill-ring
        (car kill-ring)
        (current-kill 0 t)))
"##,
    )
}

#[test]
fn div_cx246_clipboard_manager_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'emacs-clipboard-manager-exit-hook)
      (boundp 'x-lost-selection-functions)
      (boundp 'x-sent-selection-functions))
"##,
    )
}

#[test]
fn div_cx246_clipboard_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((kill-ring nil))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "Clipboard mega test buffer content")
    (put-text-property 1 6 'face 'bold)
    (kill-new "clipboard-mega")
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (let ((state (list kill-ring
                         (current-kill 0 t)
                         (boundp 'interprogram-cut-function)
                         (boundp 'gui-set-selection)
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    )
}
