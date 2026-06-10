//! Oracle parity tests for number conversion: `float`, `truncate`,
//! `floor`, `ceiling`, `round`, `abs` — strict edge cases.
//!
//! GNU src/floatfns.c: numeric type conversion and rounding.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_float_from_int() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(float 42)");
    assert_ok_eq("42.0", &o, &n);
}

#[test]
fn oracle_float_from_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(float -7)");
    assert_ok_eq("-7.0", &o, &n);
}

#[test]
fn oracle_truncate_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(truncate 3.7)");
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_truncate_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(truncate -3.7)");
    assert_ok_eq("-3", &o, &n);
}

#[test]
fn oracle_floor_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(floor 3.7)");
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_floor_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(floor -3.7)");
    assert_ok_eq("-4", &o, &n);
}

#[test]
fn oracle_ceiling_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(ceiling 3.2)");
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_ceiling_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(ceiling -3.2)");
    assert_ok_eq("-3", &o, &n);
}

#[test]
fn oracle_round_up() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(round 3.6)");
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_abs_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(abs -42)");
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_float_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(float 'sym)");
    assert_err_kind(&o, &n, "wrong-type-argument");
}
