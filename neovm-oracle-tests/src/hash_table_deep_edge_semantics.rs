//! Oracle parity for hash table deep edge cases.
//! GNU src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- make-hash-table variants ---

#[test]
fn oracle_make_hash_table_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(hash-table-p (make-hash-table :test 'eq))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_make_hash_table_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(hash-table-p (make-hash-table :test 'equal))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_make_hash_table_eql() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(hash-table-p (make-hash-table :test 'eql))"#);
    assert_ok_eq("t", &o, &n);
}

// --- gethash ---

#[test]
fn oracle_gethash_found() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq ht (make-hash-table :test 'eq)) (puthash 'key 'val ht) (gethash 'key ht))"#,
    );
    assert_ok_eq("val", &o, &n);
}

#[test]
fn oracle_gethash_default_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq ht (make-hash-table :test 'eq)) (gethash 'nokey ht 'default))"#,
    );
    assert_ok_eq("default", &o, &n);
}

#[test]
fn oracle_gethash_missing_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq ht (make-hash-table :test 'eq)) (gethash 'nokey ht))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

// --- puthash overwrite ---

#[test]
fn oracle_puthash_overwrites() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq ht (make-hash-table :test 'eq)) (puthash 'a 1 ht) (puthash 'a 2 ht) (gethash 'a ht))"#,
    );
    assert_ok_eq("2", &o, &n);
}

// --- remhash ---

#[test]
fn oracle_remhash_removes_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq ht (make-hash-table :test 'eq)) (puthash 'a 1 ht) (remhash 'a ht) (gethash 'a ht))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

// --- clrhash ---

#[test]
fn oracle_clrhash_empties_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq ht (make-hash-table :test 'eq)) (puthash 'a 1 ht) (puthash 'b 2 ht) (clrhash ht) (hash-table-count ht))"#,
    );
    assert_ok_eq("0", &o, &n);
}

// --- hash-table-count ---

#[test]
fn oracle_hash_table_count_after_inserts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq ht (make-hash-table :test 'eq)) (puthash 'a 1 ht) (puthash 'b 2 ht) (hash-table-count ht))"#,
    );
    assert_ok_eq("2", &o, &n);
}

// --- copy-hash-table independence ---

#[test]
fn oracle_copy_hash_table_independent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Copy should be independent of original
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq ht (make-hash-table :test 'eq)) (puthash 'a 1 ht) (puthash 'b 2 ht) (setq cp (copy-hash-table ht)) (puthash 'a 99 ht) (list (gethash 'a cp) (gethash 'a ht)))"#,
    );
    assert_ok_eq("(1 99)", &o, &n);
}
