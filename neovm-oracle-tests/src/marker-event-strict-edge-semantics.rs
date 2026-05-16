//! Oracle parity for marker + event operations.
//! GNU src/marker.c, src/keyboard.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_make_marker_creates_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(markerp (make-marker))");
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_set_marker_returns_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mk*")) (erase-buffer) (insert "0123456789") (markerp (set-marker (make-marker) 5)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_copy_marker_preserves_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mk2*")) (erase-buffer) (insert "0123456789") (let* ((m (set-marker (make-marker) 3)) (c (copy-marker m))) (eq (marker-position m) (marker-position c))))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_marker_insertion_type_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(marker-insertion-type (make-marker))");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_set_marker_nil_detaches() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (let ((m (make-marker))) (set-marker m 10 (current-buffer)) (set-marker m nil) (marker-position m)))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_single_key_description_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(single-key-description ?a)"#);
    assert_ok_eq("\"a\"", &o, &n);
}
