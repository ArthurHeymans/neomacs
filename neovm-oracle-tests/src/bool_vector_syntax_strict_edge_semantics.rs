//! Oracle parity tests for bool-vector, syntax-table, and fillarray.
//!
//! GNU src/alloc.c, src/syntax.c, src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_bool_vector_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(= (length (bool-vector)) 0)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_bool_vector_with_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(= (length (bool-vector t nil t)) 3)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_bool_vector_aref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(aref (bool-vector t nil t) 0)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_bool_vector_aref_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(aref (bool-vector t nil t) 1)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_standard_syntax_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(syntax-table-p (standard-syntax-table))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_syntax_table_p_on_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(syntax-table-p [1 2 3])"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_set_syntax_table_returns_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(syntax-table-p (set-syntax-table (standard-syntax-table)))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_fillarray_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(fillarray [1 2 3] 99)"#);
    assert_ok_eq("[99 99 99]", &o, &n);
}

#[test]
fn oracle_fillarray_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(fillarray "abc" ?x)"#);
    assert_ok_eq("\"xxx\"", &o, &n);
}

#[test]
fn oracle_modify_syntax_entry_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(progn (modify-syntax-entry ?a "w") (char-syntax ?a))"#);
    assert_ok_eq("119", &o, &n);
}
