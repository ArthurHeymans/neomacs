//! Divergence tests: register, bookmark stubs, and narrow/widen edge cases.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_point_to_register() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (goto-char 5)
  (point-to-register ?a)
  (goto-char 1)
  (jump-to-register ?a)
  (list (point)
        (get-register ?a)))"#,
    );
}

#[test]
fn divergence_copy_to_register() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello World!")
  (copy-to-register ?r 1 6)
  (list (get-register ?r)
        (buffer-string)))"#,
    );
}

#[test]
fn divergence_insert_register() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello World!")
  (copy-to-register ?r 1 6)
  (goto-char 12)
  (insert-register ?r)
  (list (buffer-string)
        (point)))"#,
    );
}

#[test]
fn divergence_register_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (set-register ?x 42)
  (set-register ?y "hello")
  (set-register ?z '(a b c))
  (list (get-register ?x)
        (get-register ?y)
        (get-register ?z)))"#,
    );
}

#[test]
fn divergence_narrow_to_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "line1\nline2\nline3\nline4\nline5")
  (goto-char 8)
  (push-mark)
  (forward-line 2)
  (narrow-to-region 8 (point))
  (list (point-min)
        (point-max)
        (buffer-string)
        (buffer-narrowed-p)))"#,
    );
}

#[test]
fn divergence_widen_after_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "line1\nline2\nline3")
  (narrow-to-region 7 12)
  (list (point-min) (point-max) (buffer-string) (buffer-narrowed-p))
  (widen)
  (list (point-min) (point-max) (buffer-string) (buffer-narrowed-p)))"#,
    );
}

#[test]
fn divergence_narrow_with_markers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (let ((m1 (set-marker (make-marker) 3))
        (m2 (set-marker (make-marker) 8)))
    (narrow-to-region 4 7)
    (list (point-min) (point-max)
          (marker-position m1)
          (marker-position m2)
          (buffer-string))
    (widen)
    (list (marker-position m1)
          (marker-position m2))))"#,
    );
}

#[test]
fn divergence_bookmark_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'bookmark-set)
  (fboundp 'bookmark-jump)
  (fboundp 'bookmark-all-names)
  (fboundp 'bookmark-load))"#,
    );
}

#[test]
fn divergence_fringe_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'set-fringe-mode)
  (fboundp 'fringe-columns)
  (boundp 'overflow-newline-into-fringe)
  (booleanp overflow-newline-into-fringe))"#,
    );
}

#[test]
fn divergence_scroll_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'scroll-up)
  (fboundp 'scroll-down)
  (fboundp 'scroll-left)
  (fboundp 'scroll-right)
  (fboundp 'recenter))"#,
    );
}
