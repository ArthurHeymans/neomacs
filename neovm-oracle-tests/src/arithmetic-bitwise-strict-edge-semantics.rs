//! Oracle parity tests for bitwise arithmetic: `logand`, `logior`,
//! `logxor`, `lognot`, `ash` — strict edge cases.
//!
//! GNU src/data.c: bitwise operations on integers.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_logand_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(logand 7 3)");
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_logand_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(logand 42 0)");
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_logior_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(logior 1 2 4)");
    assert_ok_eq("7", &o, &n);
}

#[test]
fn oracle_logxor_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(logxor 7 3)");
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_logxor_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(logxor)");
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_lognot_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(lognot 0)");
    assert_ok_eq("-1", &o, &n);
}

#[test]
fn oracle_lognot_negative_one() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(lognot -1)");
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_ash_left() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(ash 1 3)");
    assert_ok_eq("8", &o, &n);
}

#[test]
fn oracle_ash_right() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(ash 8 -2)");
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_ash_zero_shift() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(ash 42 0)");
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_mod_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(% 10 3)");
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_mod_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(% -10 3)");
    assert_ok_eq("-1", &o, &n);
}
