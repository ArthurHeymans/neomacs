//! Divergence tests: misc predicates, type-checking, equality edge.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_type_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (type-of 42)
  (type-of "hello")
  (type-of '(1 2))
  (type-of [1 2])
  (type-of 'foo)
  (type-of ?A)) "#,
    );
}

#[test]
fn divergence_equality_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (equal '(1 2 3) '(1 2 3))
  (equal [1 2 3] [1 2 3])
  (eq 'foo 'foo)
  (eql 42 42)
  (equal-including-properties "abc" "abc")) "#,
    );
}

#[test]
fn divergence_number_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (numberp 42)
  (numberp 3.14)
  (numberp "string")
  (integerp 42)
  (integerp 3.14)
  (floatp 3.14)
  (floatp 42)
  (natnump 5)
  (natnump -1)) "#,
    );
}

#[test]
fn divergence_sequence_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (sequencep '(1 2 3))
  (sequencep [1 2 3])
  (sequencep "abc")
  (listp '(1 2))
  (listp nil)
  (consp '(1 2))
  (arrayp [1 2])
  (arrayp "abc")
  (stringp "abc")) "#,
    );
}

#[test]
fn divergence_nil_t_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (null nil)
  (null t)
  (not nil)
  (not t)
  (booleanp nil)
  (booleanp t)
  (booleanp 0)) "#,
    );
}

#[test]
fn divergence_char_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (characterp ?A)
  (characterp 128)
  (characterp #x4e2d)
  (characterp ?\n)
  (wholenump 5)
  (wholenump -1)) "#,
    );
}

#[test]
fn divergence_function_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (functionp 'car)
  (functionp 'lambda)
  (functionp 42)
  (subrp (symbol-function 'car))
  (byte-code-function-p (symbol-function 'car))
  (commandp 'car)) "#,
    );
}

#[test]
fn divergence_buffer_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (bufferp (current-buffer))
  (bufferp nil)
  (buffer-live-p (current-buffer))
  (buffer-modified-p (current-buffer))
  (buffer-file-name (current-buffer))) "#,
    );
}

#[test]
fn divergence_marker_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((m (make-marker)))
  (list (markerp m)
        (marker-position m)
        (marker-buffer m)
        (set-marker m 1 (current-buffer))
        (marker-position m)
        (markerp 42))) "#,
    );
}

#[test]
fn divergence_window_frame_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (windowp (selected-window))
  (window-live-p (selected-window))
  (window-valid-p (selected-window))
  (framep (selected-frame))
  (frame-live-p (selected-frame))) "#,
    );
}

#[test]
fn divergence_process_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'processp)
  (processp nil)
  (fboundp 'process-live-p)) "#,
    );
}
