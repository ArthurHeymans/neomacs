//! Oracle parity for sort + compare-strings strict edges.
//! GNU src/fns.c, src/sort.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_sort_nil_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sort nil '<)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_sort_vector_returns_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(vectorp (sort [3 1 2] '<))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_sort_length_preserved() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(= (length (sort '(3 1 4 2) '<)) 4)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_compare_strings_equal_returns_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(compare-strings "abc" nil nil "abc" nil nil)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_compare_strings_less_returns_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(< (compare-strings "abc" nil nil "abd" nil nil) 0)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_compare_strings_case_insensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eq (compare-strings "ABC" nil nil "abc" nil nil) t)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_sort_called_with_custom_pred() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sort '(3 1 4 2) (lambda (a b) (< a b)))"#);
    assert_ok_eq("(1 2 3 4)", &o, &n);
}
