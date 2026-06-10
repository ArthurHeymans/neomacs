//! Oracle parity for delete, delq, member, memq, assoc, assq deep interaction.
//! GNU src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_delete_removes_matching() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // delete returns the list with matching elements removed
    let (o, n) = eval_oracle_and_neovm(r#"(delete "a" (list "a" "b" "a"))"#);
    assert_ok_eq("(\"b\")", &o, &n);
}

#[test]
fn oracle_delq_removes_by_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // delq is destructive: use the returned value
    let (o, n) = eval_oracle_and_neovm(r#"(delq 1 (list 1 2 1 3))"#);
    assert_ok_eq("(2 3)", &o, &n);
}

#[test]
fn oracle_member_returns_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(member 'c '(a b c d e))"#);
    assert_ok_eq("(c d e)", &o, &n);
}

#[test]
fn oracle_member_not_found() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(member 'z '(a b c))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_memq_uses_eq_not_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(memq (make-string 2 ?a) (list (make-string 2 ?a)))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_assoc_finds_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(assoc 'a '((a . 1) (b . 2) (a . 3)))"#);
    assert_ok_eq("(a . 1)", &o, &n);
}

#[test]
fn oracle_assq_nil_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(assq 'c '((a . 1) (b . 2)))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_rassoc_finds_by_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(rassoc 2 '((a . 1) (b . 2) (c . 3)))"#);
    assert_ok_eq("(b . 2)", &o, &n);
}

#[test]
fn oracle_assoc_on_non_cons_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // assoc skips non-cons elements (a), matches cons (b . 2)
    let (o, n) = eval_oracle_and_neovm(r#"(assoc 'b '(a (b . 2) (c . 3)))"#);
    assert_ok_eq("(b . 2)", &o, &n);
}

#[test]
fn oracle_member_dotted_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(member 'c '(a b c . d))"#);
    assert_ok_eq("(c . d)", &o, &n);
}
