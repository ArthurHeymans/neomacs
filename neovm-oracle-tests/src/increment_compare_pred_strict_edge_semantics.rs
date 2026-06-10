//! Oracle parity for 1+, 1-, <, >, <=, >=, /=, floatp, listp edges.
//! GNU src/data.c, src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_one_plus_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(1+ 41)");
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_one_minus_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(1- 43)");
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_one_plus_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(1+ -1)");
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_lt_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(< 1 2 3)");
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_lt_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(< 1 1)");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_gt_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(> 3 2 1)");
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_le_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(<= 1 1 2)");
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_ge_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(>= 3 3 2)");
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_not_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(/= 1 2)");
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_not_equal_same() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm("(/= 1 1)");
    assert_ok_eq("nil", &o, &n);
}
