//! Oracle parity tests for marker edge cases.
//!
//! GNU src/marker.c: `make-marker`, `copy-marker`, `marker-position`,
//! `marker-buffer`, `marker-insertion-type`, `set-marker` — markers
//! are positions that move with text insertions.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

use crate::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_make_marker_creates_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK t""#]];
    let (oracle, neovm) =
        crate::common::eval_oracle_and_neovm_expect("(markerp (make-marker))", expect);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_marker_without_buffer_returns_nil_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    let (oracle, neovm) =
        crate::common::eval_oracle_and_neovm_expect("(marker-position (make-marker))", expect);
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_marker_without_buffer_returns_nil_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    let (oracle, neovm) =
        crate::common::eval_oracle_and_neovm_expect("(marker-buffer (make-marker))", expect);
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_set_marker_assigns_position_and_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (5 t)""#]];
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-mark*"))
  (erase-buffer)
  (insert "0123456789")
  (let ((m (make-marker)))
    (set-marker m 5)
    (list (marker-position m)
          (not (null (marker-buffer m))))))"#,
        expect,
    );
    assert_ok_eq("(5 t)", &oracle, &neovm);
}

#[test]
fn oracle_marker_insertion_type_default_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(marker-insertion-type (make-marker))",
        expect,
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_copy_marker_preserves_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (3 t)""#]];
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-copymark*"))
  (erase-buffer)
  (insert "0123456789")
  (let* ((m (set-marker (make-marker) 3))
         (c (copy-marker m)))
    (list (marker-position c)
          (eq (marker-buffer c) (marker-buffer m)))))"#,
        expect,
    );
    assert_ok_eq("(3 t)", &oracle, &neovm);
}

#[test]
fn oracle_set_marker_nil_detaches() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (nil nil)""#]];
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-detach*"))
  (let ((m (set-marker (make-marker) 10)))
    (set-marker m nil)
    (list (marker-position m) (marker-buffer m))))"#,
        expect,
    );
    assert_ok_eq("(nil nil)", &oracle, &neovm);
}

#[test]
fn oracle_markerp_on_non_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (nil nil nil)""#]];
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list (markerp nil) (markerp 42) (markerp "hello"))"#,
        expect,
    );
    assert_ok_eq("(nil nil nil)", &oracle, &neovm);
}
