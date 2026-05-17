//! Oracle parity for apply, funcall deep edge cases.
//! GNU src/eval.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- apply with spread ---

#[test]
fn oracle_apply_with_spread_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(apply '+ 1 2 '(3 4 5))"#);
    assert_ok_eq("15", &o, &n);
}

#[test]
fn oracle_apply_with_empty_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(apply '+ '())"#);
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_apply_with_single_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(apply 'length '((a b c)))"#);
    assert_ok_eq("3", &o, &n);
}

// --- funcall ---

#[test]
fn oracle_funcall_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(funcall '+ 1 2 3)"#);
    assert_ok_eq("6", &o, &n);
}

#[test]
fn oracle_funcall_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(funcall (lambda (x y) (+ x y)) 10 20)"#);
    assert_ok_eq("30", &o, &n);
}

// --- apply + funcall interaction ---

#[test]
fn oracle_apply_funcall_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(apply 'funcall (list '+ 1 2 3))"#);
    assert_ok_eq("6", &o, &n);
}

// --- mapcar with various functions ---

#[test]
fn oracle_mapcar_with_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mapcar '1+ '(1 2 3))"#);
    assert_ok_eq("(2 3 4)", &o, &n);
}

#[test]
fn oracle_mapcar_with_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mapcar (lambda (x) (* x 2)) '(1 2 3))"#);
    assert_ok_eq("(2 4 6)", &o, &n);
}

// --- mapc returns original list ---

#[test]
fn oracle_mapc_returns_original_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (setq lst '(1 2 3)) (eq lst (mapc 'ignore lst)))"#);
    assert_ok_eq("t", &o, &n);
}
