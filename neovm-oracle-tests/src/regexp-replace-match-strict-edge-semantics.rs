//! Oracle parity for regex replace + match-data operations.
//! GNU src/search.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_regexp_quote_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(regexp-quote "hello.world")"#);
    assert_ok_eq("\"hello\\\\.world\"", &o, &n);
}

#[test]
fn oracle_regexp_quote_special_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(stringp (regexp-quote "a*b+c?d[e]f"))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_match_beginning_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (string-match "cd" "abcdef") (match-beginning 0))"#);
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_match_end_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(progn (string-match "cd" "abcdef") (match-end 0))"#);
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_match_data_has_markers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (string-match "foo" "foobar") (consp (match-data)))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_set_match_data_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (string-match "xyz" "abcxyzdef") (let ((saved (match-data))) (set-match-data saved) (equal saved (match-data))))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_replace_match_simple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*rm2*")) (erase-buffer) (insert "hello world") (goto-char 1) (search-forward "world" nil t) (replace-match "earth" t t) (buffer-string))"#,
    );
    assert_ok_eq("\"hello earth\"", &o, &n);
}

#[test]
fn oracle_looking_at_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*la*")) (erase-buffer) (insert "hello") (goto-char 1) (looking-at "hel"))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_looking_at_no_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*la2*")) (erase-buffer) (insert "hello") (goto-char 1) (looking-at "xyz"))"#,
    );
    assert_ok_eq("nil", &o, &n);
}
