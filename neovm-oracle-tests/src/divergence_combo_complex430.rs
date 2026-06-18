//! Complex combo batch 430 — 18 final edge probes: charset-priority-list
//! length, coding-system-priority-list length, get-buffer-window-list,
//! window-at, window-list-1, window-live-p, minibuffer-window,
//! selected-window, selected-frame-deep, frame-first-window,
//! frame-root-window, frame-selected-window, window-child, window-parent,
//! window-left-column, window-top-line, window-parameter-alist,
//! window-valid-p.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// get-buffer-window-list: listing windows showing a buffer.
#[test]
fn div_cx430_get_buffer_window_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (current-buffer)))
  (get-buffer-window-list buf nil t))
"##,
    );
}

/// window-at: window by screen coordinates.
#[test]
fn div_cx430_window_at_coords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(window-at 0 0)
"##,
    );
}

/// window-list-1: window list with parameters.
#[test]
fn div_cx430_window_list_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(length (window-list-1 nil nil nil t))
"##,
    );
}

/// window-live-p: checking if window is live.
#[test]
fn div_cx430_window_live_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (window-live-p (selected-window))
      (window-live-p nil)
      (windowp (selected-window)))
"##,
    );
}

/// minibuffer-window / selected-window.
#[test]
fn div_cx430_minibuf_selected_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (window-live-p (minibuffer-window))
      (window-live-p (selected-window))
      (eq (minibuffer-window) (selected-window)))
"##,
    );
}

/// frame-first-window / frame-root-window / frame-selected-window.
#[test]
fn div_cx430_frame_window_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (selected-frame)))
  (list (window-live-p (frame-first-window f))
        (window-live-p (frame-root-window f))
        (window-live-p (frame-selected-window f))))
"##,
    );
}

/// window-child / window-parent / window-valid-p.
#[test]
fn div_cx430_window_child_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (list (window-child w)
        (window-parent w)
        (window-valid-p w)))
"##,
    );
}

/// window-left-column / window-top-line: window positioning.
#[test]
fn div_cx430_window_left_column_top() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (list (window-left-column w)
        (window-top-line w)))
"##,
    );
}

/// window-parameter-alist: accessing parameter storage.
#[test]
fn div_cx430_window_parameter_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (set-window-parameter w 'test-param 'test-val)
  (list (assq 'test-param (window-parameters w))
        (window-parameter w 'test-param)))
"##,
    );
}

/// window-configuration-to-register with different registers.
#[test]
fn div_cx430_window_config_registers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abc")
  (window-configuration-to-register ?a)
  (jump-to-register ?a)
  (buffer-string))
"##,
    );
}

/// line-move-visual with different display properties.
#[test]
fn div_cx430_line_move_visual_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "short line\nlong line with more text")
  (line-move-visual 1)
  (point))
"##,
    );
}

/// frame-char-width / frame-char-height.
#[test]
fn div_cx430_frame_char_dimensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (frame-char-width)
      (frame-char-height))
"##,
    );
}

/// pixelwise window operations.
#[test]
fn div_cx430_window_pixelwise() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (window-pixel-left (selected-window))
      (window-pixel-top (selected-window)))
"##,
    );
}

/// buffer-local-value / default-value deep.
#[test]
fn div_cx430_buffer_local_default_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (setq-local neo-cx430-var 'local-val)
  (setq-default neo-cx430-var 'default-val)
  (list (buffer-local-value 'neo-cx430-var (current-buffer))
        (default-value 'neo-cx430-var)))
"##,
    );
}

/// window-scroll-functions / window-size-change-functions.
#[test]
fn div_cx430_window_scroll_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (window-scroll-functions)
      (window-size-change-functions))
"##,
    );
}

/// safe-length / proper-list-p.
#[test]
fn div_cx430_safe_length_proper_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (safe-length '(a b c))
      (safe-length '(a . b))
      (proper-list-p '(a b c))
      (proper-list-p '(a . b)))
"##,
    );
}

/// set-char-table-default / char-table-subtype deeper.
#[test]
fn div_cx430_char_table_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'category-table)))
  (set-char-table-default ct ?x)
  (list (char-table-subtype ct)
        (aref ct ?x)))
"##,
    );
}

/// force-mode-line-update / current-idle-time.
#[test]
fn div_cx430_force_mode_line_update() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (force-mode-line-update nil)
      (condition-case e (current-idle-time) (error (car e))))
"##,
    );
}
