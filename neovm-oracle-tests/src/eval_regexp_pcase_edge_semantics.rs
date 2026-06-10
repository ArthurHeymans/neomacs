//! Oracle parity for eval, regexp-quote, match-string, and pcase.
//! GNU src/eval.c, src/search.c, lisp/emacs-lisp/pcase.el.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- eval (in-process) ---

#[test]
fn oracle_eval_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eval '(+ 1 2))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_eval_quoted_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eval '(list 1 2 3))"#);
    assert_ok_eq("(1 2 3)", &o, &n);
}

#[test]
fn oracle_eval_self_evaluating() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eval 42)"#);
    assert_ok_eq("42", &o, &n);
}

// --- regexp-quote (via binary, needs full library) ---

#[test]
fn oracle_regexp_quote_escapes_special_chars_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(regexp-quote "a.b*c[d]e^f$g")"#);
    // prin1 of regexp-quoted string: each backslash is printed as \\
    assert_ok_eq("\"a\\\\.b\\\\*c\\\\[d]e\\\\^f\\\\$g\"", &o, &n);
}

#[test]
fn oracle_regexp_quote_no_special_chars_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(regexp-quote "hello")"#);
    assert_ok_eq("\"hello\"", &o, &n);
}

// --- match-string (via binary) ---

#[test]
fn oracle_match_string_after_string_match_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (string-match "[a-z]+" "hello world") (match-string 0 "hello world"))"#,
    );
    assert_ok_eq("\"hello\"", &o, &n);
}

// --- pcase (via binary, needs full library) ---

#[test]
fn oracle_pcase_literal_match_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(pcase 42 (1 'one) (42 'forty-two) (_ 'other))"#);
    assert_ok_eq("forty-two", &o, &n);
}

#[test]
fn oracle_pcase_wildcard_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(pcase 99 (1 'one) (_ 'other))"#);
    assert_ok_eq("other", &o, &n);
}
