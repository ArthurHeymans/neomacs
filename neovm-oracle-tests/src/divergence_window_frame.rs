//! Window & frame management divergence probes (calibration).
//!
//! Probes frame lifecycle, frame dimensions/parameters, window-tree structure,
//! window edges, parent/child/sibling, window-buffer/point, split-window,
//! window parameters, window configurations, and terminals — all under the
//! `--batch` oracle harness where the window/frame model is minimal.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- frame basics -----------------------------------------------------------

#[test]
fn div_wf_frame_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (frame-live-p (selected-frame))
      (framep (selected-frame))
      (length (frame-list))
      (eq (car (frame-list)) (selected-frame))
      (eq (selected-frame) (terminal-frame (frame-terminal))))
"##,
    );
}

#[test]
fn div_wf_frame_dimensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (frame-width)
      (frame-height)
      (frame-char-width)
      (frame-char-height)
      (frame-pixel-width)
      (frame-pixel-height))
"##,
    );
}

#[test]
fn div_wf_frame_parameters_common() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (frame-parameter nil 'name)
      (frame-parameter nil 'width)
      (frame-parameter nil 'height)
      (frame-parameter nil 'foreground-color)
      (frame-parameter nil 'background-color)
      (frame-parameter nil 'cursor-color)
      (frame-parameter nil 'mouse-color)
      (frame-parameter nil 'border-color)
      (frame-parameter nil 'font)
      (frame-parameter nil 'minibuffer)
      (frame-parameter nil 'menu-bar-lines)
      (frame-parameter nil 'tool-bar-lines)
      (frame-parameter nil 'vertical-scroll-bars)
      (frame-parameter nil 'visibility)
      (frame-parameter nil 'auto-raise)
      (frame-parameter nil 'auto-lower)
      (frame-parameter nil 'fullscreen)
      (frame-parameter nil 'background-mode)
      (frame-parameter nil 'display-type))
"##,
    );
}

// --- window structure -------------------------------------------------------

#[test]
fn div_wf_window_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (window-live-p (selected-window))
      (windowp (selected-window))
      (eq (selected-window) (frame-selected-window))
      (window-minibuffer-p (minibuffer-window))
      (window-minibuffer-p (selected-window))
      (window-valid-p (selected-window)))
"##,
    );
}

#[test]
fn div_wf_window_tree_and_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (car (window-tree))
      (count-windows)
      (length (window-list nil 'nomini))
      (length (window-list nil nil))
      (length (window-list nil 'nomini (frame-first-window))))
"##,
    );
}

#[test]
fn div_wf_window_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (window-edges (selected-window))
      (window-inside-edges (selected-window))
      (window-pixel-edges (selected-window))
      (window-inside-pixel-edges (selected-window)))
"##,
    );
}

#[test]
fn div_wf_window_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (window-total-height (selected-window))
      (window-total-width (selected-window))
      (window-body-height (selected-window))
      (window-body-width (selected-window))
      (window-mode-line-height (selected-window)))
"##,
    );
}

#[test]
fn div_wf_window_parent_child_sibling() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((w (selected-window))
       (parent (window-parent w)))
  (list (windowp parent)
        (window-next-sibling w)
        (window-prev-sibling w)
        (eq (frame-root-window) parent)
        (windowp (frame-root-window))))
"##,
    );
}

#[test]
fn div_wf_split_window_vertical() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case err
    (let ((w (split-window nil nil 'below)))
      (list (window-live-p w)
            (windowp (window-parent w))
            (eq (window-next-sibling w) (selected-window))
            (count-windows)))
  (error (cons 'errored (car err))))
"##,
    );
}

#[test]
fn div_wf_split_window_horizontal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case err
    (let ((w (split-window nil nil 'right)))
      (list (window-live-p w)
            (window-combined-p w)
            (window-combined-p w 'horizontal)
            (count-windows)))
  (error (cons 'errored (car err))))
"##,
    );
}

#[test]
fn div_wf_split_then_delete_other_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case err
    (progn
      (split-window)
      (split-window nil nil 'right)
      (let ((n1 (count-windows)))
        (delete-other-windows)
        (list n1 (count-windows))))
  (error (cons 'errored (car err))))
"##,
    );
}

// --- window-buffer / window-point ------------------------------------------

#[test]
fn div_wf_window_buffer_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *wf-test*")))
  (set-window-buffer (selected-window) buf t)
  (list (eq (window-buffer (selected-window)) buf)
        (buffer-name (window-buffer (selected-window))))
  (kill-buffer buf))
"##,
    );
}

#[test]
fn div_wf_get_buffer_window_lru_mru() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (windowp (get-lru-window))
      (windowp (get-mru-window))
      (windowp (get-buffer-window (current-buffer))))
"##,
    );
}

#[test]
fn div_wf_walk_windows_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((n 0))
  (walk-windows (lambda (w) (setq n (1+ n))) 'nomini))
  n)
"##,
    );
}

// --- window parameters ------------------------------------------------------

#[test]
fn div_wf_window_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (set-window-parameter w 'wf-param 'hello)
  (list (window-parameter w 'wf-param)
        (window-parameter w 'nonexistent)
        (window-parameters w)))
"##,
    );
}

#[test]
fn div_wf_window_dedicated() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (set-window-dedicated-p w t)
  (list (window-dedicated-p w)
        (progn (set-window-dedicated-p w nil) (window-dedicated-p w))))
"##,
    );
}

// --- window configuration round-trip ---------------------------------------

#[test]
fn div_wf_window_configuration_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case err
    (let ((cfg (current-window-configuration)))
      (split-window)
      (let ((n1 (count-windows)))
        (set-window-configuration cfg)
        (list (window-configuration-p cfg) n1 (count-windows))))
  (error (cons 'errored (car err))))
"##,
    );
}

#[test]
fn div_wf_save_window_excursion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case err
    (save-window-excursion
      (split-window)
      (split-window nil nil 'right)
      (count-windows))
  (error (cons 'errored (car err))))
"##,
    );
}

// --- terminal ---------------------------------------------------------------

#[test]
fn div_wf_terminal_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((term (frame-terminal (selected-frame))))
  (list (terminal-live-p term)
        (terminalp term)
        (eq (terminal-name term) (terminal-name term))))
"##,
    );
}

#[test]
fn div_wf_make_frame_in_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case err
    (let ((n-before (length (frame-list))))
      (let ((f (make-frame)))
        (list n-before (length (frame-list)) (frame-live-p f))))
  (error (cons 'errored (car err))))
"##,
    );
}

#[test]
fn div_wf_window_start_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-current-buffer (get-buffer-create " *wf-wstart*")
  (insert "line one\nline two\nline three\n")
  (set-window-buffer (selected-window) (current-buffer))
  (list (window-start (selected-window))
        (window-end (selected-window))))
"##,
    );
}

#[test]
fn div_wf_frame_first_and_root_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (windowp (frame-first-window))
      (windowp (frame-root-window))
      (eq (frame-first-window) (frame-first-window (selected-frame)))
      (eq (frame-selected-window) (selected-window)))
"##,
    );
}

#[test]
fn div_wf_balance_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case err
    (progn
      (split-window)
      (split-window nil nil 'right)
      (balance-windows)
      (list (count-windows)
            (window-total-height (selected-window))
            (window-total-width (selected-window))))
  (error (cons 'errored (car err))))
"##,
    );
}
