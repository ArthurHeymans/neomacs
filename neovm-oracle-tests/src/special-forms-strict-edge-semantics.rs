//! Oracle parity for special form semantics — strict edge cases.
//! GNU src/eval.c: `and`, `or`, `if`, `cond`, `while`, `progn`,
//! `setq`, `let`, `let*`, `quote`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_and_short_circuit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(and nil (/ 1 0))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_and_last_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(and t 42)"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_or_short_circuit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(or t (/ 1 0))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_or_last_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(or nil nil 42)"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_if_then() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(if t 'yes 'no)"#);
    assert_ok_eq("yes", &o, &n);
}

#[test]
fn oracle_if_else() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(if nil 'yes 'no)"#);
    assert_ok_eq("no", &o, &n);
}

#[test]
fn oracle_while_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (defvar neovm--test-wl 0) (let ((i 0)) (while (< i 5) (setq neovm--test-wl (1+ neovm--test-wl)) (setq i (1+ i))) neovm--test-wl))"#,
    );
    assert_ok_eq("5", &o, &n);
}

#[test]
fn oracle_progn_returns_last() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(progn 1 2 3)"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_let_bindings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(let ((a 1) (b 2)) (+ a b))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_setq_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (defvar neovm--test-sqm nil) (setq neovm--test-sqm 42 neovm--test-sqm (1+ neovm--test-sqm)))"#,
    );
    assert_ok_eq("43", &o, &n);
}
