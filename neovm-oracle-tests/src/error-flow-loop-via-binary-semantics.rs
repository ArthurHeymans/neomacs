//! Oracle parity for ignore-errors, condition-case data, cl-loop via binary.
//! These require the full Lisp library.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- ignore-errors ---

#[test]
fn oracle_ignore_errors_success_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(ignore-errors (+ 1 2))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_ignore_errors_error_returns_nil_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(ignore-errors (error "boom"))"#);
    assert_ok_eq("nil", &o, &n);
}

// --- condition-case error data ---

#[test]
fn oracle_condition_case_cdr_is_error_args_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(condition-case err (error "test-message") (error (cdr err)))"#);
    assert_ok_eq("(\"test-message\")", &o, &n);
}

// --- cl-loop (via binary, needs cl-lib) ---

#[test]
fn oracle_cl_loop_collect_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (require 'cl-lib) (cl-loop for i from 1 to 5 collect i))"#);
    assert_ok_eq("(1 2 3 4 5)", &o, &n);
}

#[test]
fn oracle_cl_loop_sum_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (require 'cl-lib) (cl-loop for i from 1 to 5 sum i))"#);
    assert_ok_eq("15", &o, &n);
}

// --- error with various args ---

#[test]
fn oracle_error_signals_properly_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(condition-case err (error "msg-%s" "arg") (error (car err)))"#);
    assert_ok_eq("error", &o, &n);
}
