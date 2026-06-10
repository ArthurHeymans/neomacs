//! Oracle parity tests for `delete`, `delq` — strict edges.
//!
//! GNU src/fns.c: `delete` and `delq` remove elements by equality,
//! operating on sequences (lists and vectors).

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_delete_by_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(delete 'b '(a b c b d))"#);
    assert_ok_eq("(a c d)", &o, &n);
}

#[test]
fn oracle_delete_sequence_on_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(delete 2 [1 2 3 2 4])"#);
    assert_ok_eq("[1 3 4]", &o, &n);
}

#[test]
fn oracle_delete_first_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // delete removes all matches by default
    let (o, n) = eval_oracle_and_neovm(r#"(delete 'x '(x x x))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_delq_removes_all_eq_matches() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(delq 'a '(a b a c a d))"#);
    assert_ok_eq("(b c d)", &o, &n);
}

#[test]
fn oracle_delq_nil_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(delq 'a nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_delete_string_from_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // delete uses equal, so string matching works
    let (o, n) = eval_oracle_and_neovm(r#"(delete "hi" '("hi" "there" "hi"))"#);
    assert_ok_eq("(\"there\")", &o, &n);
}

#[test]
fn oracle_delete_not_found_returns_original() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(delete 99 '(1 2 3))"#);
    assert_ok_eq("(1 2 3)", &o, &n);
}

#[test]
fn oracle_delq_vs_delete_eq_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // delq uses eq, so string identity matters; delete uses equal
    let (o, n) = eval_oracle_and_neovm(
        r#"(list
   (delq "s" '("s"))
   (delete "s" '("s")))"#,
    );
    assert_ok_eq("((\"s\") nil)", &o, &n);
}
