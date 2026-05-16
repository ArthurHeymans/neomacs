//! Oracle parity tests for number predicates: `natnump`, `integerp`,
//! `floatp`, `numberp` — strict edge cases.
//!
//! GNU src/data.c: type predicates have subtle behavior around
//! bignums, float/integer boundaries, and non-numeric inputs.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_natnump_positive_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(natnump 42)");
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_natnump_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(natnump 0)");
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_natnump_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(natnump -1)");
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_natnump_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(natnump 3.14)");
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_floatp_integer_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(floatp 42)");
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_floatp_float_returns_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(floatp 3.14)");
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_integerp_float_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(integerp 3.14)");
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_integerp_large_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(integerp 999999999999999)");
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_numberp_on_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(numberp 'sym)");
    assert_ok_eq("nil", &oracle, &neovm);
}
