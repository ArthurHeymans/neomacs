//! Divergence tests: advsi dispatch, subr argument parsing, lambda list edge cases.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_lambda_rest_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((fn (lambda (&rest args) args)))
  (list (funcall fn)
        (funcall fn 1)
        (funcall fn 1 2 3)))"#,
    );
}

#[test]
fn divergence_lambda_optional_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(cl-flet ((fn ((a 10) (b 20)) (+ a b)))
  (list (fn 1 2)
        (fn 1)
        (fn)))"#,
    );
}

#[test]
fn divergence_lambda_key_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(cl-flet ((fn (&key a b) (list a b)))
  (list (fn :a 1 :b 2)
        (fn :a 1)
        (fn)
        (fn :b 3 :a 4)))"#,
    );
}

#[test]
fn divergence_apply_with_splice() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (apply #'+ 1 2 '(3 4))
  (apply #'+ '(1 2 3))
  (apply #'list 'a 'b '(c d e)))"#,
    );
}

#[test]
fn divergence_funcall_vs_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (funcall #'+ 1 2 3)
  (apply #'+ 1 2 '(3))
  (eq (funcall #'identity 42) 42))"#,
    );
}

#[test]
fn divergence_macroexpand_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (macroexpand '(when t 42))
  (macroexpand '(and 1 2 3))
  (macroexpand '(or nil 42)))"#,
    );
}

#[test]
fn divergence_define_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'define-inline)
  (fboundp 'inline-let)
  (fboundp 'inline-quote))"#,
    );
}

#[test]
fn divergence_closure_over_let_star() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((x 1)
        (f (lambda () x))
        (x 2))
  (list (funcall f) x))"#,
    );
}

#[test]
fn divergence_nested_let_star_bindings() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((a 1)
         (b (1+ a))
         (c (+ a b)))
  (list a b c))"#,
    );
}

#[test]
fn divergence_setq_default_buffer_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-sqbl-var 0)
  (setq-default my-sqbl-var 10)
  (set (make-local-variable 'my-sqbl-var) 20)
  (list my-sqbl-var
        (default-value 'my-sqbl-var)
        (buffer-local-value 'my-sqbl-var (current-buffer))))"#,
    );
}
