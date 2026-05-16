//! Oracle parity tests for condition-case error handling — strict edges.
//!
//! Uses only built-in subrs (no bootstrap macros).  Tests error
//! re-signaling, handler matching, and body pass-through.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_condition_case_no_error_returns_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(condition-case nil 42 (error 'never))"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_condition_case_arith_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(condition-case err (/ 1 0) (arith-error 'caught-div))"#);
    assert_ok_eq("caught-div", &o, &n);
}

#[test]
fn oracle_condition_case_handler_not_found_propagates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(condition-case nil
         (condition-case nil (/ 1 0) (void-variable 'wrong-handler))
       (arith-error 'outer-caught))"#,
    );
    assert_ok_eq("outer-caught", &o, &n);
}

#[test]
fn oracle_condition_case_multiple_handlers_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(condition-case err (/ 1 0) (arith-error 'arithmetic) (error 'generic))"#,
    );
    assert_ok_eq("arithmetic", &o, &n);
}

#[test]
fn oracle_condition_case_error_data_is_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(condition-case err (/ 1 0) (arith-error (consp err)))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_condition_case_nested_catches_at_right_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(condition-case nil
         (condition-case nil (/ 1 0) (arith-error 'inner))
       (arith-error 'outer))"#,
    );
    assert_ok_eq("inner", &o, &n);
}
