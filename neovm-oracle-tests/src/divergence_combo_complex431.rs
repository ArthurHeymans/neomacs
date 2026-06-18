//! Complex combo batch 431 — 18 probes into tab-bar, frame display,
//! window state/swap, select-window, with-selected-window/frame,
//! window combination constraints, tty-type, frame-geometry,
//! frame-list-z-order-delete, and pixel-resolution.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// tab-bar-mode / tab-bar-new-tab / tab-bar-close-tab.
#[test]
fn div_cx431_tab_bar_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'tab-bar)
  (list (boundp 'tab-bar-mode)
        (fboundp 'tab-bar-new-tab)
        (fboundp 'tab-bar-close-tab)))
"##,
    );
}

/// tab-line-mode / global-tab-line-mode.
#[test]
fn div_cx431_tab_line_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'tab-line)
  (list (boundp 'global-tab-line-mode)
        (boundp 'tab-line-tab-name-format-function)))
"##,
    );
}

/// window-swap-states / window-state-buffers: state exchange.
#[test]
fn div_cx431_window_swap_states() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "buffer a")
  (let ((state (window-state-get (selected-window))))
    (condition-case e
        (window-state-buffers state)
      (error (car e)))))
"##,
    );
}

/// select-window with norecord flag.
#[test]
fn div_cx431_select_window_norecord() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (select-window w 'norecord)
  (selected-window))
"##,
    );
}

/// with-selected-window / with-selected-frame.
#[test]
fn div_cx431_with_selected_window_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (with-selected-window w
    (with-selected-frame (selected-frame)
      (buffer-name))))
"##,
    );
}

/// window-combination-resize / window-combination-limit.
#[test]
fn div_cx431_window_combination() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (list (window-combination-resize w)
        (window-combination-limit w)))
"##,
    );
}

/// window-splits / window-combination-p.
#[test]
fn div_cx431_window_splits_comb() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (list (window-splits w)
        (window-combined-p w t)))
"##,
    );
}

/// tty-type: terminal type as a string.
#[test]
fn div_cx431_tty_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (tty-type) (error (car e))))
"##,
    );
}

/// frame-geometry: frame position and size.
#[test]
fn div_cx431_frame_geometry() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((g (frame-geometry (selected-frame))))
      (list (assq 'outer-position g)
            (assq 'outer-size g)))
  (error (car e)))
"##,
    );
}

/// frame-list-z-order-delete: frame stacking order.
#[test]
fn div_cx431_frame_list_z_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (length (frame-list-z-order-delete 1))
  (error (car e)))
"##,
    );
}

/// frame-restack: restacking frames (may be stubbed).
#[test]
fn div_cx431_frame_restack() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (frame-restack (selected-frame) (selected-frame) nil)
  (error (car e)))
"##,
    );
}

/// frame-monitor-attributes / frame-attribute.
#[test]
fn div_cx431_frame_monitor_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((m (frame-monitor-attributes)))
      (list (listp m) (> (length m) 0)))
  (error (car e)))
"##,
    );
}

/// pixel-resolution-width / pixel-resolution-height.
#[test]
fn div_cx431_pixel_resolution() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'pixel-resolution)
  (list (condition-case e (pixel-resolution-width) (error (car e)))
        (condition-case e (pixel-resolution-height) (error (car e)))))
"##,
    );
}

/// buffer-local-set-state / buffer-local-restore-state.
#[test]
fn div_cx431_buffer_local_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (setq-local neo-cx431-var 'original)
  (let ((state (buffer-local-set-state 'neo-cx431-var 'modified)))
    (list neo-cx431-var
          (progn (buffer-local-restore-state state)
                 neo-cx431-var))))
"##,
    );
}

/// process-get / process-put with large plist.
#[test]
fn div_cx431_process_plist_large() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx431-pl"
                          :command '("echo" "done")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (process-put proc 'key1 'val1)
  (process-put proc 'key2 'val2)
  (process-put proc 'key3 'val3)
  (prog1 (list (process-get proc 'key2)
               (process-get proc 'key4 'default))
    (delete-process proc)))
"##,
    );
}

/// delete-process on already-exited process.
#[test]
fn div_cx431_delete_process_exited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx431-dp"
                          :command '("echo" "done")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (delete-process proc)
  (delete-process proc)
  'ok)
"##,
    );
}

/// window-pixel-width-before/after-size-change.
#[test]
fn div_cx431_window_pixel_size_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (list (window-pixel-width w)
        (window-pixel-height w)))
"##,
    );
}
