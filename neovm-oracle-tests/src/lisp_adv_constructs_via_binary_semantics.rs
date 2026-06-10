//! Oracle parity for advanced Lisp constructs via binary execution.
//! Tests defmacro, closures, apply, fboundp, symbol-function, macrop.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- defmacro ---

#[test]
fn oracle_defmacro_basic_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defmacro nvm--test-dm (x) (list '+ x 1))
  (nvm--test-dm 5))"#,
    );
    assert_ok_eq("6", &o, &n);
}

#[test]
fn oracle_defmacro_returns_lambda_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defmacro nvm--test-dm2 (x) (list 'quote x))
  (nvm--test-dm2 42))"#,
    );
    assert_ok_eq("42", &o, &n);
}

// --- closures ---

#[test]
fn oracle_closure_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defun nvm--mk-adder (n)
    (lambda (x) (+ x n)))
  (funcall (nvm--mk-adder 10) 5))"#,
    );
    assert_ok_eq("15", &o, &n);
}

// --- apply ---

#[test]
fn oracle_apply_with_list_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defun nvm--apply-fn (a b c) (list a b c))
  (apply 'nvm--apply-fn '(1 2 3)))"#,
    );
    assert_ok_eq("(1 2 3)", &o, &n);
}

#[test]
fn oracle_apply_with_args_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(apply '+ 1 2 '(3 4))"#);
    assert_ok_eq("10", &o, &n);
}

// --- fboundp ---

#[test]
fn oracle_fboundp_defined_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defun nvm--fb-fn () 42)
  (fboundp 'nvm--fb-fn))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_fboundp_undefined_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(fboundp 'nvm--no-such-function-xyz)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- symbol-function ---

#[test]
fn oracle_symbol_function_returns_function_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defun nvm--sf-fn (x) x)
  (functionp (symbol-function 'nvm--sf-fn)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

// --- macrop on macro symbol-function ---

#[test]
fn oracle_macrop_on_symbol_function_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defmacro nvm--mac-test (x) (list '1+ x))
  (macrop (symbol-function 'nvm--mac-test)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

// --- funcall with lambda ---

#[test]
fn oracle_funcall_lambda_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(funcall (lambda (x y) (+ x y)) 10 20)"#);
    assert_ok_eq("30", &o, &n);
}
