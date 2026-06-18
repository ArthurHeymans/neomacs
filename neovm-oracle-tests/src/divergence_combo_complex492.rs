/// Batch 492: display-buffer fallback, window state deep, frame state deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx492_display_buffer_fallback() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((display-buffer-fallback-action
           '((display-buffer--maybe-same-window
              display-buffer--maybe-pop-up-window
              display-buffer--maybe-pop-up-frame
              display-buffer--maybe-use-least-recent-window))))
  (let ((buf (get-buffer-create " *cx492-fb*")))
    (window-live-p (display-buffer buf)))
"##,
    );
}

#[test]
fn div_cx492_window_state_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((state (window-state-get (selected-window) t)))
  (list (listp state) (> (length state) 0)))
"##,
    );
}

#[test]
fn div_cx492_frame_state_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((state (window-state-get (selected-window) t)))
  (window-state-put state (selected-window) 'safe))
"##,
    );
}

#[test]
fn div_cx492_window_parameter_get_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-parameter w 'test-1 'val-1)
  (set-window-parameter w 'test-2 'val-2)
  (list (window-parameter w 'test-1) (window-parameter w 'test-2)))
"##,
    );
}

#[test]
fn div_cx492_window_dedicated() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-dedicated-p w t)
  (window-dedicated-p w))
"##,
    );
}

#[test]
fn div_cx492_window_prefer_h_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window))
      (split-window-preferred-function 'split-window-sensibly))
  (window-live-p (condition-case e (split-window-sensibly w) (error (car e)))))
"##,
    );
}

#[test]
fn div_cx492_window_side() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-side w))
"##,
    );
}

#[test]
fn div_cx492_window_atom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-atom w))
"##,
    );
}

#[test]
fn div_cx492_window_nest() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-nest w))
"##,
    );
}

#[test]
fn div_cx492_window_factor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-factor w))
"##,
    );
}

#[test]
fn div_cx492_window_combination() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-combination-p w t))
"##,
    );
}

#[test]
fn div_cx492_window_valid() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-valid-p w))
"##,
    );
}

#[test]
fn div_cx492_window_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (condition-case e (other-window 1) (error (car e))))
"##,
    );
}

#[test]
fn div_cx492_window_scroll_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-scroll-bars w nil)
  (window-scroll-bars w))
"##,
    );
}

#[test]
fn div_cx492_window_display_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-display-table w nil)
  (window-display-table w))
"##,
    );
}
