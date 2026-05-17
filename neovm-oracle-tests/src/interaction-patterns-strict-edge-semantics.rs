//! Oracle parity for subr interaction patterns and deep edge cases.
//! Tests non-trivial combinations to surface subtle divergences.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- sort interaction patterns ---

#[test]
fn oracle_sort_ascending() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sort '(3 1 4 1 5) '<)"#);
    assert_ok_eq("(1 1 3 4 5)", &o, &n);
}

#[test]
fn oracle_sort_descending() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sort '(3 1 4 1 5) '>)"#);
    assert_ok_eq("(5 4 3 1 1)", &o, &n);
}

#[test]
fn oracle_sort_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(sort '("banana" "apple" "cherry") 'string<)"#);
    assert_ok_eq("(\"apple\" \"banana\" \"cherry\")", &o, &n);
}

// --- delq / delete with shared structure ---

#[test]
fn oracle_delq_destructive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (setq a (list 1 2 3)) (setq b a) (delq 2 a) (list a b))"#);
    // Both a and b should be modified to (1 3)
    assert_ok_eq("((1 3) (1 3))", &o, &n);
}

#[test]
fn oracle_delete_with_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // delete uses equal, delq uses eq
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (setq dl (list "ab" "cd" "ab")) (delete "ab" dl))"#);
    // delete with equal removes all matching strings
    assert_ok_eq("(\"cd\")", &o, &n);
}

// --- copy-sequence ---

#[test]
fn oracle_copy_sequence_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq orig (cons 1 (cons 2 nil))) (setq cp (copy-sequence orig)) (setcar orig 99) (car cp))"#,
    );
    // Copy should be independent
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_copy_sequence_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq v [1 2 3]) (setq cp (copy-sequence v)) (aset v 0 99) (aref cp 0))"#,
    );
    assert_ok_eq("1", &o, &n);
}

// --- nthcdr edge cases ---

#[test]
fn oracle_nthcdr_past_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nthcdr 10 '(a b c))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_nthcdr_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nthcdr 0 '(a b c))"#);
    assert_ok_eq("(a b c)", &o, &n);
}

#[test]
fn oracle_nthcdr_exact_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nthcdr 3 '(a b c))"#);
    assert_ok_eq("nil", &o, &n);
}

// --- replace-match interaction ---

#[test]
fn oracle_replace_match_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (set-buffer (get-buffer-create "*rm1*")) (erase-buffer) (insert "hello world") (goto-char 1) (re-search-forward "[a-z]+" nil t) (replace-match "X") (buffer-string))"#,
    );
    assert_ok_eq("\"X world\"", &o, &n);
}
