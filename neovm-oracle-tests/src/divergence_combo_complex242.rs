//! Complex combo batch 242 — `minibuffer` / `recursive-edit` /
//! `enable-recursive-minibuffers` / `minibuffer-depth` /
//! `read-from-minibuffer` / `read-string` / `completing-read` availability.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx242_minibuffer_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'read-from-minibuffer)
      (fboundp 'read-string)
      (fboundp 'read-no-blanks-input)
      (fboundp 'completing-read)
      (fboundp 'read-char)
      (fboundp 'read-event)
      (fboundp 'read-key)
      (fboundp 'read-command)
      (fboundp 'read-variable)
      (fboundp 'read-function))
"##,
    );
}

#[test]
fn div_cx242_minibuffer_depth_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (>= (minibuffer-depth) 0)
      (integerp (minibuffer-depth)))
"##,
    );
}

#[test]
fn div_cx242_enable_recursive_minibuffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'enable-recursive-minibuffers)
      (booleanp enable-recursive-minibuffers))
"##,
    );
}

#[test]
fn div_cx242_minibuffer_window_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((mw (minibuffer-window)))
  (list (windowp mw)
        (minibufferp mw)
        (window-minibuffer-p mw)))
"##,
    );
}

#[test]
fn div_cx242_active_minibuffer_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (windowp (active-minibuffer-window))
      (eq (active-minibuffer-window) (minibuffer-window)))
"##,
    );
}

#[test]
fn div_cx242_minibuffer_history_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'minibuffer-history)
      (boundp 'file-name-history)
      (boundp 'extended-command-history)
      (boundp 'command-history)
      (boundp 'shell-command-history)
      (boundp 'regexp-history)
      (boundp 'search-ring)
      (boundp 'regexp-search-ring))
"##,
    );
}

#[test]
fn div_cx242_minibuffer_prompt_setup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'minibuffer-prompt)
          (fboundp 'minibuffer-message)
          (fboundp 'minibuffer-complete)
          (fboundp 'minibuffer-complete-word)
          (boundp 'minibuffer-prompt-properties)
          (boundp 'minibuffer-electric-default-map))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx242_minibuffer_completion_helpers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'minibuffer-completion-help)
      (fboundp 'minibuffer-complete-and-exit)
      (fboundp 'exit-minibuffer)
      (fboundp 'minibuffer-completion-confirm)
      (boundp 'completion-show-commit-message))
"##,
    );
}

#[test]
fn div_cx242_minibuffer_keymap_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'minibuffer-local-map)
      (boundp 'minibuffer-local-ns-map)
      (boundp 'minibuffer-local-completion-map)
      (boundp 'minibuffer-local-must-match-map)
      (keymapp minibuffer-local-map)
      (keymapp minibuffer-local-completion-map))
"##,
    );
}

#[test]
fn div_cx242_minibuffer_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((mw (minibuffer-window)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "Minibuffer mega test buffer content")
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (let ((state (list (minibuffer-depth)
                         (windowp mw)
                         (minibufferp mw)
                         (boundp 'enable-recursive-minibuffers)
                         (boundp 'minibuffer-local-map)
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
    );
}
