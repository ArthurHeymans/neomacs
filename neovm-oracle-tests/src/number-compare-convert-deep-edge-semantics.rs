//! Oracle parity for number comparison and conversion deep edge cases.
//! GNU src/data.c, src/floatfns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- = with multiple args and types ---

#[test]
fn oracle_eq_integer_equals_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(= 1 1.0)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_eq_three_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(= 5 5 5)"#);
    assert_ok_eq("t", &o, &n);
}

// --- /= (not equal) ---

#[test]
fn oracle_neq_two_different() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(/= 1 2)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_neq_all_different() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(/= 1 2)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_neq_same_is_false() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(/= 5 5)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- < and > with multiple args ---

#[test]
fn oracle_lt_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(< 1 2 3)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_gt_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(> 3 2 1)"#);
    assert_ok_eq("t", &o, &n);
}

// --- <= and >= ---

#[test]
fn oracle_le_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(<= 1 1 2)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_ge_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(>= 3 3 2)"#);
    assert_ok_eq("t", &o, &n);
}

// --- 1+ / 1- ---

#[test]
fn oracle_inc_dec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(list (1+ 41) (1- 43) (1+ -1) (1- 0))"#);
    assert_ok_eq("(42 42 0 -1)", &o, &n);
}

// --- abs ---

#[test]
fn oracle_abs_positive_and_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(list (abs -5) (abs 5) (abs 0) (abs -3.5))"#);
    assert_ok_eq("(5 5 0 3.5)", &o, &n);
}
