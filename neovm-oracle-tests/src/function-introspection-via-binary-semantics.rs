//! Oracle parity for subr-arity, special-form-p, function introspection via binary.
//! GNU src/data.c, src/eval.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm_via_binary};

#[test]
fn oracle_subr_arity_car_1_1_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(subr-arity (symbol-function 'car))"#);
    assert_ok_eq("(1 . 1)", &o, &n);
}

#[test]
fn oracle_subr_arity_plus_variadic_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(subr-arity (symbol-function '+))"#);
    assert_ok_eq("(0 . many)", &o, &n);
}

#[test]
fn oracle_special_form_p_if_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(special-form-p 'if)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_special_form_p_car_is_nil_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(special-form-p 'car)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_functionp_subr_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(functionp (symbol-function 'car))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_macrop_on_macro_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(
        r#"(progn (defmacro nvm--fi-macro (x) (list '1+ x)) (macrop (symbol-function 'nvm--fi-macro)))"#,
    );
    assert_ok_eq("t", &o, &n);
}
