//! Divergence tests: remaining edge cases - random, counter, format edge.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_buffer_modified_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello")
  (let ((tick1 (buffer-modified-tick)))
    (insert " World")
    (let ((tick2 (buffer-modified-tick)))
      (list (< tick1 tick2)
            (integerp tick1)
            (integerp tick2)))))"#,
    );
}

#[test]
fn divergence_buffer_chars_modified_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello")
  (let ((tick (buffer-chars-modified-tick)))
    (list (integerp tick)
          (>= tick 0))))"#,
    );
}

#[test]
fn divergence_format_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (format-message "hello %s" "world")
  (format-message "`foo' and `bar'")
  (stringp (format-message "%d" 42)))"#,
    );
}

#[test]
fn divergence_propertize_buffer_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello World")
  (put-text-property 1 6 'face 'bold)
  (list (buffer-substring 1 6)
        (buffer-substring-no-properties 1 6)
        (buffer-substring-propertized 1 6)))"#,
    );
}

#[test]
fn divergence_minibuffer_prompt_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'minibuffer-prompt-properties)
  (listp minibuffer-prompt-properties)
  (plist-get minibuffer-prompt-properties 'read-only))"#,
    );
}

#[test]
fn divergence_resize_mini_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'resize-mini-windows)
  (member resize-mini-windows '(nil t grow-only))
  (boundp 'max-mini-window-height))"#,
    );
}

#[test]
fn divergence_enable_recursive_minibuffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (booleanp enable-recursive-minibuffers)
  (boundp 'minibuffer-depth-indicator-function))"#,
    );
}

#[test]
fn divergence_visible_bell() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (booleanp visible-bell)
  (boundp 'ring-bell-function)
  (boundp 'visible-bell))"#,
    );
}

#[test]
fn divergence_wait_delayed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'redisplay-sit-for)
  (fboundp 'sit-for)
  (fboundp 'discard-input))"#,
    );
}

#[test]
fn divergence_track_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'track-mouse)
  (boundp 'track-mouse)
  (fboundp 'mouse-position)
  (fboundp 'mouse-set-point))"#,
    );
}
