//! Oracle parity for copy-syntax-table + char-or-string-p + char-table-p.
//! GNU src/syntax.c, src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_copy_syntax_table_is_syntax_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(syntax-table-p (copy-syntax-table))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_copy_syntax_table_copy_is_independent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (let ((orig (syntax-table)) (cpy (copy-syntax-table))) (not (eq orig cpy))))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_or_string_p_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-or-string-p ?a)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_or_string_p_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-or-string-p "hello")"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_table_p_on_char_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-table-p (make-char-table 'syntax-table))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_char_table_p_on_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(char-table-p [1 2 3])"#);
    assert_ok_eq("nil", &o, &n);
}
