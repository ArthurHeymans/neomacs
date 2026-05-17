//! Oracle parity for Lisp binding forms via binary.
//! Requires full bootstrap: if-let, when-let, and-let*, while-let.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm_via_binary};

#[test]
fn oracle_if_let_binds_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(if-let ((x 1)) x)"#);
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_if_let_star_sequential_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(if-let* ((x 1) (y (+ x 2))) y)"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_when_let_binds_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(when-let ((x t)) x)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_when_let_nil_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(when-let ((x nil)) 'never)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_and_let_star_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(and-let* ((x 1) (y 2) (z 3)) z)"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_and_let_star_short_circuit_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(and-let* ((x 1) (y nil) (z 3)) 'never)"#);
    assert_ok_eq("nil", &o, &n);
}
