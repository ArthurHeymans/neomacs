//! Oracle parity tests for type predicates: `arrayp`, `vectorp`,
//! `char-table-p`, `bool-vector-p`, `keywordp`.
//!
//! GNU src/data.c: type predicates distinguish Lisp object types.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_arrayp_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(arrayp [1 2 3])"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_arrayp_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(arrayp "hello")"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_arrayp_list_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(arrayp '(a b c))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_vectorp_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(vectorp [1 2])"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_vectorp_string_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(vectorp "hello")"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_char_table_p_on_char_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-table-p (make-char-table 'syntax-table))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_table_p_on_vector_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-table-p [1 2 3])"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_keywordp_on_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(keywordp :test)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_keywordp_on_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(keywordp 'test)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_sequencep_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sequencep '(a b))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_sequencep_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sequencep 42)"#);
    assert_ok_eq("nil", &o, &n);
}
