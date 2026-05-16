//! Oracle parity tests for `assoc`, `assq`, `rassoc`, `member`, `memq` —
//! strict edge cases with non-cons elements and dotted lists.
//!
//! GNU src/fns.c: These functions iterate a list, applying `equal` or `eq`
//! to the appropriate part. Non-cons elements cause `wrong-type-argument`.
//! Dotted lists are treated as proper up to the non-nil cdr.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_assoc_finds_by_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(assoc 'b '((a . 1) (b . 2) (c . 3)))"#);
    assert_ok_eq("(b . 2)", &o, &n);
}

#[test]
fn oracle_assoc_returns_nil_for_missing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(assoc 'x '((a . 1) (b . 2)))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_assq_uses_eq_not_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // "key" as string — assq uses eq, string identity fails
    let (o, n) = eval_oracle_and_neovm(r#"(assq "key" '(("key" . 1)))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_assoc_uses_equal_for_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(assoc "key" '(("key" . 1)))"#);
    assert_ok_eq("(\"key\" . 1)", &o, &n);
}

#[test]
fn oracle_assoc_non_cons_element_skipped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU: assoc silently skips non-cons elements (not an error).
    let (o, n) = eval_oracle_and_neovm(r#"(assoc 'a '(a (b . 2)))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_rassoc_finds_by_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(rassoc 2 '((a . 1) (b . 2) (c . 3)))"#);
    assert_ok_eq("(b . 2)", &o, &n);
}

#[test]
fn oracle_member_finds_by_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(member 'b '(a b c))"#);
    assert_ok_eq("(b c)", &o, &n);
}

#[test]
fn oracle_member_nil_not_found() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(member 'x '(a b c))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_memq_uses_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // memq uses eq, so distinct string objects won't match
    let (o, n) = eval_oracle_and_neovm(r#"(memq "hello" '("hello"))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_member_dotted_list_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // dotted list: member treats cdr as the next element until non-cons
    let (o, n) = eval_oracle_and_neovm(r#"(member 'c '(a b c . d))"#);
    assert_ok_eq("(c . d)", &o, &n);
}

#[test]
fn oracle_assoc_empty_alist_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(assoc 'a nil)"#);
    assert_ok_eq("nil", &o, &n);
}
