//! Divergence tests: eval-region, eval-buffer, eval-defun, loaddefs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_eval_region_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((result nil))
  (with-temp-buffer
    (insert "(setq my-eval-test 42)")
    (eval-region (point-min) (point-max)))
  my-eval-test)"#,
    );
}

#[test]
fn divergence_eval_buffer_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "(+ 1 2 3)")
  (eval-buffer (current-buffer)))"#,
    );
}

#[test]
fn divergence_eval_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defun my-eval-defun-test (x) (* x x))
  (list (my-eval-defun-test 5)
        (my-eval-defun-test 10)))"#,
    );
}

#[test]
fn divergence_eval_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (eval '(+ 1 2))
  (eval '(list 1 2 3))
  (eval 't)
  (eval 'nil))"#,
    );
}

#[test]
fn divergence_eval_lexical_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((x 42))
  (list (eval 'x)
        (eval 'x t)
        (let ((x 99))
          (eval 'x))))"#,
    );
}

#[test]
fn divergence_load_suffixes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (listp load-suffixes)
  (member ".elc" load-suffixes)
  (member ".el" load-suffixes)
  (listp load-file-rep-suffixes))"#,
    );
}

#[test]
fn divergence_load_source_file_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'load-file)
  (fboundp 'load-library)
  (fboundp 'locate-library)
  (stringp (locate-library "subr")))"#,
    );
}

#[test]
fn divergence_read_from_string_positions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (read-from-string "(a b c) (d e)" 0)
  (read-from-string "(a b c) (d e)" 8)
  (car (read-from-string "(a b c) (d e)" 0))
  (cdr (read-from-string "(a b c) (d e)" 0)))"#,
    );
}

#[test]
fn divergence_read_multiple_forms() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((str "(a b) (c d) (e f)")
        (p1 (read-from-string str 0))
        (p2 (read-from-string str (cdr p1)))
        (p3 (read-from-string str (cdr p2))))
  (list (car p1) (car p2) (car p3)))"#,
    );
}

#[test]
fn divergence_standard_input() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'read-from-minibuffer)
  (fboundp 'read-string)
  (fboundp 'read-number)
  (fboundp 'read-regexp))"#,
    );
}
