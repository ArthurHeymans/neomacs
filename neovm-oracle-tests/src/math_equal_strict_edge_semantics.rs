//! Oracle parity for % modulo + = equality edges.
//! GNU src/data.c, src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_modulo_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(% 10 3)");
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_modulo_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(% 5 5)");
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_numeric_equal_different_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(= 1 1.0)");
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_numeric_equal_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(= 1 1 1)");
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_numeric_equal_different() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(= 1 2)");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_abs_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(abs -42)");
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_max_of_three() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(max 3 7 2)");
    assert_ok_eq("7", &o, &n);
}

#[test]
fn oracle_min_of_three() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(min 3 7 2)");
    assert_ok_eq("2", &o, &n);
}
