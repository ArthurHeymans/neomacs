//! Oracle parity for key description + subr metadata.
//! GNU src/keyboard.c, src/data.c, src/eval.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_key_description_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(key-description "a")"#);
    assert_ok_eq("\"a\"", &o, &n);
}

#[test]
fn oracle_single_key_description_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(single-key-description ?a)"#);
    assert_ok_eq("\"a\"", &o, &n);
}

#[test]
fn oracle_single_key_description_control() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(single-key-description ?\C-a)"#);
    assert_ok_eq("\"C-a\"", &o, &n);
}

#[test]
fn oracle_subr_arity_car() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(subr-arity (symbol-function 'car))"#);
    assert_ok_eq("(1 . 1)", &o, &n);
}

#[test]
fn oracle_subr_arity_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(subr-arity (symbol-function 'list))"#);
    assert_ok_eq("(0 . many)", &o, &n);
}

#[test]
fn oracle_interactive_form_non_interactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(interactive-form 'car)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_interactive_form_for_interactive_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(consp (interactive-form (lambda () (interactive) 42)))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_accessible_keymaps_returns_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(listp (accessible-keymaps))"#);
    assert_ok_eq("t", &o, &n);
}
