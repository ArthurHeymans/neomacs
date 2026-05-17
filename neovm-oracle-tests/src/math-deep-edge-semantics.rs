//! Oracle parity for math function deep edge cases.
//! GNU src/data.c, src/floatfns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- remainder (%) vs mod ---

#[test]
fn oracle_remainder_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(% 10 3)"#);
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_remainder_negative_dividend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(% -10 3)"#);
    assert_ok_eq("-1", &o, &n);
}

#[test]
fn oracle_mod_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mod 10 3)"#);
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_mod_negative_dividend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // mod is always non-negative (mathematical modulo)
    let (o, n) = eval_oracle_and_neovm(r#"(mod -10 3)"#);
    assert_ok_eq("2", &o, &n);
}

// --- rounding operations with negative floats ---

#[test]
fn oracle_floor_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(floor -3.5)"#);
    assert_ok_eq("-4", &o, &n);
}

#[test]
fn oracle_ceiling_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(ceiling -3.5)"#);
    assert_ok_eq("-3", &o, &n);
}

#[test]
fn oracle_truncate_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(truncate -3.5)"#);
    assert_ok_eq("-3", &o, &n);
}

#[test]
fn oracle_round_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(round -3.5)"#);
    assert_ok_eq("-4", &o, &n);
}

#[test]
fn oracle_round_positive_half() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(round 3.5)"#);
    assert_ok_eq("4", &o, &n);
}

// --- expt ---

#[test]
fn oracle_expt_positive_exponent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(expt 2 10)"#);
    assert_ok_eq("1024", &o, &n);
}

#[test]
fn oracle_expt_negative_exponent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(expt 2 -1)"#);
    assert_ok_eq("0.5", &o, &n);
}

#[test]
fn oracle_expt_zero_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(expt 0 5)"#);
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_expt_zero_exponent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(expt 5 0)"#);
    assert_ok_eq("1", &o, &n);
}
