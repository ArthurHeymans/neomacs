//! Oracle parity tests for print/eval: `prin1-to-string`, `eval`,
//! `identity`, `number-to-string`, `string-to-number`.
//!
//! GNU src/print.c, src/eval.c, src/editfns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prin1_to_string_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(prin1-to-string 42)"#);
    assert_ok_eq("\"42\"", &o, &n);
}

#[test]
fn oracle_prin1_to_string_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(prin1-to-string "hello")"#);
    assert_ok_eq("\"\\\"hello\\\"\"", &o, &n);
}

#[test]
fn oracle_prin1_to_string_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(prin1-to-string 'hello)"#);
    assert_ok_eq("\"hello\"", &o, &n);
}

#[test]
fn oracle_eval_self_evaluating() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eval 42)"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_eval_quoted_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eval '(+ 1 2))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_identity_returns_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(list (identity 42) (identity nil) (identity 'sym))"#);
    assert_ok_eq("(42 nil sym)", &o, &n);
}

#[test]
fn oracle_number_to_string_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(number-to-string 42)"#);
    assert_ok_eq("\"42\"", &o, &n);
}

#[test]
fn oracle_number_to_string_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(number-to-string -99)"#);
    assert_ok_eq("\"-99\"", &o, &n);
}

#[test]
fn oracle_string_to_number_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number (number-to-string 42))"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_number_to_string_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(number-to-string 'sym)"#);
    assert_err_kind(&o, &n, "wrong-type-argument");
}
