//! Oracle parity for eval/apply/funcall/macroexpand strict edges.
//! GNU src/eval.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_eval_self_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eval 42)"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_eval_quoted() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eval '(+ 1 2))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_apply_simple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(apply '+ '(1 2 3))"#);
    assert_ok_eq("6", &o, &n);
}

#[test]
fn oracle_funcall_simple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(funcall '+ 1 2 3)"#);
    assert_ok_eq("6", &o, &n);
}

#[test]
fn oracle_macroexpand_if() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(macroexpand '(if t 1 2))"#);
    assert_ok_eq("(if t 1 2)", &o, &n);
}

#[test]
fn oracle_apply_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(apply '+ nil)"#);
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_funcall_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(funcall (lambda () 99))"#);
    assert_ok_eq("99", &o, &n);
}
