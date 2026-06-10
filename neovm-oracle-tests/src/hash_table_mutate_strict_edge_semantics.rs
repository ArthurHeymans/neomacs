//! Oracle parity tests for hash-table mutation operations.
//!
//! GNU src/fns.c: `remhash`, `clrhash`, `maphash`, `puthash` overwrite,
//! and `hash-table-count` edge cases.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_remhash_present_key_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(let ((h (make-hash-table))) (puthash 'k 1 h) (remhash 'k h))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_remhash_absent_key_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(let ((h (make-hash-table))) (remhash 'no h))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_remhash_updates_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(let ((h (make-hash-table))) (puthash 'a 1 h) (puthash 'b 2 h) (remhash 'a h) (hash-table-count h))"#,
    );
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_clrhash_empties_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(let ((h (make-hash-table))) (puthash 'a 1 h) (puthash 'b 2 h) (clrhash h) (hash-table-count h))"#,
    );
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_clrhash_on_empty_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(let ((h (make-hash-table))) (clrhash h) (hash-table-count h))"#);
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_puthash_overwrite() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(let ((h (make-hash-table))) (puthash 'k 1 h) (puthash 'k 2 h) (gethash 'k h))"#,
    );
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_hash_table_count_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(hash-table-count (make-hash-table))"#);
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_hash_table_rehashing_preserves_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Fill table beyond its initial size to trigger internal rehash.
    let (o, n) = eval_oracle_and_neovm(
        r#"(let ((h (make-hash-table :size 2))
           (i 0))
      (while (< i 10)
        (puthash i (* i 10) h)
        (setq i (1+ i)))
      (gethash 5 h))"#,
    );
    assert_ok_eq("50", &o, &n);
}

#[test]
fn oracle_maphash_applies_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defvar neovm--test-mh-sum 0)
  (let ((h (make-hash-table)))
    (puthash 'a 1 h)
    (puthash 'b 2 h)
    (puthash 'c 3 h)
    (maphash (lambda (_k v) (setq neovm--test-mh-sum (+ neovm--test-mh-sum v))) h))
  neovm--test-mh-sum)"#,
    );
    assert_ok_eq("6", &o, &n);
}

#[test]
fn oracle_hash_table_count_increments() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(let ((h (make-hash-table)))
         (puthash 'a 1 h)
         (puthash 'b 2 h)
         (puthash 'c 3 h)
         (hash-table-count h))"#,
    );
    assert_ok_eq("3", &o, &n);
}
