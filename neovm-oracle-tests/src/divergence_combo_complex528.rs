/// Batch 528: window-state-get deep, window-state-put with various parameters.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx528_window_state_get_min() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-state-get w))
"##,
    );
}

#[test]
fn div_cx528_window_state_get_ignore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-state-get w t))
"##,
    );
}

#[test]
fn div_cx528_window_state_put_safe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "content")
  (let ((state (window-state-get (selected-window))))
    (window-state-put state nil 'safe)))
"##,
    );
}

#[test]
fn div_cx528_window_state_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "buffer-content")
  (let ((state (window-state-get (selected-window))))
    (window-state-buffers state)))
"##,
    );
}

#[test]
fn div_cx528_window_state_put_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(fboundp 'window-state-put)
"##,
    );
}

#[test]
fn div_cx528_window_state_with_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-parameter w 'test-param 'test-value)
  (let ((state (window-state-get w)))
    (window-state-put state nil 'safe)))
"##,
    );
}

#[test]
fn div_cx528_window_state_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((state (window-state-get (selected-window) t)))
  (list (listp state) (> (length (flatten-tree state)) 10)))
"##,
    );
}

#[test]
fn div_cx528_window_state_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "roundtrip")
  (let ((w (selected-window))
        (buf (current-buffer)))
    (let ((state (window-state-get w t)))
      (window-state-put state w 'safe)
      (window-buffer w))))
"##,
    );
}

#[test]
fn div_cx528_window_state_usable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (let ((state (window-state-get (selected-window))))
    (window-state-usable-state (selected-window) state)))
"##,
    );
}

#[test]
fn div_cx528_window_state_ignore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(window-state-ignored-parameters)
"##,
    );
}

#[test]
fn div_cx528_window_swap_states() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "swapped")
  (window-swap-states nil nil))
"##,
    );
}

#[test]
fn div_cx528_window_state_put_noignore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "no-ignore")
  (let ((state (window-state-get (selected-window) t)))
    (window-state-put state nil 'safe)))
"##,
    );
}

#[test]
fn div_cx528_window_state_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (let ((state (window-state-get (selected-window))))
    (delete-other-windows)
    (window-state-put state nil 'safe)))
"##,
    );
}

#[test]
fn div_cx528_window_state_get_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (let ((state (window-state-get (selected-window))))
    (eq (length state) 0))
"##,
    );
}

#[test]
fn div_cx528_window_state_norecord() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "content")
  (let ((state (window-state-get (selected-window) t)))
    (window-state-put state nil 'norecord)))
"##,
    );
}
