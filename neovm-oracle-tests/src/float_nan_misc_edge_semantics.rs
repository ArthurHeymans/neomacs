//! Oracle parity for float, isnan, and misc edge cases.
//! GNU src/floatfns.c, src/data.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- float ---

#[test]
fn oracle_float_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(float 42)"#);
    assert_ok_eq("42.0", &o, &n);
}

#[test]
fn oracle_float_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(float 0)"#);
    assert_ok_eq("0.0", &o, &n);
}

#[test]
fn oracle_float_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(float -1)"#);
    assert_ok_eq("-1.0", &o, &n);
}

// --- isnan ---

#[test]
fn oracle_isnan_regular_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(isnan 0.0)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_isnan_on_nan() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(isnan (/ 0.0 0.0))"#);
    assert_ok_eq("t", &o, &n);
}

// --- floatp ---

#[test]
fn oracle_floatp_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(floatp 3.14)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_floatp_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(floatp 42)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- integerp ---

#[test]
fn oracle_integerp_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(integerp 3.14)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- numberp ---

#[test]
fn oracle_numberp_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(numberp 3.14)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_numberp_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(numberp 42)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_numberp_string_is_not_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(numberp "42")"#);
    assert_ok_eq("nil", &o, &n);
}
