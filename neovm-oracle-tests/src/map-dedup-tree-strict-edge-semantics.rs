//! Oracle parity for mapconcat + delete-dups + copy-alist + copy-tree.
//! GNU src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm, eval_oracle_and_neovm_with_bootstrap};

#[test]
fn oracle_mapconcat_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mapconcat 'identity '("a" "b" "c") ",")"#);
    assert_ok_eq("\"a,b,c\"", &o, &n);
}

#[test]
fn oracle_mapconcat_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mapconcat 'identity nil ",")"#);
    assert_ok_eq("\"\"", &o, &n);
}

#[test]
fn oracle_mapconcat_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mapconcat 'identity '("x") "-")"#);
    assert_ok_eq("\"x\"", &o, &n);
}

#[test]
fn oracle_delete_dups_removes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_with_bootstrap(r#"(delete-dups '(a b a c b d))"#);
    assert_ok_eq("(a b c d)", &o, &n);
}

#[test]
fn oracle_delete_dups_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_with_bootstrap(r#"(delete-dups nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_copy_alist_is_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(equal '((a . 1) (b . 2)) (copy-alist '((a . 1) (b . 2))))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_copy_sequence_list_is_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(equal '(a b c) (copy-sequence '(a b c)))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_maphash_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (defvar neovm--test-mh-count 0) (let ((h (make-hash-table))) (puthash 'a 1 h) (puthash 'b 2 h) (maphash (lambda (_k _v) (setq neovm--test-mh-count (1+ neovm--test-mh-count))) h)) neovm--test-mh-count)"#,
    );
    assert_ok_eq("2", &o, &n);
}
