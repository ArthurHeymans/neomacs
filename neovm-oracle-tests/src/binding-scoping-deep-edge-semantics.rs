//! Oracle parity for let, let*, setq, defvar, defconst scoping edge cases.
//! GNU src/eval.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- let parallel binding ---

#[test]
fn oracle_let_parallel_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(let ((x 1) (y 2)) (+ x y))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_let_shadowing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Inner let shadows outer, outer preserved after inner ends
    let (o, n) = eval_oracle_and_neovm(r#"(let ((x 1)) (let ((x 2)) x) x)"#);
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_let_nil_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(let ((x nil)) x)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- let* sequential binding ---

#[test]
fn oracle_let_star_sequential() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // let* allows later bindings to reference earlier ones
    let (o, n) = eval_oracle_and_neovm(r#"(let* ((x 1) (y (+ x 10))) y)"#);
    assert_ok_eq("11", &o, &n);
}

#[test]
fn oracle_let_vs_let_star_parallel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // In let, bindings are parallel — y can't see x
    let (o, n) = eval_oracle_and_neovm(r#"(let ((x 1)) (let ((x 10) (y x)) y))"#);
    // y gets outer x (1), not inner x (10)
    assert_ok_eq("1", &o, &n);
}

// --- setq ---

#[test]
fn oracle_setq_multiple_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(progn (setq a 1 b 2 c 3) (list a b c))"#);
    assert_ok_eq("(1 2 3)", &o, &n);
}

#[test]
fn oracle_setq_returns_last_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(setq zz 42)"#);
    assert_ok_eq("42", &o, &n);
}

// --- defvar ---

#[test]
fn oracle_defvar_no_override() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // defvar does not override an existing value
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (setq dv-test 100) (defvar dv-test 200) dv-test)"#);
    assert_ok_eq("100", &o, &n);
}

#[test]
fn oracle_defvar_initializes_unbound() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(progn (defvar dv-test-new 42) dv-test-new)"#);
    assert_ok_eq("42", &o, &n);
}

// --- defconst ---

#[test]
fn oracle_defconst_sets_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(progn (defconst dc-test 42) dc-test)"#);
    assert_ok_eq("42", &o, &n);
}
