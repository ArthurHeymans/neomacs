//! Oracle parity for deep special form edge cases.
//! catch/throw, unwind-protect, prog1/prog2, condition-case handlers.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm, eval_oracle_and_neovm_via_binary};

// --- catch / throw ---

#[test]
fn oracle_catch_throw_returns_thrown_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(catch 'my-tag (throw 'my-tag 42) 99)"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_catch_no_throw_returns_last_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(catch 'my-tag 1 2 3)"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_catch_nested_throw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(catch 'outer (catch 'inner (throw 'outer 'done) 'never) 'also-never)"#,
    );
    assert_ok_eq("done", &o, &n);
}

// --- unwind-protect ---

#[test]
fn oracle_unwind_protect_returns_body_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(unwind-protect 42)"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_unwind_protect_runs_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq uwp-side-effect nil) (unwind-protect 1 (setq uwp-side-effect 'cleaned-up)) uwp-side-effect)"#,
    );
    assert_ok_eq("cleaned-up", &o, &n);
}

// --- prog1 ---

#[test]
fn oracle_prog1_returns_first() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(prog1 1 2 3)"#);
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_prog1_side_effects() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq p1-counter 0) (prog1 (setq p1-counter (+ p1-counter 1)) (setq p1-counter (+ p1-counter 10))) p1-counter)"#,
    );
    assert_ok_eq("11", &o, &n);
}

// --- condition-case ---

#[test]
fn oracle_condition_case_error_handler_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(
        r#"(condition-case err (error "test-message") (error (symbol-name (car err))))"#,
    );
    assert_ok_eq("\"error\"", &o, &n);
}

#[test]
fn oracle_condition_case_no_error_returns_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(condition-case err 42 (error 'not-reached))"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_condition_case_var_is_nil_on_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // When no error, the bound variable (err) is nil
    let (o, n) = eval_oracle_and_neovm(r#"(condition-case err 42 (error err))"#);
    assert_ok_eq("42", &o, &n);
}
